use headers::{Header, HeaderName, HeaderValue};

const SECURELOCATION_HEADER_NAME: &'static str = "SECURELOCATION.UPNP.ORG";

/// Represents a header used to specify a secure url for a device's DDD.
///
/// Can be used instead of the `Location` header field.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SecureLocation(pub String);

impl Header for SecureLocation {
    fn name() -> &'static HeaderName {
        &HeaderName::from_static(SECURELOCATION_HEADER_NAME)
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

        match core::str::from_utf8(value) {
            Ok(n) => Ok(SecureLocation(n.to_string())),
            Err(_) => Err(headers::Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        if let Ok(value) = HeaderValue::from_str(&self.0) {
            values.extend([value]);
        } else {
            debug_assert!(false, "Encoding configid header was invalid");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hyper::header::Header;

    use super::SecureLocation;

    #[test]
    fn positive_securelocation() {
        let securelocation_header_value = &[b"https://192.168.1.1/"[..].to_vec()];

        SecureLocation::parse_header(securelocation_header_value).unwrap();
    }

    #[test]
    fn positive_invalid_url() {
        let securelocation_header_value = &[b"just some text"[..].to_vec()];

        SecureLocation::parse_header(securelocation_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_empty() {
        let securelocation_header_value = &[b""[..].to_vec()];

        SecureLocation::parse_header(securelocation_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_invalid_utf8() {
        let securelocation_header_value = &[b"https://192.168.1.1/\x80"[..].to_vec()];

        SecureLocation::parse_header(securelocation_header_value).unwrap();
    }
}
