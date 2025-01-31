//! Format http into an HTTPU packet.
use std::io::Write as _;

use crate::error::SSDPError;
use crate::net::packet::PacketBuffer;

use headers::HeaderMap;

pub struct Request<'map> {
    pub method: &'map str,
    pub headers: &'map HeaderMap,
    body: Option<&'map [u8]>,
}

pub struct Response<'map> {
    pub headers: &'map HeaderMap,
    pub body: Option<&'map [u8]>,
}

impl<'map> Request<'map> {
    pub fn new(headers: &'map HeaderMap) -> Self {
        Request {
            method: "NOTIFY",
            headers,
            body: None,
        }
    }

    pub fn serialize(&self, packet: &mut PacketBuffer) -> Result<(), SSDPError> {
        packet.buffer.truncate(0);
        writeln!(packet, "{} * HTTP/1.1", self.method)?;
        writeln!(packet, "HOST: 239.255.255.250:1900")?;
        for (name, value) in self.headers {
            write!(packet, "{}: ", name.as_str().to_uppercase())?;
            packet.write_all(value.as_bytes())?;
            writeln!(packet, "")?;
        }
        writeln!(packet, "")?;
        Ok(())
    }
}
