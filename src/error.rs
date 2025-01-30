use hyper;
use std::io;
use std::net;

use quick_error::quick_error;

quick_error! {
    /// Enumerates all errors that can occur when dealing with an SSDP message.
    #[derive(Debug)]
    pub enum SSDPError {
        /// Message is not valid HTTP.
        ///
        /// Message is supplied as a list of bytes.
        InvalidHttp(message: Vec<u8>) {
            // description("invalid HTTP")
            display("invalid HTTP message: '{:?}'", message)
        }
        /// Message did not specify HTTP/1.1 as version.
        InvalidHttpVersion { }
        /// Message consists of an error code.
        ///
        /// Error code is supplied.
        ResponseCode(code: u16) {
            // description("HTTP Error response")
            display("HTTP Error response: {}", code)
        }
        /// Method supplied is not a valid SSDP method.
        ///
        /// Method received is supplied.
        InvalidMethod(method: String) {
            // description("invalid SSDP method")
            display("invalid SSDP method: '{}'", method)
        }
        /// Uri supplied is not a valid SSDP uri.
        ///
        /// URI received is supplied.
        InvalidUri(uri: String) {
            // description("invalid URI")
            display("invalid URI: '{}'", uri)
        }
        /// Header is missing from the message.
        ///
        /// Expected header is supplied.
        MissingHeader(header: &'static str) {
            // description("missing header")
            display("missing header: '{}'", header)
        }
        /// Header has an invalid value.
        ///
        /// Header name with error message are supplied.
        InvalidHeader(header: &'static str, msg: &'static str) {
            // description("invalid header")
            display("invalid header: '{}': {}", header, msg)
        }

        Io(err: io::Error) {
            from()
            display("IO operation failed: {}", err)
        }

        AddrParseError(err: net::AddrParseError) {
            from()
            display("invalid address: {}", err)
        }

        Hyper(err: hyper::Error) {
            from()
            display("invalid HTTP: {}", err)
        }

        HyperParseError(err: hyper::error::ParseError) {
            from()
            display("invalid HTTP message received: {}", err)
        }
    }
}

pub type SSDPResult<T> = Result<T, SSDPError>;
