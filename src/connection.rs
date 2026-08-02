use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::sync::Arc;

use crate::HashMap;

use crate::statement_cache::StatementCache;
use crate::error::Error;
use crate::ustr::UStr;
use crate::io::StatementId;
use crate::message::{
    BackendMessageFormat, Close, Query, ReadyForQuery, ReceivedMessage, Terminate,
    TransactionStatus,
};
use crate::statement::StatementMetadata;
use crate::codec::Oid;
use crate::{ConnectOptions, TypeInfo};

pub use self::stream::Stream;

mod establish;
mod executor;
mod resolve;
mod sasl;
mod stream;
mod tls;

/// A connection to a PostgreSQL database.
///
/// See [`ConnectOptions`] for connection URL reference.
pub struct Connection {
    pub(crate) inner: Box<ConnectionInner>,
}

pub struct ConnectionInner {
    // underlying TCP or UDS stream,
    // wrapped in a potentially TLS stream,
    // wrapped in a buffered stream
    pub(crate) stream: Stream,

    // process id of this backend
    // used to send cancel requests
    #[allow(dead_code)]
    process_id: u32,

    // secret key of this backend
    // used to send cancel requests
    #[allow(dead_code)]
    secret_key: u32,

    // sequence of statement IDs for use in preparing statements
    // in PostgreSQL, the statement is prepared to a user-supplied identifier
    next_statement_id: StatementId,

    // cache statement by query string to the id and columns
    cache_statement: StatementCache<(StatementId, Arc<StatementMetadata>)>,

    // cache user-defined types by id <-> info
    cache_type_info: HashMap<Oid, TypeInfo>,
    cache_type_oid: HashMap<UStr, Oid>,
    cache_elem_type_to_array: HashMap<Oid, Oid>,
    cache_table_data: HashMap<Oid, TableData>,

    // number of ReadyForQuery messages that we are currently expecting
    pub(crate) pending_ready_for_query_count: usize,

    // current transaction status
    transaction_status: TransactionStatus,
}

pub(crate) struct TableData {
    table_name: Arc<str>,
    /// Attribute number -> name.
    columns: BTreeMap<i16, Arc<str>>,
}

impl Connection {
    /// the version number of the server in `libpq` format
    pub fn server_version_num(&self) -> Option<u32> {
        self.inner.stream.server_version_num
    }

    /// The backend's transaction status as reported by the last completed query cycle.
    ///
    /// Refreshed from every `ReadyForQuery` frame the driver processes, including the one
    /// received during the initial handshake. Between query cycles, this reflects the current
    /// server-side state — useful for detecting `TransactionStatus::Error` (a failed transaction
    /// that must be rolled back before further queries succeed).
    pub fn transaction_status(&self) -> TransactionStatus {
        self.inner.transaction_status
    }

    // will return when the connection is ready for another query
    pub(crate) async fn wait_until_ready(&mut self) -> Result<(), Error> {
        if !self.inner.stream.write_buffer_mut().is_empty() {
            self.inner.stream.flush().await?;
        }

        while self.inner.pending_ready_for_query_count > 0 {
            let message = self.inner.stream.recv().await?;

            if let BackendMessageFormat::ReadyForQuery = message.format {
                self.handle_ready_for_query(message)?;
            }
        }

        Ok(())
    }

    async fn recv_ready_for_query(&mut self) -> Result<(), Error> {
        let r: ReadyForQuery = self.inner.stream.recv_expect().await?;

        self.inner.pending_ready_for_query_count -= 1;
        self.inner.transaction_status = r.transaction_status;

        Ok(())
    }

    #[inline(always)]
    fn handle_ready_for_query(&mut self, message: ReceivedMessage) -> Result<(), Error> {
        self.inner.pending_ready_for_query_count = self
            .inner
            .pending_ready_for_query_count
            .checked_sub(1)
            .ok_or_else(|| crate::err_protocol!("received more ReadyForQuery messages than expected"))?;

        self.inner.transaction_status = message.decode::<ReadyForQuery>()?.transaction_status;

        Ok(())
    }

    /// Queue a simple query (not prepared) to execute the next time this connection is used.
    ///
    /// Used for rolling back transactions and releasing advisory locks.
    #[inline(always)]
    pub(crate) fn queue_simple_query(&mut self, query: &str) -> Result<(), Error> {
        self.inner.stream.write_msg(Query(query))?;
        self.inner.pending_ready_for_query_count += 1;

        Ok(())
    }

}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection").finish()
    }
}

impl Connection {
    /// Establish a new database connection from a URL.
    #[inline]
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let options: ConnectOptions = url.parse()?;
        Self::connect_with(&options).await
    }

    /// Establish a new database connection with the provided options.
    pub async fn connect_with(options: &ConnectOptions) -> Result<Self, Error> {
        Connection::establish(options).await
    }

    /// Explicitly close this database connection.
    ///
    /// This notifies the database server that the connection is closing so that it can
    /// free up any server-side resources in use.
    pub async fn close(mut self) -> Result<(), Error> {
        // The normal, graceful termination procedure is that the frontend sends a Terminate
        // message and immediately closes the connection.
        //
        // On receipt of this message, the backend closes the connection and terminates.
        self.inner.stream.send(Terminate).await?;
        self.inner.stream.shutdown().await?;

        Ok(())
    }

    /// Immediately close the connection without sending a graceful shutdown.
    ///
    /// This should still at least send a TCP `FIN` frame to let the server know we're dying.
    #[doc(hidden)]
    pub async fn close_hard(mut self) -> Result<(), Error> {
        self.inner.stream.shutdown().await?;

        Ok(())
    }

    /// Checks if a connection to the database is still valid.
    pub async fn ping(&mut self) -> Result<(), Error> {
        // The simplest call-and-response that's possible.
        self.write_sync();
        self.wait_until_ready().await
    }

    /// The number of statements currently cached in the connection.
    pub fn cached_statements_size(&self) -> usize {
        self.inner.cache_statement.len()
    }

    /// Removes all statements from the cache, closing them on the server if
    /// needed.
    pub async fn clear_cached_statements(&mut self) -> Result<(), Error> {
        self.inner.cache_type_oid.clear();

        let mut cleared = 0_usize;

        self.wait_until_ready().await?;

        while let Some((id, _)) = self.inner.cache_statement.remove_lru() {
            self.inner.stream.write_msg(Close::Statement(id))?;
            cleared += 1;
        }

        if cleared > 0 {
            self.write_sync();
            self.inner.stream.flush().await?;

            self.wait_for_close_complete(cleared).await?;
            self.recv_ready_for_query().await?;
        }

        Ok(())
    }

    /// Restore any buffers in the connection to their default capacity, if possible.
    pub fn shrink_buffers(&mut self) {
        self.inner.stream.shrink_buffers();
    }

    #[doc(hidden)]
    pub fn flush(&mut self) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        self.wait_until_ready()
    }

    #[doc(hidden)]
    pub fn should_flush(&self) -> bool {
        !self.inner.stream.write_buffer().is_empty()
    }
}

