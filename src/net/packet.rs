use std::fmt;
use std::io::{self, Error, ErrorKind};
use std::net::{SocketAddr, UdpSocket};

/// Maximum length for packets received on a `PacketReceiver`.
pub const MAX_PCKT_LEN: usize = 1500;

/// A `PacketReceiver` that abstracts over a network socket and reads full packets
/// from the connection. Packets received from this connection are assumed to
/// be no larger than what the typical MTU would be on a standard router.
///
/// See `net::packet::MAX_PCKT_LEN`.
pub struct PacketReceiver(UdpSocket);

/// An owned buffer suitable for packet.
#[derive(Clone)]
pub struct PacketBuffer {
    pub(crate) buffer: Vec<u8>,
    pub(crate) mmu: usize,
}

impl Default for PacketBuffer {
    fn default() -> Self {
        PacketBuffer {
            buffer: vec![],
            mmu: MAX_PCKT_LEN,
        }
    }
}

impl PacketReceiver {
    /// Create a new PacketReceiver from the given UdpSocket.
    pub fn new(udp: UdpSocket) -> PacketReceiver {
        PacketReceiver(udp)
    }

    /// Receive a packet from the underlying connection.
    pub fn recv_pckt(&self) -> io::Result<(Vec<u8>, SocketAddr)> {
        let mut pckt_buf = vec![0u8; MAX_PCKT_LEN];

        let (size, addr) = self.0.recv_from(&mut pckt_buf)?;

        // Check For Something That SHOULD NEVER Occur.
        if size > pckt_buf.len() {
            Err(Error::new(ErrorKind::Other, "UdpSocket Reported Receive Length Greater Than Buffer"))
        } else {
            // `truncate` does not reallocate the vec's backing storage
            pckt_buf.truncate(size);

            Ok((pckt_buf, addr))
        }
    }
}

impl fmt::Display for PacketReceiver {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.local_addr() {
            Ok(addr) => write!(f, "{}", addr),
            Err(err) => write!(f, "{}", err),
        }
    }
}

impl PacketBuffer {
    pub fn as_slice(&self) -> &[u8] {
        self.buffer.as_slice()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl io::Write for PacketBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let space = self.mmu.saturating_sub(self.buffer.len());
        let take = buf.len().min(space);
        self.buffer.extend_from_slice(&buf[..take]);
        Ok(take)
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        let mut space = self.mmu.saturating_sub(self.buffer.len());
        let mut written = 0;

        for slice in bufs {
            let take = slice.len().min(space);
            self.buffer.extend_from_slice(&slice[..take]);
            let done = take == space;

            written += take;
            space -= take;

            if done {
                break;
            }
        }

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
