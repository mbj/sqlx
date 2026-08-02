use crate::error::Error;
use crate::executor::Execute;
use crate::io::{PortalId, StatementId};
use crate::message::{
    self, BackendMessageFormat, Bind, Close, CommandComplete, DataRow, ParameterDescription, Parse,
    ParseComplete, RowDescription,
};
use crate::statement::StatementMetadata;
use crate::{
    statement::Statement, Arguments, Connection, QueryResult, Row, TypeInfo,
    ValueFormat,
};
use futures_core::future::BoxFuture;
use futures_core::stream::BoxStream;
use futures_core::Stream;
use futures_util::{FutureExt, StreamExt, TryFutureExt, TryStreamExt};
use crate::sql_str::SqlStr;
use crate::Either;
use std::future;
use std::{pin::pin, sync::Arc};

async fn prepare(
    conn: &mut Connection,
    sql: &str,
    arg_types: &[TypeInfo],
    metadata: Option<Arc<StatementMetadata>>,
    persistent: bool,
    resolve_column_origin: bool,
) -> Result<(StatementId, Arc<StatementMetadata>), Error> {
    let id = if persistent {
        let id = conn.inner.next_statement_id;
        conn.inner.next_statement_id = id.next();
        id
    } else {
        StatementId::UNNAMED
    };

    // build a list of type OIDs to send to the database in the PARSE command
    // we have not yet started the query sequence, so we are *safe* to cleanly make
    // additional queries here to get any missing OIDs
    let param_types = conn.resolve_types(arg_types).await?;

    // flush and wait until we are re-ready
    conn.wait_until_ready().await?;

    // next we send the PARSE command to the server
    conn.inner.stream.write_msg(Parse {
        param_types: &param_types,
        query: sql,
        statement: id,
    })?;

    if metadata.is_none() {
        // get the statement columns and parameters
        conn.inner
            .stream
            .write_msg(message::Describe::Statement(id))?;
    }

    // we ask for the server to immediately send us the result of the PARSE command
    conn.write_sync();
    conn.inner.stream.flush().await?;

    // indicates that the SQL query string is now successfully parsed and has semantic validity
    conn.inner.stream.recv_expect::<ParseComplete>().await?;

    let metadata = if let Some(metadata) = metadata {
        // each SYNC produces one READY FOR QUERY
        conn.recv_ready_for_query().await?;

        // we already have metadata
        metadata
    } else {
        let parameters = recv_desc_params(conn).await?;

        let row_desc = recv_desc_rows(conn).await?;

        // each SYNC produces one READY FOR QUERY
        conn.recv_ready_for_query().await?;

        let metadata = conn
            .resolve_statement_metadata::<true>(Some(parameters), row_desc, resolve_column_origin)
            .await?;

        // ensure that if we did fetch custom data, we wait until we are fully ready before
        // continuing
        conn.wait_until_ready().await?;

        metadata
    };

    Ok((id, metadata))
}

async fn recv_desc_params(conn: &mut Connection) -> Result<ParameterDescription, Error> {
    conn.inner.stream.recv_expect().await
}

async fn recv_desc_rows(conn: &mut Connection) -> Result<Option<RowDescription>, Error> {
    let rows: Option<RowDescription> = match conn.inner.stream.recv().await? {
        // describes the rows that will be returned when the statement is eventually executed
        message if message.format == BackendMessageFormat::RowDescription => {
            Some(message.decode()?)
        }

        // no data would be returned if this statement was executed
        message if message.format == BackendMessageFormat::NoData => None,

        message => {
            return Err(crate::err_protocol!(
                "expecting RowDescription or NoData but received {:?}",
                message.format
            ));
        }
    };

    Ok(rows)
}

impl Connection {
    // wait for CloseComplete to indicate a statement was closed
    pub(super) async fn wait_for_close_complete(&mut self, mut count: usize) -> Result<(), Error> {
        // we need to wait for the [CloseComplete] to be returned from the server
        while count > 0 {
            match self.inner.stream.recv().await? {
                message if message.format == BackendMessageFormat::PortalSuspended => {
                    // there was an open portal
                    // this can happen if the last time a statement was used it was not fully executed
                }

                message if message.format == BackendMessageFormat::CloseComplete => {
                    // successfully closed the statement (and freed up the server resources)
                    count -= 1;
                }

                message => {
                    return Err(crate::err_protocol!(
                        "expecting PortalSuspended or CloseComplete but received {:?}",
                        message.format
                    ));
                }
            }
        }

        Ok(())
    }

    #[inline(always)]
    pub(crate) fn write_sync(&mut self) {
        self.inner
            .stream
            .write_msg(message::Sync)
            .expect("BUG: Sync should not be too big for protocol");

        // all SYNC messages will return a ReadyForQuery
        self.inner.pending_ready_for_query_count += 1;
    }

    async fn get_or_prepare(
        &mut self,
        sql: &str,
        parameters: &[TypeInfo],
        persistent: bool,
        // optional metadata that was provided by the user, this means they are reusing
        // a statement object
        metadata: Option<Arc<StatementMetadata>>,
        resolve_column_origin: bool,
    ) -> Result<(StatementId, Arc<StatementMetadata>), Error> {
        if let Some(statement) = self.inner.cache_statement.get_mut(sql) {
            return Ok((*statement).clone());
        }

        let statement = prepare(
            self,
            sql,
            parameters,
            metadata,
            persistent,
            resolve_column_origin,
        )
        .await?;

        if persistent && self.inner.cache_statement.is_enabled() {
            if let Some((id, _)) = self.inner.cache_statement.insert(sql, statement.clone()) {
                self.inner.stream.write_msg(Close::Statement(id))?;
                self.write_sync();

                self.inner.stream.flush().await?;

                self.wait_for_close_complete(1).await?;
                self.recv_ready_for_query().await?;
            }
        }

        Ok(statement)
    }

    pub(crate) async fn run<'e, 'c: 'e, 'q: 'e>(
        &'c mut self,
        query: SqlStr,
        arguments: Option<Arguments>,
        persistent: bool,
        metadata_opt: Option<Arc<StatementMetadata>>,
    ) -> Result<impl Stream<Item = Result<Either<QueryResult, Row>, Error>> + 'e, Error> {
        let sql = query.as_str();

        // before we continue, wait until we are "ready" to accept more queries
        self.wait_until_ready().await?;

        let mut metadata: Arc<StatementMetadata>;

        let format = if let Some(mut arguments) = arguments {
            // Check this before we write anything to the stream.
            //
            // Note: Postgres actually interprets this value as unsigned,
            // making the max number of parameters 65535, not 32767
            // https://github.com/launchbadge/sqlx/issues/3464
            // https://www.postgresql.org/docs/current/limits.html
            let num_params = u16::try_from(arguments.len()).map_err(|_| {
                crate::err_protocol!(
                    "Connection::run(): too many arguments for query: {}",
                    arguments.len()
                )
            })?;

            // prepare the statement if this our first time executing it
            // always return the statement ID here
            let (statement, metadata_) = self
                .get_or_prepare(sql, &arguments.types, persistent, metadata_opt, false)
                .await?;

            metadata = metadata_;

            // patch holes created during encoding
            arguments.apply_patches(self, &metadata.parameters).await?;

            // consume messages till `ReadyForQuery` before bind and execute
            self.wait_until_ready().await?;

            // bind to attach the arguments to the statement and create a portal
            self.inner.stream.write_msg(Bind {
                portal: PortalId::UNNAMED,
                statement,
                formats: &[ValueFormat::Binary],
                num_params,
                params: &arguments.buffer,
                result_formats: &[ValueFormat::Binary],
            })?;

            // executes the portal up to the passed limit
            // the protocol-level limit acts nearly identically to the `LIMIT` in SQL
            self.inner.stream.write_msg(message::Execute {
                portal: PortalId::UNNAMED,
                // Non-zero limits cause query plan pessimization by disabling parallel workers:
                // https://github.com/launchbadge/sqlx/issues/3673
                limit: 0,
            })?;
            // From https://www.postgresql.org/docs/current/protocol-flow.html:
            //
            // "An unnamed portal is destroyed at the end of the transaction, or as
            // soon as the next Bind statement specifying the unnamed portal as
            // destination is issued. (Note that a simple Query message also
            // destroys the unnamed portal."

            // we ask the database server to close the unnamed portal and free the associated resources
            // earlier - after the execution of the current query.
            self.inner
                .stream
                .write_msg(Close::Portal(PortalId::UNNAMED))?;

            // finally, [Sync] asks postgres to process the messages that we sent and respond with
            // a [ReadyForQuery] message when it's completely done. Theoretically, we could send
            // dozens of queries before a [Sync] and postgres can handle that. Execution on the server
            // is still serial but it would reduce round-trips. Some kind of builder pattern that is
            // termed batching might suit this.
            self.write_sync();

            // prepared statements are binary
            ValueFormat::Binary
        } else {
            // Query will trigger a ReadyForQuery
            self.queue_simple_query(sql)?;

            // metadata starts out as "nothing"
            metadata = Arc::new(StatementMetadata::default());

            // and unprepared statements are text
            ValueFormat::Text
        };

        self.inner.stream.flush().await?;

        Ok(crate::try_stream! {
            loop {
                let message = self.inner.stream.recv().await?;

                match message.format {
                    BackendMessageFormat::BindComplete
                    | BackendMessageFormat::ParseComplete
                    | BackendMessageFormat::ParameterDescription
                    | BackendMessageFormat::NoData
                    // unnamed portal has been closed
                    | BackendMessageFormat::CloseComplete
                    => {
                        // harmless messages to ignore
                    }

                    // "Execute phase is always terminated by the appearance of
                    // exactly one of these messages: CommandComplete,
                    // EmptyQueryResponse (if the portal was created from an
                    // empty query string), ErrorResponse, or PortalSuspended"
                    BackendMessageFormat::CommandComplete => {
                        // a SQL command completed normally
                        let cc: CommandComplete = message.decode()?;

                        let rows_affected = cc.rows_affected();
                        r#yield!(Either::Left(QueryResult {
                            rows_affected,
                        }));
                    }

                    BackendMessageFormat::EmptyQueryResponse => {
                        // empty query string passed to an unprepared execute
                    }

                    // Message::ErrorResponse is handled in self.stream.recv()

                    // incomplete query execution has finished
                    BackendMessageFormat::PortalSuspended => {}

                    // indicates that a *new* set of rows are about to be returned
                    BackendMessageFormat::RowDescription => {
                        let new_metadata = self.resolve_statement_metadata::<false>(
                            None,
                            Some(message.decode()?),
                            false,
                        ).await?;

                        metadata = new_metadata;
                    }

                    BackendMessageFormat::DataRow => {
                        // one of the set of rows returned by a SELECT, FETCH, etc query
                        let data: DataRow = message.decode()?;
                        let row = Row {
                            data,
                            format,
                            metadata: Arc::clone(&metadata),
                        };

                        r#yield!(Either::Right(row));
                    }

                    BackendMessageFormat::ReadyForQuery => {
                        // processing of the query string is complete
                        self.handle_ready_for_query(message)?;
                        break;
                    }

                    _ => {
                        return Err(crate::err_protocol!(
                            "execute: unexpected message: {:?}",
                            message.format
                        ));
                    }
                }
            }

            Ok(())
        })
    }
}

impl Connection {
    /// Execute the query and return the total number of rows affected.
    #[inline]
    pub fn execute<'e, 'q: 'e, E>(
        &'e mut self,
        query: E,
    ) -> BoxFuture<'e, Result<QueryResult, Error>>
    where
        E: 'q + Execute<'q>,
    {
        self.execute_many(query).try_collect().boxed()
    }

    /// Execute multiple queries and return the rows affected from each query, in a stream.
    #[inline]
    pub fn execute_many<'e, 'q: 'e, E>(
        &'e mut self,
        query: E,
    ) -> BoxStream<'e, Result<QueryResult, Error>>
    where
        E: 'q + Execute<'q>,
    {
        self.fetch_many(query)
            .try_filter_map(|step| async move {
                Ok(match step {
                    Either::Left(rows) => Some(rows),
                    Either::Right(_) => None,
                })
            })
            .boxed()
    }

    /// Execute the query and return the generated results as a stream.
    #[inline]
    pub fn fetch<'e, 'q: 'e, E>(
        &'e mut self,
        query: E,
    ) -> BoxStream<'e, Result<Row, Error>>
    where
        E: 'q + Execute<'q>,
    {
        self.fetch_many(query)
            .try_filter_map(|step| async move {
                Ok(match step {
                    Either::Left(_) => None,
                    Either::Right(row) => Some(row),
                })
            })
            .boxed()
    }

    /// Execute the query and return all the generated results, collected into a [`Vec`].
    #[inline]
    pub fn fetch_all<'e, 'q: 'e, E>(
        &'e mut self,
        query: E,
    ) -> BoxFuture<'e, Result<Vec<Row>, Error>>
    where
        E: 'q + Execute<'q>,
    {
        self.fetch(query).try_collect().boxed()
    }

    /// Execute the query and return exactly one row.
    #[inline]
    pub fn fetch_one<'e, 'q: 'e, E>(
        &'e mut self,
        query: E,
    ) -> BoxFuture<'e, Result<Row, Error>>
    where
        E: 'q + Execute<'q>,
    {
        self.fetch_optional(query)
            .and_then(|row| {
                future::ready(match row {
                    Some(row) => Ok(row),
                    None => Err(Error::RowNotFound),
                })
            })
            .boxed()
    }

    /// Execute multiple queries and return the generated results as a stream
    /// from each query, in a stream.
    pub fn fetch_many<'e, 'q, E>(
        &'e mut self,
        mut query: E,
    ) -> BoxStream<'e, Result<Either<QueryResult, Row>, Error>>
    where
        E: Execute<'q>,
        'q: 'e,
        E: 'q,
    {
        // False positive: https://github.com/rust-lang/rust-clippy/issues/12560
        #[allow(clippy::map_clone)]
        let metadata = query.statement().map(|s| Arc::clone(&s.metadata));
        let arguments = query.take_arguments().map_err(Error::Encode);
        let persistent = query.persistent();
        let sql = query.sql();

        Box::pin(crate::try_stream! {
            let arguments = arguments?;
            let mut s = pin!(self.run(sql, arguments, persistent, metadata).await?);

            while let Some(v) = s.try_next().await? {
                r#yield!(v);
            }

            Ok(())
        })
    }

    /// Execute the query and return at most one row.
    pub fn fetch_optional<'e, 'q, E>(
        &'e mut self,
        mut query: E,
    ) -> BoxFuture<'e, Result<Option<Row>, Error>>
    where
        E: Execute<'q>,
        'q: 'e,
        E: 'q,
    {
        // False positive: https://github.com/rust-lang/rust-clippy/issues/12560
        #[allow(clippy::map_clone)]
        let metadata = query.statement().map(|s| Arc::clone(&s.metadata));
        let arguments = query.take_arguments().map_err(Error::Encode);
        let persistent = query.persistent();

        Box::pin(async move {
            let sql = query.sql();
            let arguments = arguments?;
            let mut s = pin!(self.run(sql, arguments, persistent, metadata).await?);

            // With deferred constraints we need to check all responses as we
            // could get a OK response (with uncommitted data), only to get an
            // error response after (when the deferred constraint is actually
            // checked).
            let mut ret = None;
            while let Some(result) = s.try_next().await? {
                match result {
                    Either::Right(r) if ret.is_none() => ret = Some(r),
                    _ => {}
                }
            }
            Ok(ret)
        })
    }

    /// Prepare the SQL query to inspect the type information of its parameters and results.
    #[inline]
    pub fn prepare<'e>(
        &'e mut self,
        query: SqlStr,
    ) -> BoxFuture<'e, Result<Statement, Error>> {
        self.prepare_with(query, &[])
    }

    /// Prepare the SQL query, with parameter type information, to inspect the
    /// type information about its parameters and results.
    pub fn prepare_with<'e>(
        &'e mut self,
        sql: SqlStr,
        parameters: &'e [TypeInfo],
    ) -> BoxFuture<'e, Result<Statement, Error>> {
        Box::pin(async move {
            self.wait_until_ready().await?;

            let (_, metadata) = self
                .get_or_prepare(sql.as_str(), parameters, true, None, true)
                .await?;

            Ok(Statement { sql, metadata })
        })
    }
}
