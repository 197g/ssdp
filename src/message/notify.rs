use std::fmt::Debug;

use headers::Header;

use crate::error::{SSDPError::InvalidMethod, SSDPResult};
use crate::header::HeaderMut;
use crate::message::multicast::{self, Multicast};
use crate::message::ssdp::SSDPMessage;
use crate::message::{Config, Listen, MessageType};
use crate::receiver::FromRawSSDP;

/// Notify message that can be sent via multicast to devices on the network.
#[derive(Debug, Clone)]
pub struct NotifyMessage {
    message: SSDPMessage,
}

impl NotifyMessage {
    /// Construct a new NotifyMessage.
    pub fn new() -> Self {
        NotifyMessage {
            message: SSDPMessage::new(MessageType::Notify),
        }
    }
}

impl Multicast for NotifyMessage {
    type Item = ();

    fn multicast_with_config(&self, config: &Config) -> SSDPResult<Self::Item> {
        multicast::send(&self.message, config)?;
        Ok(())
    }
}

impl Default for NotifyMessage {
    fn default() -> Self {
        NotifyMessage::new()
    }
}

impl FromRawSSDP for NotifyMessage {
    fn from_packet(bytes: &[u8]) -> SSDPResult<NotifyMessage> {
        let message = SSDPMessage::from_packet(bytes)?;

        if message.message_type() != MessageType::Notify {
            Err(InvalidMethod("SSDP Message Received Is Not A NotifyMessage".into()))
        } else {
            Ok(NotifyMessage { message: message })
        }
    }
}

impl HeaderMut for NotifyMessage {
    fn set<H>(&mut self, value: H)
    where
        H: Header,
    {
        self.message.set(value)
    }
}

/// Notify listener that can listen to notify messages sent within the network.
pub struct NotifyListener;

impl Listen for NotifyListener {
    type Message = NotifyMessage;
}

#[cfg(test)]
mod tests {
    use super::NotifyMessage;
    use crate::receiver::FromRawSSDP;

    #[test]
    fn positive_notify_message_type() {
        let raw_message = "NOTIFY * HTTP/1.1\r\nHOST: 192.168.1.1\r\n\r\n";

        NotifyMessage::from_packet(raw_message.as_bytes()).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_search_message_type() {
        let raw_message = "M-SEARCH * HTTP/1.1\r\nHOST: 192.168.1.1\r\n\r\n";

        NotifyMessage::from_packet(raw_message.as_bytes()).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_response_message_type() {
        let raw_message = "HTTP/1.1 200 OK\r\n\r\n";

        NotifyMessage::from_packet(raw_message.as_bytes()).unwrap();
    }
}
