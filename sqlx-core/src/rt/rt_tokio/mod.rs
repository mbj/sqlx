mod socket;

pub use socket::TokioAsyncSocket;

pub fn available() -> bool {
    tokio::runtime::Handle::try_current().is_ok()
}
