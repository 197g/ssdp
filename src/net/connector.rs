use std::io;
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;

use net2::UdpSocketExt as _;

use crate::net::NetworkConnector;

use crate::net;
use crate::net::sender::UdpSender;

/// A `UdpConnector` allows Hyper to obtain `NetworkStream` objects over `UdpSockets`
/// so that Http messages created by Hyper can be sent over UDP instead of TCP.
pub struct UdpConnector(Arc<UdpSocket>);

impl UdpConnector {
    /// Create a new UdpConnector that will be bound to the given local address.
    pub fn new(
        local_addr: impl ToSocketAddrs,
        index: u32,
        _multicast_ttl: Option<u32>,
    ) -> io::Result<UdpConnector> {
        let addr = net::addr_from_trait(local_addr)?;
        debug!("Attempting to bind to {}", addr);

        let udp = UdpSocket::bind(addr)?;

        // The bind address indicates where to receive messages. This is independent from where to
        // send messages (<https://stackoverflow.com/a/26988214>) which is a problem in particular
        // for IPv6 that shares the same multicast addresses across links, i.e. the common ones are
        // not bound from the network prefix.
        match addr {
            SocketAddr::V4(v4) => {
                udp.set_multicast_if_v4(v4.ip())?
            },
            SocketAddr::V6(_) => {
                udp.set_multicast_if_v6(index)?;
            }
        };

        // TODO: This throws an invalid argument error
        // if let Some(n) = multicast_ttl {
        //     trace!("Setting ttl to {}", n);
        //     try!(udp.set_multicast_ttl_v4(n));
        // }

        Ok(UdpConnector(Arc::new(udp)))
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Destroy the UdpConnector and return the underlying UdpSocket.
    pub fn deconstruct(self) -> Arc<UdpSocket> {
        self.0
    }
}

impl NetworkConnector for UdpConnector {
    type Stream = UdpSender;

    fn connect(&self, host: &str, port: u16) -> io::Result<Self::Stream> {
        let udp_sock = Arc::clone(&self.0);
        udp_sock.set_broadcast(true)?;
        let local = self.local_addr()?;

        trace!("Connecting through {local}");
        let sock_addr = match local {
            SocketAddr::V4(_) => SocketAddr::V4(SocketAddrV4::new(
                FromStr::from_str(host).map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?,
                port,
            )),
            SocketAddr::V6(n) => {
                let mut addr: SocketAddrV6 =
                    if host.find('[') == Some(0) && host.rfind(']') == Some(host.len() - 1) {
                        FromStr::from_str(format!("{}:{}", host, port).as_str())
                            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?
                    } else {
                        FromStr::from_str(format!("[{}]:{}", host, port).as_str())
                            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?
                    };
                addr.set_flowinfo(n.flowinfo());
                addr.set_scope_id(n.scope_id());
                SocketAddr::V6(addr)
            }
        };

        Ok(UdpSender::new(udp_sock, sock_addr))
    }
}
