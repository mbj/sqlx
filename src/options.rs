use std::borrow::Cow;
use std::fmt::{self, Display, Write};
use std::path::{Path, PathBuf};

pub use ssl_mode::SslMode;

use crate::net::tls::CertificateInput;

mod connect;
mod parse;
mod ssl_mode;

#[doc = include_str!("options/doc.md")]
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    pub(crate) host: String,
    pub(crate) host_addr: Option<String>,
    pub(crate) port: u16,
    pub(crate) socket: Option<PathBuf>,
    pub(crate) username: String,
    pub(crate) password: Option<String>,
    pub(crate) database: Option<String>,
    pub(crate) ssl_mode: SslMode,
    pub(crate) ssl_root_cert: Option<CertificateInput>,
    pub(crate) ssl_client_cert: Option<CertificateInput>,
    pub(crate) ssl_client_key: Option<CertificateInput>,
    pub(crate) statement_cache_capacity: usize,
    pub(crate) application_name: Option<String>,
    pub(crate) extra_float_digits: Option<Cow<'static, str>>,
    pub(crate) options: Option<String>,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectOptions {
    /// Create connection options with hardcoded defaults.
    ///
    /// This fork does NOT read any `PG*` environment variables, does NOT consult `.pgpass`,
    /// and does NOT inspect the filesystem for well-known Unix socket directories. The only
    /// inputs to connection configuration are this builder and parsed connection URLs.
    ///
    /// Hardcoded defaults:
    /// - `host`: `"localhost"`
    /// - `port`: `5432`
    /// - `username`: `"postgres"`
    /// - `ssl_mode`: [`SslMode::Prefer`]
    /// - `statement_cache_capacity`: `100`
    /// - `extra_float_digits`: `Some("2")`
    ///
    /// All other fields are set to `None`.
    pub fn new() -> Self {
        ConnectOptions {
            host: "localhost".to_string(),
            host_addr: None,
            port: 5432,
            socket: None,
            username: "postgres".to_string(),
            password: None,
            database: None,
            ssl_mode: SslMode::Prefer,
            ssl_root_cert: None,
            ssl_client_cert: None,
            ssl_client_key: None,
            statement_cache_capacity: 100,
            application_name: None,
            extra_float_digits: Some("2".into()),
            options: None,
        }
    }

    /// Sets the name of the host to connect to.
    ///
    /// If a host name begins with a slash, it specifies
    /// Unix-domain communication rather than TCP/IP communication; the value is the name of
    /// the directory in which the socket file is stored.
    ///
    /// The default behavior when host is not specified, or is empty,
    /// is to connect to a Unix-domain socket
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .host("localhost");
    /// ```
    pub fn host(mut self, host: &str) -> Self {
        host.clone_into(&mut self.host);
        self
    }

    /// Sets the host address to connect to.
    ///
    /// This is different to the host parameter as it overwrites DNS lookups for TCP/IP
    /// communication. This is particuarly useful when the crate::Postgres port has to be
    /// proxied to localhost for security reasons.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .host("example.com");
    ///
    /// // Initially, host_addr should be None (unless PGHOSTADDR env var is set)
    /// // For this test, we assume it's not set
    /// assert_eq!(options.get_host_addr(), None);
    ///
    /// let options = options.host_addr("127.0.0.1");
    ///
    /// // After setting, host_addr should contain the specified value
    /// assert_eq!(options.get_host_addr(), Some("127.0.0.1"));
    /// ```
    pub fn host_addr(mut self, host_addr: &str) -> Self {
        self.host_addr = Some(host_addr.to_string());
        self
    }

    /// Sets the port to connect to at the server host.
    ///
    /// The default port for PostgreSQL is `5432`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .port(5432);
    /// ```
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets a custom path to a directory containing a unix domain socket,
    /// switching the connection method from TCP to the corresponding socket.
    ///
    /// By default set to `None`.
    pub fn socket(mut self, path: impl AsRef<Path>) -> Self {
        self.socket = Some(path.as_ref().to_path_buf());
        self
    }

    /// Sets the username to connect as.
    ///
    /// Defaults to be the same as the operating system name of
    /// the user running the application.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .username("postgres");
    /// ```
    pub fn username(mut self, username: &str) -> Self {
        username.clone_into(&mut self.username);
        self
    }

    /// Sets the password to use if the server demands password authentication.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .username("root")
    ///     .password("safe-and-secure");
    /// ```
    pub fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_owned());
        self
    }

    /// Sets the database name. Defaults to be the same as the user name.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .database("postgres");
    /// ```
    pub fn database(mut self, database: &str) -> Self {
        self.database = Some(database.to_owned());
        self
    }

    /// Sets whether or with what priority a secure SSL TCP/IP connection will be negotiated
    /// with the server.
    ///
    /// By default, the SSL mode is [`Prefer`](SslMode::Prefer), and the client will
    /// first attempt an SSL connection but fallback to a non-SSL connection on failure.
    ///
    /// Ignored for Unix domain socket communication.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::{SslMode, ConnectOptions};
    /// let options = ConnectOptions::new()
    ///     .ssl_mode(SslMode::Require);
    /// ```
    pub fn ssl_mode(mut self, mode: SslMode) -> Self {
        self.ssl_mode = mode;
        self
    }

    /// Sets the name of a file containing SSL certificate authority (CA) certificate(s).
    /// If the file exists, the server's certificate will be verified to be signed by
    /// one of these authorities.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::{SslMode, ConnectOptions};
    /// let options = ConnectOptions::new()
    ///     // Providing a CA certificate with less than VerifyCa is pointless
    ///     .ssl_mode(SslMode::VerifyCa)
    ///     .ssl_root_cert("./ca-certificate.crt");
    /// ```
    pub fn ssl_root_cert(mut self, cert: impl AsRef<Path>) -> Self {
        self.ssl_root_cert = Some(CertificateInput::File(cert.as_ref().to_path_buf()));
        self
    }

    /// Sets the name of a file containing SSL client certificate.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::{SslMode, ConnectOptions};
    /// let options = ConnectOptions::new()
    ///     // Providing a CA certificate with less than VerifyCa is pointless
    ///     .ssl_mode(SslMode::VerifyCa)
    ///     .ssl_client_cert("./client.crt");
    /// ```
    pub fn ssl_client_cert(mut self, cert: impl AsRef<Path>) -> Self {
        self.ssl_client_cert = Some(CertificateInput::File(cert.as_ref().to_path_buf()));
        self
    }

    /// Sets the SSL client certificate as a PEM-encoded byte slice.
    ///
    /// This should be an ASCII-encoded blob that starts with `-----BEGIN CERTIFICATE-----`.
    ///
    /// # Example
    /// Note: embedding SSL certificates and keys in the binary is not advised.
    /// This is for illustration purposes only.
    ///
    /// ```rust
    /// # use sqlx::{SslMode, ConnectOptions};
    ///
    /// const CERT: &[u8] = b"\
    /// -----BEGIN CERTIFICATE-----
    /// <Certificate data here.>
    /// -----END CERTIFICATE-----";
    ///    
    /// let options = ConnectOptions::new()
    ///     // Providing a CA certificate with less than VerifyCa is pointless
    ///     .ssl_mode(SslMode::VerifyCa)
    ///     .ssl_client_cert_from_pem(CERT);
    /// ```
    pub fn ssl_client_cert_from_pem(mut self, cert: impl AsRef<[u8]>) -> Self {
        self.ssl_client_cert = Some(CertificateInput::Inline(cert.as_ref().to_vec()));
        self
    }

    /// Sets the name of a file containing SSL client key.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::{SslMode, ConnectOptions};
    /// let options = ConnectOptions::new()
    ///     // Providing a CA certificate with less than VerifyCa is pointless
    ///     .ssl_mode(SslMode::VerifyCa)
    ///     .ssl_client_key("./client.key");
    /// ```
    pub fn ssl_client_key(mut self, key: impl AsRef<Path>) -> Self {
        self.ssl_client_key = Some(CertificateInput::File(key.as_ref().to_path_buf()));
        self
    }

    /// Sets the SSL client key as a PEM-encoded byte slice.
    ///
    /// This should be an ASCII-encoded blob that starts with `-----BEGIN PRIVATE KEY-----`.
    ///
    /// # Example
    /// Note: embedding SSL certificates and keys in the binary is not advised.
    /// This is for illustration purposes only.
    ///
    /// ```rust
    /// # use sqlx::{SslMode, ConnectOptions};
    ///
    /// const KEY: &[u8] = b"\
    /// -----BEGIN PRIVATE KEY-----
    /// <Private key data here.>
    /// -----END PRIVATE KEY-----";
    ///
    /// let options = ConnectOptions::new()
    ///     // Providing a CA certificate with less than VerifyCa is pointless
    ///     .ssl_mode(SslMode::VerifyCa)
    ///     .ssl_client_key_from_pem(KEY);
    /// ```
    pub fn ssl_client_key_from_pem(mut self, key: impl AsRef<[u8]>) -> Self {
        self.ssl_client_key = Some(CertificateInput::Inline(key.as_ref().to_vec()));
        self
    }

    /// Sets PEM encoded trusted SSL Certificate Authorities (CA).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::{SslMode, ConnectOptions};
    /// let options = ConnectOptions::new()
    ///     // Providing a CA certificate with less than VerifyCa is pointless
    ///     .ssl_mode(SslMode::VerifyCa)
    ///     .ssl_root_cert_from_pem(vec![]);
    /// ```
    pub fn ssl_root_cert_from_pem(mut self, pem_certificate: Vec<u8>) -> Self {
        self.ssl_root_cert = Some(CertificateInput::Inline(pem_certificate));
        self
    }

    /// Sets the capacity of the connection's statement cache in a number of stored
    /// distinct statements. Caching is handled using LRU, meaning when the
    /// amount of queries hits the defined limit, the oldest statement will get
    /// dropped.
    ///
    /// The default cache capacity is 100 statements.
    pub fn statement_cache_capacity(mut self, capacity: usize) -> Self {
        self.statement_cache_capacity = capacity;
        self
    }

    /// Sets the application name. Defaults to None
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .application_name("my-app");
    /// ```
    pub fn application_name(mut self, application_name: &str) -> Self {
        self.application_name = Some(application_name.to_owned());
        self
    }

    /// Sets or removes the `extra_float_digits` connection option.
    ///
    /// This changes the default precision of floating-point values returned in text mode (when
    /// not using prepared statements such as calling methods of [`Executor`] directly).
    ///
    /// Historically, Postgres would by default round floating-point values to 6 and 15 digits
    /// for `float4`/`REAL` (`f32`) and `float8`/`DOUBLE` (`f64`), respectively, which would mean
    /// that the returned value may not be exactly the same as its representation in Postgres.
    ///
    /// The nominal range for this value is `-15` to `3`, where negative values for this option
    /// cause floating-points to be rounded to that many fewer digits than normal (`-1` causes
    /// `float4` to be rounded to 5 digits instead of six, or 14 instead of 15 for `float8`),
    /// positive values cause Postgres to emit that many extra digits of precision over default
    /// (or simply use maximum precision in Postgres 12 and later),
    /// and 0 means keep the default behavior (or the "old" behavior described above
    /// as of Postgres 12).
    ///
    /// SQLx sets this value to 3 by default, which tells Postgres to return floating-point values
    /// at their maximum precision in the hope that the parsed value will be identical to its
    /// counterpart in Postgres. This is also the default in Postgres 12 and later anyway.
    ///
    /// However, older versions of Postgres and alternative implementations that talk the Postgres
    /// protocol may not support this option, or the full range of values.
    ///
    /// If you get an error like "unknown option `extra_float_digits`" when connecting, try
    /// setting this to `None` or consult the manual of your database for the allowed range
    /// of values.
    ///
    /// For more information, see:
    /// * [Postgres manual, 20.11.2: Client Connection Defaults; Locale and Formatting][20.11.2]
    /// * [Postgres manual, 8.1.3: Numeric Types; Floating-point Types][8.1.3]
    ///
    /// [`Executor`]: crate::executor::Executor
    /// [20.11.2]: https://www.postgresql.org/docs/current/runtime-config-client.html#RUNTIME-CONFIG-CLIENT-FORMAT
    /// [8.1.3]: https://www.postgresql.org/docs/current/datatype-numeric.html#DATATYPE-FLOAT
    ///
    /// ### Examples
    /// ```rust
    /// # use sqlx::ConnectOptions;
    ///
    /// let mut options = ConnectOptions::new()
    ///     // for Redshift and Postgres 10
    ///     .extra_float_digits(2);
    ///
    /// let mut options = ConnectOptions::new()
    ///     // don't send the option at all (Postgres 9 and older)
    ///     .extra_float_digits(None);
    /// ```
    pub fn extra_float_digits(mut self, extra_float_digits: impl Into<Option<i8>>) -> Self {
        self.extra_float_digits = extra_float_digits.into().map(|it| it.to_string().into());
        self
    }

    /// Set additional startup options for the connection as a list of key-value pairs.
    ///
    /// Escapes the options’ backslash and space characters as per
    /// https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-OPTIONS
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .options([("geqo", "off"), ("statement_timeout", "5min")]);
    /// ```
    pub fn options<K, V, I>(mut self, options: I) -> Self
    where
        K: Display,
        V: Display,
        I: IntoIterator<Item = (K, V)>,
    {
        // Do this in here so `options_str` is only set if we have an option to insert
        let options_str = self.options.get_or_insert_with(String::new);
        for (k, v) in options {
            if !options_str.is_empty() {
                options_str.push(' ');
            }

            options_str.push_str("-c ");
            write!(OptionsWriteEscaped(options_str), "{k}={v}").ok();
        }
        self
    }

    /// We try using a socket if hostname starts with `/` or if socket parameter
    /// is specified.
    pub(crate) fn fetch_socket(&self) -> Option<String> {
        match self.socket {
            Some(ref socket) => {
                let full_path = format!("{}/.s.PGSQL.{}", socket.display(), self.port);
                Some(full_path)
            }
            None if self.host.starts_with('/') => {
                let full_path = format!("{}/.s.PGSQL.{}", self.host, self.port);
                Some(full_path)
            }
            _ => None,
        }
    }
}

impl ConnectOptions {
    /// Get the current host.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .host("127.0.0.1");
    /// assert_eq!(options.get_host(), "127.0.0.1");
    /// ```
    pub fn get_host(&self) -> &str {
        &self.host
    }

    /// Get the current host addr.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .host("example.com");
    ///
    /// // Initially, host_addr should be None (unless PGHOSTADDR env var is set)
    /// // For this test, we assume it's not set
    /// assert_eq!(options.get_host_addr(), None);
    ///
    /// let options = options.host_addr("127.0.0.1");
    ///
    /// // After setting host_addr, it should return the configured value
    /// assert_eq!(options.get_host_addr(), Some("127.0.0.1"));
    /// ```
    pub fn get_host_addr(&self) -> Option<&str> {
        self.host_addr.as_deref()
    }

    /// Get the server's port.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .port(6543);
    /// assert_eq!(options.get_port(), 6543);
    /// ```
    pub fn get_port(&self) -> u16 {
        self.port
    }

    /// Get the socket path.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .socket("/tmp");
    /// assert!(options.get_socket().is_some());
    /// ```
    pub fn get_socket(&self) -> Option<&PathBuf> {
        self.socket.as_ref()
    }

    /// Get the server's port.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .username("foo");
    /// assert_eq!(options.get_username(), "foo");
    /// ```
    pub fn get_username(&self) -> &str {
        &self.username
    }

    /// Get the current database name.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .database("postgres");
    /// assert!(options.get_database().is_some());
    /// ```
    pub fn get_database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    /// Get the SSL mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::{ConnectOptions, SslMode};
    /// let options = ConnectOptions::new();
    /// assert!(matches!(options.get_ssl_mode(), SslMode::Prefer));
    /// ```
    pub fn get_ssl_mode(&self) -> SslMode {
        self.ssl_mode
    }

    /// Get the application name.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .application_name("service");
    /// assert!(options.get_application_name().is_some());
    /// ```
    pub fn get_application_name(&self) -> Option<&str> {
        self.application_name.as_deref()
    }

    /// Get the options.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sqlx::ConnectOptions;
    /// let options = ConnectOptions::new()
    ///     .options([("foo", "bar")]);
    /// assert!(options.get_options().is_some());
    /// ```
    pub fn get_options(&self) -> Option<&str> {
        self.options.as_deref()
    }
}

/// Writer that escapes passed-in PostgreSQL options.
///
/// Escapes backslashes and spaces with an additional backslash according to
/// https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-OPTIONS
#[derive(Debug)]
struct OptionsWriteEscaped<'a>(&'a mut String);

impl Write for OptionsWriteEscaped<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut span_start = 0;

        for (span_end, matched) in s.match_indices([' ', '\\']) {
            write!(self.0, r"{}\{matched}", &s[span_start..span_end])?;
            span_start = span_end + matched.len();
        }

        // Write the rest of the string after the last match, or all of it if no matches
        self.0.push_str(&s[span_start..]);

        Ok(())
    }

    fn write_char(&mut self, ch: char) -> fmt::Result {
        if matches!(ch, ' ' | '\\') {
            self.0.push('\\');
        }

        self.0.push(ch);

        Ok(())
    }
}

#[test]
fn test_options_formatting() {
    let options = ConnectOptions::new().options([("geqo", "off")]);
    assert_eq!(options.options, Some("-c geqo=off".to_string()));
    let options = options.options([("search_path", "sqlx")]);
    assert_eq!(
        options.options,
        Some("-c geqo=off -c search_path=sqlx".to_string())
    );
    let options = ConnectOptions::new()
        .options([("geqo", "off"), ("statement_timeout", "5min")]);
    assert_eq!(
        options.options,
        Some("-c geqo=off -c statement_timeout=5min".to_string())
    );
    // https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-OPTIONS
    let options = ConnectOptions::new()
        .options([("application_name", r"/back\slash/ and\ spaces")]);
    assert_eq!(
        options.options,
        Some(r"-c application_name=/back\\slash/\ and\\\ spaces".to_string())
    );
    let options = ConnectOptions::new();
    assert_eq!(options.options, None);
}

#[test]
fn test_pg_write_escaped() {
    let mut buf = String::new();
    let mut x = OptionsWriteEscaped(&mut buf);
    x.write_str("x").unwrap();
    x.write_str("").unwrap();
    x.write_char('\\').unwrap();
    x.write_str("y \\").unwrap();
    x.write_char(' ').unwrap();
    x.write_char('z').unwrap();
    assert_eq!(buf, r"x\\y\ \\\ z");
}

#[test]
fn test_new_hardcoded_defaults() {
    let options = ConnectOptions::new();
    assert_eq!(options.host, "localhost");
    assert_eq!(options.port, 5432);
    assert_eq!(options.username, "postgres");
    assert_eq!(options.ssl_mode, SslMode::Prefer);
    assert_eq!(options.statement_cache_capacity, 100);
    assert!(options.extra_float_digits.is_some());
    assert!(options.password.is_none());
    assert!(options.database.is_none());

    let options = ConnectOptions::new()
        .host("example.com")
        .port(5433)
        .username("myuser")
        .database("mydb")
        .password("mypass");

    assert_eq!(options.get_host(), "example.com");
    assert_eq!(options.get_port(), 5433);
    assert_eq!(options.get_username(), "myuser");
    assert_eq!(options.get_database(), Some("mydb"));
}
