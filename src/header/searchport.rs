use headers::{Header, HeaderName, HeaderValue};

pub const SEARCHPORT_MIN_VALUE: u16 = 49152;

/// Represents a header used to specify a unicast port to send search requests to.
///
/// If a `SearchPort` header is not included in a message then the device must
/// respond to unicast search requests on the standard port of 1900.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SearchPort(pub u16);

impl Header for SearchPort {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("searchport.upnp.org");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let Some(value) = values.next() else {
            return Err(headers::Error::invalid())?;
        };

        if values.next().is_some() {
            return Err(headers::Error::invalid())?;
        };

        let cow_str = String::from_utf8_lossy(value.as_bytes());

        let value = match u16::from_str_radix(&*cow_str, 10) {
            Ok(n) => n,
            Err(_) => return Err(headers::Error::invalid()),
        };

        if value >= SEARCHPORT_MIN_VALUE {
            Ok(SearchPort(value))
        } else {
            Err(headers::Error::invalid())
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        if let Ok(value) = HeaderValue::from_str(&self.0.to_string()) {
            values.extend([value]);
        } else {
            debug_assert!(false, "Encoding configid header was invalid");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SearchPort;

    #[test]
    fn positive_searchport() {
        let searchport_header_value = &[b"50000"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }

    #[test]
    fn positive_lower_bound() {
        let searchport_header_value = &[b"49152"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }

    #[test]
    fn positive_upper_bound() {
        let searchport_header_value = &[b"65535"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_reserved() {
        let searchport_header_value = &[b"49151"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_nan() {
        let searchport_header_value = &[b"49151a"[..].to_vec()];

        SearchPort::parse_header(searchport_header_value).unwrap();
    }
}
