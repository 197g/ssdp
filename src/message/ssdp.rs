use std::borrow::Cow;
use std::fmt::Debug;
use std::io::Write;
use std::net::{SocketAddr, ToSocketAddrs};

use headers::{Header, HeaderMap, Host};

use httparse::{Request, Response};

use crate::header::HeaderMut;
use crate::message::MessageType;
use crate::net::{self, NetworkConnector, NetworkStream};
use crate::receiver::FromRawSSDP;
use crate::{SSDPError, SSDPResult};

/// Only Valid `SearchResponse` Code
const VALID_RESPONSE_CODE: u16 = 200;

/// Case-Sensitive Method Names
const NOTIFY_METHOD: &'static str = "NOTIFY";
const SEARCH_METHOD: &'static str = "M-SEARCH";

/// Represents an SSDP method combined with both SSDP and HTTP headers.
#[derive(Debug, Clone)]
pub struct SSDPMessage {
    method: MessageType,
    headers: HeaderMap,
}

impl SSDPMessage {
    /// Construct a new SSDPMessage.
    pub fn new(message_type: MessageType) -> SSDPMessage {
        SSDPMessage {
            method: message_type,
            headers: HeaderMap::new(),
        }
    }

    /// Get the type of this message.
    pub fn message_type(&self) -> MessageType {
        self.method
    }

    /// Get the headers contained in this message.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Send this request to the given destination address using the given connector.
    ///
    /// The host header field will be taken care of by the underlying library.
    pub fn send<A: ToSocketAddrs, C, S>(&self, connector: &mut C, dst_addr: A) -> SSDPResult<()>
    where
        C: NetworkConnector<Stream = S>,
        S: Into<Box<dyn NetworkStream + Send>>,
    {
        let dst_sock_addr = net::addr_from_trait(dst_addr)?;
        match self.method {
            MessageType::Notify => {
                trace!("Notify to: {:?}", dst_sock_addr);
                send_request(NOTIFY_METHOD, &self.headers, connector, dst_sock_addr)
            }
            MessageType::Search => {
                trace!("Sending search request...");
                send_request(SEARCH_METHOD, &self.headers, connector, dst_sock_addr)
            }
            MessageType::Response => {
                trace!("Sending response to: {:?}", dst_sock_addr);
                // This might need fixing for IPV6, passing down the IP loses the scope information
                let dst_ip_string = dst_sock_addr.ip().to_string();
                let dst_port = dst_sock_addr.port();

                let net_stream = connector.connect(&dst_ip_string[..], dst_port)?.into();

                send_response(&self.headers, net_stream)
            }
        }
    }
}

#[allow(unused)]
/// Send a request using the connector with the supplied method and headers.
fn send_request<C, S>(
    method: &str,
    headers: &HeaderMap,
    connector: &mut C,
    dst_addr: SocketAddr,
) -> SSDPResult<()>
where
    C: NetworkConnector<Stream = S>,
    S: Into<Box<dyn NetworkStream + Send>>,
{
    trace!("Trying to parse url...");
    let url = url_from_addr(dst_addr)?;
    trace!("Url: {}", url);

    let mut request = Request {
        method: Some(&method),
        path: Some("*"),
        version: Some(1),
        headers: &mut [],
    };

    trace!("Copying headers...");
    let mut headers = headers.clone();
    trace!("Setting length");
    headers.set(headers::ContentLength(0));

    trace!("actual .send ...");
    // request.start()?.send()?;

    Ok(())
}

/// Send an Ok response on the Writer with the supplied headers.
fn send_response<W>(headers: &HeaderMap, mut dst_writer: W) -> SSDPResult<()>
where
    W: Write,
{
    let mut headers = headers.clone();
    headers.set(headers::ContentLength(0));

    let mut response = Response {
        version: Some(1),
        code: Some(200),
        reason: Some("OK"),
        headers: &mut [],
    };

    // Have to make sure response is destroyed here for lifetime issues with temp_headers
    // response.start()?.end()?;

    Ok(())
}

/// Convert the given address to a Url with a base of "httpm://".
fn url_from_addr(addr: SocketAddr) -> SSDPResult<url::Url> {
    use url::{Host, Origin};

    let (host, port);
    match addr {
        SocketAddr::V4(v4) => {
            port = v4.port();
            host = Host::Ipv4(*v4.ip())
        }
        SocketAddr::V6(v6) => {
            port = v6.port();
            host = Host::Ipv6(*v6.ip())
        }
    };

    let url = Origin::Tuple("httpm".to_string(), host, port).ascii_serialization();
    Ok(url::Url::parse(&url).expect("origin parses as url"))
}

impl HeaderMut for SSDPMessage {
    fn set<H>(&mut self, value: H)
    where
        H: headers::Header,
    {
        HeaderMut::set(&mut self.headers, value)
    }
}

impl FromRawSSDP for SSDPMessage {
    fn from_packet(bytes: &[u8]) -> SSDPResult<SSDPMessage> {
        let http1 = httparse::ParserConfig::default();

        fn is_complete(status: httparse::Status<usize>) -> SSDPResult<usize> {
            match status {
                httparse::Status::Complete(n) => Ok(n),
                httparse::Status::Partial => Err(SSDPError::PartialHttp),
            }
        }

        // Cheap check, is this a response or a request? Note that the `/` makes this an invalid
        // method so we can not confuse these two cases.
        //
        // On header parsing note that most requests should not have more than this count of
        // headers. Each here is two bytes making the stack usage still quite small.
        let mut initial_buffer = [httparse::EMPTY_HEADER; 32];
        if bytes.starts_with(b"HTTP/1") {
            let mut fallback_buffer: Box<[httparse::Header]>;
            let mut response = httparse::Response::new(&mut initial_buffer);

            let after_header_count = match http1.parse_response(&mut response, bytes) {
                Ok(count) => is_complete(count)?,
                Err(httparse::Error::TooManyHeaders) => {
                    fallback_buffer = vec![httparse::EMPTY_HEADER; 1 << 12].into();
                    response = httparse::Response::new(&mut fallback_buffer[..]);
                    is_complete(http1.parse_response(&mut response, bytes)?)?
                }
                Err(other) => {
                    return Err(other)?;
                }
            };

            let Some(body) = bytes.get(after_header_count..) else {
                return Err(SSDPError::PartialHttp);
            };

            if !body.is_empty() {
                return Err(SSDPError::InvalidBodyForMethod("M-SEARCH".into()));
            }

            let message_result = message_from_response(response);
            log_message_result(&message_result, bytes);

            message_result
        } else {
            let mut fallback_buffer: Box<[httparse::Header]>;
            let mut request = httparse::Request::new(&mut initial_buffer);

            let after_header_count = match http1.parse_request(&mut request, bytes) {
                Ok(count) => is_complete(count)?,
                Err(httparse::Error::TooManyHeaders) => {
                    fallback_buffer = vec![httparse::EMPTY_HEADER; 1 << 12].into();
                    request = httparse::Request::new(&mut fallback_buffer[..]);
                    is_complete(http1.parse_request(&mut request, bytes)?)?
                }
                Err(other) => {
                    return Err(other)?;
                }
            };

            let Some(body) = bytes.get(after_header_count..) else {
                return Err(SSDPError::PartialHttp);
            };

            let method = request.method.unwrap();
            let message_result = message_from_request(request);
            log_message_result(&message_result, bytes);

            if !body.is_empty() {
                let method = method.to_string().into();
                return Err(SSDPError::InvalidBodyForMethod(method));
            }

            message_result
        }
    }
}

/// Logs a debug! message based on the value of the `SSDPResult`.
fn log_message_result(result: &SSDPResult<SSDPMessage>, message: &[u8]) {
    match *result {
        Ok(_) => debug!("Received Valid SSDPMessage:\n{}", String::from_utf8_lossy(message)),
        Err(ref e) => debug!("Received Invalid SSDPMessage Error: {}", e),
    }
}

/// Attempts to construct an `SSDPMessage` from the given request pieces.
fn message_from_request(parts: Request<'_, '_>) -> SSDPResult<SSDPMessage> {
    validate_http_version(parts.version)?;
    let headers = validate_http_headers(&parts.headers)?;

    // Shouldn't have to do this but hyper doesn't make sure that HTTP/1.1
    // messages contain Host headers so we will assure conformance ourselves.
    if headers.get(Host::name()).is_none() {
        return Err(SSDPError::MissingHeader(Host::name().as_str()).into());
    }

    match parts.path.expect("filled by httparse") {
        "*" => {}
        n => {
            return Err(SSDPError::InvalidUri(n.to_string()))?;
        }
    };

    match parts.method.expect("filled by httparse") {
        NOTIFY_METHOD => Ok(SSDPMessage {
            method: MessageType::Notify,
            headers: headers,
        }),
        SEARCH_METHOD => Ok(SSDPMessage {
            method: MessageType::Search,
            headers: headers,
        }),
        n => Err(SSDPError::InvalidMethod(n.to_string()).into()),
    }
}

/// Attempts to construct an `SSDPMessage` from the given response pieces.
fn message_from_response(parts: Response<'_, '_>) -> SSDPResult<SSDPMessage> {
    let status_code = parts.code.expect("filled by httparse");

    validate_http_version(parts.version)?;
    validate_response_code(status_code)?;
    let headers = validate_http_headers(&parts.headers)?;

    Ok(SSDPMessage {
        method: MessageType::Response,
        headers: headers,
    })
}

/// Validate the HTTP version for an SSDP message.
///
/// Request lines for HTTPU and HTTPMU requests MUST use HTTP/1.1 as the version.
///
/// Assumes that we parsed the request as HTTP1 in the first place.
fn validate_http_version(minor: Option<u8>) -> SSDPResult<()> {
    if minor != Some(1) {
        Err(SSDPError::InvalidHttpVersion.into())
    } else {
        Ok(())
    }
}

/// Validate that the Host header is present.
fn validate_http_headers(headers: &[httparse::Header<'_>]) -> SSDPResult<HeaderMap> {
    let mut map = HeaderMap::new();

    for hdr in headers {
        let key = headers::HeaderName::from_bytes(hdr.name.as_bytes())
            .map_err(|_| SSDPError::InvalidHeader(Cow::Owned(hdr.name.to_string())))?;
        let value = headers::HeaderValue::from_bytes(hdr.value)
            .map_err(|_| SSDPError::InvalidHeader(Cow::Owned(hdr.name.to_string())))?;
        map.insert(key, value);
    }

    Ok(map)
}

/// Validate the response code for an SSDP message.
fn validate_response_code(code: u16) -> SSDPResult<()> {
    if code != VALID_RESPONSE_CODE {
        Err(SSDPError::ResponseCode(code).into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod mocks {
    use std::cell::RefCell;
    use std::io::{self, ErrorKind, Read, Write};
    use std::net::SocketAddr;
    use std::sync::mpsc::{self, Receiver, Sender};

    use crate::net::{NetworkConnector, NetworkStream};

    pub struct MockConnector {
        pub receivers: RefCell<Vec<Receiver<Vec<u8>>>>,
    }

    impl MockConnector {
        pub fn new() -> MockConnector {
            MockConnector {
                receivers: RefCell::new(Vec::new()),
            }
        }
    }

    impl NetworkConnector for MockConnector {
        type Stream = MockStream;

        fn connect(&self, _: &str, _: u16) -> io::Result<Self::Stream> {
            let (send, recv) = mpsc::channel();

            self.receivers.borrow_mut().push(recv);

            Ok(MockStream { sender: send })
        }
    }

    pub struct MockStream {
        sender: Sender<Vec<u8>>,
    }

    impl NetworkStream for MockStream {
        fn peer_addr(&mut self) -> io::Result<SocketAddr> {
            Err(io::Error::new(ErrorKind::AddrNotAvailable, ""))
        }
    }

    impl Read for MockStream {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(ErrorKind::ConnectionAborted, ""))
        }
    }

    impl Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            // Hyper will generate a request with a /, we need to intercept that.
            let mut buffer = vec![0u8; buf.len()];

            let mut found = false;
            for (src, dst) in buf.iter().zip(buffer.iter_mut()) {
                if *src == b'/' && !found && buf[0] != b'H' {
                    *dst = b'*';
                    found = true;
                } else {
                    *dst = *src;
                }
            }

            self.sender.send(buffer).unwrap();

            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    mod send {
        use std::sync::mpsc::Receiver;

        use super::super::mocks::MockConnector;
        use super::super::SSDPMessage;
        use crate::message::MessageType;

        fn join_buffers(recv_list: &[Receiver<Vec<u8>>]) -> Vec<u8> {
            let mut buffer = Vec::new();

            for recv in recv_list {
                for recv_buf in recv {
                    buffer.extend(&recv_buf[..])
                }
            }

            buffer
        }

        #[test]
        fn positive_search_method_line() {
            let message = SSDPMessage::new(MessageType::Search);
            let mut connector = MockConnector::new();

            message.send(&mut connector, ("127.0.0.1", 0)).unwrap();

            let sent_message = String::from_utf8(join_buffers(&*connector.receivers.borrow())).unwrap();

            assert_eq!(&sent_message[..19], "M-SEARCH * HTTP/1.1");
        }

        #[test]
        fn positive_notify_method_line() {
            let message = SSDPMessage::new(MessageType::Notify);
            let mut connector = MockConnector::new();

            message.send(&mut connector, ("127.0.0.1", 0)).unwrap();

            let sent_message = String::from_utf8(join_buffers(&*connector.receivers.borrow())).unwrap();

            assert_eq!(&sent_message[..17], "NOTIFY * HTTP/1.1");
        }

        #[test]
        fn positive_response_method_line() {
            let message = SSDPMessage::new(MessageType::Response);
            let mut connector = MockConnector::new();

            message.send(&mut connector, ("127.0.0.1", 0)).unwrap();

            let sent_message = String::from_utf8(join_buffers(&*connector.receivers.borrow())).unwrap();

            assert_eq!(&sent_message[..15], "HTTP/1.1 200 OK");
        }

        #[test]
        fn positive_host_header() {
            let message = SSDPMessage::new(MessageType::Search);
            let mut connector = MockConnector::new();

            message.send(&mut connector, ("127.0.0.1", 0)).unwrap();

            let sent_message = String::from_utf8(join_buffers(&*connector.receivers.borrow())).unwrap();

            assert!(sent_message.contains("Host: 127.0.0.1:0"));
        }
    }

    mod parse {
        use super::super::SSDPMessage;
        use crate::receiver::FromRawSSDP;

        #[test]
        fn positive_valid_http() {
            let raw_message = "NOTIFY * HTTP/1.1\r\nHOST: 192.168.1.1\r\n\r\n";

            SSDPMessage::from_packet(raw_message.as_bytes()).unwrap();
        }

        #[test]
        fn positive_intact_header() {
            let raw_message = "NOTIFY * HTTP/1.1\r\nHOST: 192.168.1.1\r\n\r\n";
            let message = SSDPMessage::from_packet(raw_message.as_bytes()).unwrap();
            use headers::HeaderMapExt as _;

            assert_eq!(message.headers.typed_get::<headers::Host>().unwrap().hostname(), "192.168.1.1");
        }

        #[test]
        #[should_panic]
        fn negative_http_version() {
            let raw_message = "NOTIFY * HTTP/2.0\r\nHOST: 192.168.1.1\r\n\r\n";

            SSDPMessage::from_packet(raw_message.as_bytes()).unwrap();
        }

        #[test]
        #[should_panic]
        fn negative_no_host() {
            let raw_message = "NOTIFY * HTTP/1.1\r\n\r\n";

            SSDPMessage::from_packet(raw_message.as_bytes()).unwrap();
        }

        #[test]
        #[should_panic]
        fn negative_path_included() {
            let raw_message = "NOTIFY / HTTP/1.1\r\n\r\n";

            SSDPMessage::from_packet(raw_message.as_bytes()).unwrap();
        }
    }
}
