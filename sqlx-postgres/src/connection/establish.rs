use crate::HashMap;

use crate::common::StatementCache;
use crate::connection::{sasl, stream::PgStream};
use crate::error::Error;
use crate::io::StatementId;
use crate::message::{
    Authentication, BackendKeyData, BackendMessageFormat, Password, ReadyForQuery, Startup,
};
use crate::{PgConnectOptions, PgConnection, PgSessionOptions};

use super::PgConnectionInner;

// https://www.postgresql.org/docs/current/protocol-flow.html#id-1.10.5.7.3
// https://www.postgresql.org/docs/current/protocol-flow.html#id-1.10.5.7.11

impl PgConnection {
    pub(crate) async fn establish(options: &PgConnectOptions) -> Result<Self, Error> {
        // Upgrade to TLS if we were asked to and the server supports it
        let stream = PgStream::connect(options).await?;

        Self::establish_with_stream(stream, &options.session).await
    }

    /// Establish a PostgreSQL connection over a pre-existing socket.
    ///
    /// The provided socket must already be connected and, if applicable, TLS-upgraded.
    /// SQLx will perform the PostgreSQL startup handshake and authentication on it.
    ///
    /// This is runtime-agnostic. The caller must provide a type implementing sqlx's
    /// [`Socket`] trait. For a tokio-specific convenience wrapper, see
    /// [`establish_tokio_socket`](Self::establish_tokio_socket).
    pub async fn establish_with_socket(
        socket: Box<dyn crate::net::Socket>,
        options: &PgSessionOptions,
    ) -> Result<Self, Error> {
        let stream = PgStream::from_socket(socket);
        Self::establish_with_stream(stream, options).await
    }

    /// Establish a PostgreSQL connection over a pre-existing tokio async stream.
    ///
    /// The provided stream must already be connected and, if applicable, TLS-upgraded.
    /// SQLx will perform the PostgreSQL startup handshake and authentication on it.
    ///
    /// This is a convenience wrapper around [`establish_with_socket`](Self::establish_with_socket)
    /// for tokio `AsyncRead + AsyncWrite` types (e.g. `tokio_rustls::TlsStream<TcpStream>`).
    #[cfg(feature = "_rt-tokio")]
    pub async fn establish_tokio_socket<S>(
        socket: S,
        options: &PgSessionOptions,
    ) -> Result<Self, Error>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync + Unpin + 'static,
    {
        let socket = crate::rt::rt_tokio::TokioAsyncSocket::new(socket);
        Self::establish_with_socket(Box::new(socket), options).await
    }

    async fn establish_with_stream(
        mut stream: PgStream,
        session: &PgSessionOptions,
    ) -> Result<Self, Error> {
        // To begin a session, a frontend opens a connection to the server
        // and sends a startup message.

        let mut params = vec![
            // Sets the display format for date and time values,
            // as well as the rules for interpreting ambiguous date input values.
            ("DateStyle", "ISO, MDY"),
            // Sets the client-side encoding (character set).
            // <https://www.postgresql.org/docs/devel/multibyte.html#MULTIBYTE-CHARSET-SUPPORTED>
            ("client_encoding", "UTF8"),
            // Sets the time zone for displaying and interpreting time stamps.
            ("TimeZone", "UTC"),
        ];

        if let Some(ref extra_float_digits) = session.extra_float_digits {
            params.push(("extra_float_digits", extra_float_digits));
        }

        if let Some(ref application_name) = session.application_name {
            params.push(("application_name", application_name));
        }

        if let Some(ref options) = session.options {
            params.push(("options", options));
        }

        stream.write(Startup {
            username: Some(&session.username),
            database: session.database.as_deref(),
            params: &params,
        })?;

        stream.flush().await?;

        // The server then uses this information and the contents of
        // its configuration files (such as pg_hba.conf) to determine whether the connection is
        // provisionally acceptable, and what additional
        // authentication is required (if any).

        let mut process_id = 0;
        let mut secret_key = 0;
        let transaction_status;

        loop {
            let message = stream.recv().await?;
            match message.format {
                BackendMessageFormat::Authentication => match message.decode()? {
                    Authentication::Ok => {
                        // the authentication exchange is successfully completed
                        // do nothing; no more information is required to continue
                    }

                    Authentication::CleartextPassword => {
                        // The frontend must now send a [PasswordMessage] containing the
                        // password in clear-text form.

                        stream
                            .send(Password::Cleartext(
                                session.password.as_deref().unwrap_or_default(),
                            ))
                            .await?;
                    }

                    Authentication::Md5Password(body) => {
                        // The frontend must now send a [PasswordMessage] containing the
                        // password (with user name) encrypted via MD5, then encrypted again
                        // using the 4-byte random salt specified in the
                        // [AuthenticationMD5Password] message.

                        stream
                            .send(Password::Md5 {
                                username: &session.username,
                                password: session.password.as_deref().unwrap_or_default(),
                                salt: body.salt,
                            })
                            .await?;
                    }

                    Authentication::Sasl(body) => {
                        sasl::authenticate(&mut stream, session, body).await?;
                    }

                    method => {
                        return Err(err_protocol!(
                            "unsupported authentication method: {:?}",
                            method
                        ));
                    }
                },

                BackendMessageFormat::BackendKeyData => {
                    // provides secret-key data that the frontend must save if it wants to be
                    // able to issue cancel requests later

                    let data: BackendKeyData = message.decode()?;

                    process_id = data.process_id;
                    secret_key = data.secret_key;
                }

                BackendMessageFormat::ReadyForQuery => {
                    // start-up is completed. The frontend can now issue commands
                    transaction_status = message.decode::<ReadyForQuery>()?.transaction_status;

                    break;
                }

                _ => {
                    return Err(err_protocol!(
                        "establish: unexpected message: {:?}",
                        message.format
                    ))
                }
            }
        }

        Ok(PgConnection {
            inner: Box::new(PgConnectionInner {
                stream,
                process_id,
                secret_key,
                transaction_status,
                transaction_depth: 0,
                pending_ready_for_query_count: 0,
                next_statement_id: StatementId::NAMED_START,
                cache_statement: StatementCache::new(session.statement_cache_capacity),
                cache_type_oid: HashMap::new(),
                cache_type_info: HashMap::new(),
                cache_elem_type_to_array: HashMap::new(),
                cache_table_to_column_names: HashMap::new(),
                log_settings: session.log_settings.clone(),
            }),
        })
    }
}
