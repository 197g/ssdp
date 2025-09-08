//! Messaging primitives for discovering devices and services.

use std::io;
use std::net::IpAddr;

use crate::net::connector::UdpConnector;
use crate::net::{IpVersionMode, NetifAddr};

pub mod listen;
pub mod multicast;
mod notify;
mod search;
mod ssdp;

use netdev::get_interfaces;

pub use crate::message::listen::Listen;
pub use crate::message::multicast::Multicast;
pub use crate::message::notify::{NotifyListener, NotifyMessage};
pub use crate::message::search::{SearchListener, SearchRequest, SearchResponse};

/// Multicast Socket Information
pub const UPNP_MULTICAST_IPV4_ADDR: &'static str = "239.255.255.250";
pub const UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR: &'static str = "FF05::C";
pub const UPNP_MULTICAST_PORT: u16 = 1900;

/// Default TTL For Multicast
pub const UPNP_MULTICAST_TTL: u32 = 2;

/// Enumerates different types of SSDP messages.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum MessageType {
    /// A notify message.
    Notify,
    /// A search message.
    Search,
    /// A response to a search message.
    Response,
}

#[derive(Clone)]
pub struct Config {
    pub ipv4_addr: String,
    pub ipv6_addr: String,
    pub port: u16,
    pub ttl: u32,
    pub mode: IpVersionMode,
}

impl Config {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_ipv4_addr<S: Into<String>>(mut self, value: S) -> Self {
        self.ipv4_addr = value.into();
        self
    }

    pub fn set_ipv6_addr<S: Into<String>>(mut self, value: S) -> Self {
        self.ipv6_addr = value.into();
        self
    }

    pub fn set_port(mut self, value: u16) -> Self {
        self.port = value;
        self
    }

    pub fn set_ttl(mut self, value: u32) -> Self {
        self.ttl = value;
        self
    }

    pub fn set_mode(mut self, value: IpVersionMode) -> Self {
        self.mode = value;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ipv4_addr: UPNP_MULTICAST_IPV4_ADDR.to_string(),
            ipv6_addr: UPNP_MULTICAST_IPV6_LINK_LOCAL_ADDR.to_string(),
            port: UPNP_MULTICAST_PORT,
            ttl: UPNP_MULTICAST_TTL,
            mode: IpVersionMode::Any,
        }
    }
}

/// Generate `UdpConnector` objects for all local `IPv4` interfaces.
fn all_local_connectors(multicast_ttl: Option<u32>, filter: &IpVersionMode) -> io::Result<Vec<UdpConnector>> {
    trace!("Fetching all local connectors");
    map_local(|iface| match (filter, iface.sock) {
        (&IpVersionMode::V4Only, IpAddr::V4(n)) | (&IpVersionMode::Any, IpAddr::V4(n)) => {
            Ok(Some(UdpConnector::new((n, 0), iface.index, multicast_ttl)?))
        }
        (&IpVersionMode::V6Only, IpAddr::V6(n)) | (&IpVersionMode::Any, IpAddr::V6(n)) => {
            // Skip addresses we can not bind to..
            Ok(Some(UdpConnector::new((n, 0), iface.index, multicast_ttl)?))
        }
        _ => Ok(None),
    })
}

/// Invoke the closure for every local address found on the system
///
/// This method filters out _loopback_ and _global_ addresses.
fn map_local<F, R>(mut f: F) -> io::Result<Vec<R>>
where
    F: FnMut(&NetifAddr) -> io::Result<Option<R>>,
{
    let addrs_iter = get_local_addrs()?;

    let mut obj_list = Vec::with_capacity(addrs_iter.len());

    for addr in addrs_iter {
        trace!("Found {} @ {}", addr.sock, addr.index);
        match addr.sock {
            IpAddr::V4(n) if !n.is_loopback() => {
                if let Some(x) = f(&addr)? {
                    obj_list.push(x);
                }
            }
            // Filter all loopback and global IPv6 addresses
            IpAddr::V6(n) if !n.is_loopback() && is_not_global_v6(n) => {
                if let Some(x) = f(&addr)? {
                    obj_list.push(x);
                }
            }
            _ => (),
        }
    }

    Ok(obj_list)
}

/// Determine if an address is not global.
///
/// This may return incorrectly return `false` for some addresses that are not actually global. We
/// error on the side of caution by under-approximating the set.
fn is_not_global_v6(addr: std::net::Ipv6Addr) -> bool {
    // As by [RFC3056], everything in `2002::/16`
    fn is_6to4(addr: std::net::Ipv6Addr) -> bool {
        addr.segments()[0] == 0x2002
    }

    addr.is_unspecified()
        || addr.is_loopback()
        // The most important case
        || addr.is_unique_local()
        // Second most relevant case, at least by my judgement.
        || is_6to4(addr)

    // There are two more cases (unstable features) that are less relevant. We only want interfaces
    // which are probably useful to the user (they can provide a specific configuration if they
    // whish).
    // || addr.is_benchmarking()
    // || addr.is_documentation()
    //
    // Do not try to bind to link-local address.
    // || addr.is_unicast_link_local()
}

/// Generate a list of some object R constructed from all local `Ipv4Addr` objects.
///
/// If any of the `SocketAddr`'s fail to resolve, this function will not return an error.
fn get_local_addrs() -> io::Result<Vec<NetifAddr>> {
    let iface_iter = get_interfaces().into_iter();
    Ok(iface_iter
        // NOTE: this is incomplete. With IPv6 all link-local addresses need to be annotated with
        // the network interface, i.e. scope identifier, to which the belong. The scope id can be
        // parsed by std's `IPv6Addr as FromStr` but is just a literal integer `u32`. *Usually*
        // that can be set as the interface index however this is subject to the platform
        // implementation and need not generally be the identity mapping.
        .flat_map(|iface| {
            let ipv4 = iface.ipv4.into_iter().map(|ip| IpAddr::from(ip.addr()));
            let ipv6 = iface.ipv6.into_iter().map(|ip| IpAddr::from(ip.addr()));
            let index = iface.index;

            ipv4.chain(ipv6).map(move |ip| NetifAddr { sock: ip, index })
        })
        .collect())
}
