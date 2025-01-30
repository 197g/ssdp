use headers::{Header, HeaderName, HeaderValue};

/// Represents a header used to denote the boot instance of a root device.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct BootID(pub u32);

impl Header for BootID {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("bootid.upnp.org");
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

        // Value needs to be a 31 bit non-negative integer, so convert to i32
        let value = match i32::from_str_radix(&*cow_str, 10) {
            Ok(n) => n,
            Err(_) => return Err(headers::Error::invalid()),
        };

        // Check if value is negative, then convert to u32
        if value.is_negative() {
            Err(headers::Error::invalid())
        } else {
            Ok(BootID(value as u32))
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        if let Ok(value) = HeaderValue::from_str(&format!("{}", self.0)) {
            values.extend([value]);
        } else {
            debug_assert!(false, "Encoding configid header was invalid");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BootID;

    #[test]
    fn positive_bootid() {
        let bootid_header_value = &[b"1216907400"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    fn positive_leading_zeros() {
        let bootid_header_value = &[b"0000001216907400"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    fn positive_lower_bound() {
        let bootid_header_value = &[b"0"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    fn positive_upper_bound() {
        let bootid_header_value = &[b"2147483647"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    fn positive_negative_zero() {
        let bootid_header_value = &[b"-0"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_overflow() {
        let bootid_header_value = &[b"2290649224"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_negative_overflow() {
        let bootid_header_value = &[b"-2290649224"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_nan() {
        let bootid_header_value = &[b"2290wow649224"[..].to_vec()];

        BootID::parse_header(bootid_header_value).unwrap();
    }
}
