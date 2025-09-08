use std::fmt::Debug;
use std::io;
use std::net::ToSocketAddrs;
use std::time::Duration;

use headers::{Header, HeaderMapExt as _};

use crate::error::{
    SSDPError::{InvalidMethod, MissingHeader},
    SSDPResult,
};

use crate::header::{HeaderMut, MX};
use crate::message::multicast::{self, Multicast};
use crate::message::ssdp::SSDPMessage;
use crate::message::{self, Config, Listen, MessageType};
use crate::net;
use crate::receiver::{FromRawSSDP, SSDPReceiver};

/// Overhead to add to device response times to account for transport time.
const NETWORK_TIMEOUT_OVERHEAD: u8 = 1;

/// Devices are required to respond within 1 second of receiving unicast message.
const DEFAULT_UNICAST_TIMEOUT: u8 = 1 + NETWORK_TIMEOUT_OVERHEAD;

/// Search request that can be sent via unicast or multicast to devices on the network.
#[derive(Debug, Clone)]
pub struct SearchRequest {
    message: SSDPMessage,
}

impl SearchRequest {
    /// Construct a new SearchRequest.
    pub fn new() -> SearchRequest {
        SearchRequest {
            message: SSDPMessage::new(MessageType::Search),
        }
    }

    /// Send this search request to a single host.
    ///
    /// Currently this sends the unicast message on all available network
    /// interfaces. This assumes that the network interfaces are operating
    /// on either different subnets or different ip address ranges.
    pub fn unicast<A: ToSocketAddrs>(&mut self, dst_addr: A) -> SSDPResult<SSDPReceiver<SearchResponse>> {
        let mode = net::IpVersionMode::from_addr(&dst_addr)?;
        let mut connectors = message::all_local_connectors(None, &mode)?;

        // Send On All Connectors
        for connector in &mut connectors {
            self.message.send(connector, &dst_addr)?;
        }

        let mut raw_connectors = Vec::with_capacity(connectors.len());
        raw_connectors.extend(connectors.into_iter().map(|conn| conn.deconstruct()));

        let opt_timeout = opt_unicast_timeout(self.message.headers().typed_get::<MX>());

        Ok(SSDPReceiver::new(raw_connectors, opt_timeout)?)
    }
}

impl Multicast for SearchRequest {
    type Item = SSDPReceiver<SearchResponse>;

    fn multicast_with_config(&self, config: &Config) -> SSDPResult<Self::Item> {
        let connectors = multicast::send(&self.message, config)?;
        let mcast_timeout = multicast_timeout(self.message.headers().typed_get::<MX>())?;
        trace!("Sending to {} connectors with {:?}", connectors.len(), mcast_timeout);
        let mut raw_connectors = Vec::with_capacity(connectors.len());
        raw_connectors.extend(connectors.into_iter().map(|conn| conn.deconstruct()));

        Ok(SSDPReceiver::new(raw_connectors, Some(mcast_timeout))?)
    }
}

impl Default for SearchRequest {
    fn default() -> Self {
        SearchRequest::new()
    }
}

/// Get the require timeout to use for a multicast search request.
fn multicast_timeout(mx: Option<MX>) -> SSDPResult<Duration> {
    match mx {
        Some(MX(n)) => Ok(Duration::new((n + NETWORK_TIMEOUT_OVERHEAD) as u64, 0)),
        None => Err(MissingHeader("Multicast Searches Require An MX Header")),
    }
}

/// Get the default timeout to use for a unicast search request.
fn opt_unicast_timeout(mx: Option<MX>) -> Option<Duration> {
    match mx {
        Some(MX(n)) => Some(Duration::new((n + NETWORK_TIMEOUT_OVERHEAD) as u64, 0)),
        None => Some(Duration::new(DEFAULT_UNICAST_TIMEOUT as u64, 0)),
    }
}

impl FromRawSSDP for SearchRequest {
    fn from_packet(bytes: &[u8]) -> SSDPResult<SearchRequest> {
        let message = SSDPMessage::from_packet(bytes)?;

        if message.message_type() != MessageType::Search {
            Err(InvalidMethod("SSDP Message Received Is Not A SearchRequest".into()))
        } else {
            Ok(SearchRequest { message: message })
        }
    }
}

impl HeaderMut for SearchRequest {
    fn set<H>(&mut self, value: H)
    where
        H: Header,
    {
        self.message.set(value)
    }
}

/// Search response that can be received or sent via unicast to devices on the network.
#[derive(Debug, Clone)]
pub struct SearchResponse {
    message: SSDPMessage,
}

impl SearchResponse {
    /// Construct a new SearchResponse.
    pub fn new() -> SearchResponse {
        SearchResponse {
            message: SSDPMessage::new(MessageType::Response),
        }
    }

    /// Send this search response to a single host.
    ///
    /// Currently this sends the unicast message on all available network
    /// interfaces. This assumes that the network interfaces are operating
    /// on either different subnets or different ip address ranges.
    pub fn unicast<A: ToSocketAddrs>(&mut self, dst_addr: A) -> SSDPResult<()> {
        let mode = net::IpVersionMode::from_addr(&dst_addr)?;
        let mut connectors = message::all_local_connectors(None, &mode)?;

        let mut success_count = 0;
        let mut error_count = 0;
        // Send On All Connectors
        for conn in &mut connectors {
            // Some routing errors are expected, not all interfaces can find the target addresses
            match self.message.send(conn, &dst_addr) {
                Ok(_) => success_count += 1,
                Err(_) => error_count += 1,
            }
        }

        if success_count == 0 && error_count > 0 {
            Err(io::Error::last_os_error())?;
        }

        Ok(())
    }
}

impl Default for SearchResponse {
    fn default() -> Self {
        SearchResponse::new()
    }
}

/// Search listener that can listen for search messages sent within the network.
pub struct SearchListener;

impl Listen for SearchListener {
    type Message = SearchResponse;
}

impl FromRawSSDP for SearchResponse {
    fn from_packet(bytes: &[u8]) -> SSDPResult<SearchResponse> {
        let message = SSDPMessage::from_packet(bytes)?;

        if message.message_type() != MessageType::Response {
            Err(InvalidMethod("SSDP Message Received Is Not A SearchResponse".into()))
        } else {
            Ok(SearchResponse { message: message })
        }
    }
}

impl HeaderMut for SearchResponse {
    fn set<H>(&mut self, value: H)
    where
        H: Header,
    {
        self.message.set(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::header::MX;

    #[test]
    fn positive_multicast_timeout() {
        super::multicast_timeout(Some(MX(5))).unwrap();
    }

    #[test]
    fn positive_some_opt_multicast_timeout() {
        super::opt_unicast_timeout(Some(MX(5))).unwrap();
    }

    #[test]
    fn positive_none_opt_multicast_timeout() {
        super::opt_unicast_timeout(None).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_multicast_timeout() {
        super::multicast_timeout(None).unwrap();
    }
}
