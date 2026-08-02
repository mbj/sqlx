use crate::error::Error;
use crate::Url;
use crate::{ConnectOptions, Connection};

impl ConnectOptions {
    /// Parse a `ConnectOptions` from a URL.
    pub fn from_url(url: &Url) -> Result<Self, Error> {
        Self::parse_from_url(url)
    }

    /// Return a connection URL that may be used to connect to the same database as `self`.
    ///
    /// ### Note: Lossy
    /// Any flags or settings without a URL representation are lost and default when re-parsed.
    pub fn to_url_lossy(&self) -> Url {
        self.build_url()
    }

    /// Establish a new database connection with the options specified by `self`.
    pub async fn connect(&self) -> Result<Connection, Error> {
        Connection::establish(self).await
    }
}
