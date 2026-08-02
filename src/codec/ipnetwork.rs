#[doc(no_inline)]
pub use ::ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};

mod ipaddr;

// Parent module is named after the `ipnetwork` crate, this is named after the `IpNetwork` type.
#[allow(clippy::module_inception)]
mod ipnetwork;
