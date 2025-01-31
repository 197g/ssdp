use crate::net::{self, NetworkStream};
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, UdpSocket};

/// A type that wraps a `UdpSocket` and a `SocketAddr` and implements the `NetworkStream`
/// trait.
///
/// Note that reading from this stream will generate an error, this object is
/// used for intercepting Http messages from Hyper and sending them out via Udp.
/// The response(s) from client(s) are to be handled by some other object that
/// has a cloned handle to our internal `UdpSocket` handle.
pub struct UdpSender {
    udp: UdpSocket,
    dst: SocketAddr,
    buf: net::packet::PacketBuffer,
}

impl UdpSender {
    /// Creates a new UdpSender object.
    pub fn new(udp: UdpSocket, dst: SocketAddr) -> UdpSender {
        UdpSender {
            udp,
            dst,
            buf: Default::default(),
        }
    }
}

impl NetworkStream for UdpSender {
    fn peer_addr(&mut self) -> io::Result<SocketAddr> {
        Ok(self.dst)
    }
}

impl Read for UdpSender {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        // Simulate Some Network Error So Our Process Doesnt Hang
        Err(io::Error::new(ErrorKind::ConnectionAborted, "UdpSender Can Not Be Read From"))
    }
}

impl Write for UdpSender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let data = self.buf.as_slice();
        let result = self.udp.send_to(data, self.dst);

        debug!("Sent HTTP Request:\n{}", String::from_utf8_lossy(data));
        self.buf.clear();

        result.map(|_| ())
    }
}

impl Clone for UdpSender {
    fn clone(&self) -> UdpSender {
        let udp_clone = self.udp.try_clone().unwrap();

        UdpSender {
            udp: udp_clone,
            dst: self.dst,
            buf: self.buf.clone(),
        }
    }

    fn clone_from(&mut self, source: &UdpSender) {
        let udp_clone = source.udp.try_clone().unwrap();

        self.udp = udp_clone;
        self.dst = source.dst;
    }
}
