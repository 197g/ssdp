use headers::{Header, HeaderName, HeaderValue};

/// Represents a header used to denote the configuration of a device's DDD.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ConfigID(pub u32);

impl Header for ConfigID {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("configid.upnp.org");
        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let Some(value) = values.next() else {
            return Err(headers::Error::invalid())?;
        };

        let cow_str = String::from_utf8_lossy(value.as_bytes());

        // Value needs to be a 31 bit non-negative integer, so convert to i32
        let value = match i32::from_str_radix(&*cow_str, 10) {
            Ok(n) => n,
            Err(_) => return Err(headers::Error::invalid()),
        };

        // UPnP 1.1 spec says higher numbers are reserved for future use by the
        // technical committee. Devices should use numbers in the range 0 to
        // 16777215 (2^24-1) but I am not sure where the reserved numbers will
        // appear so we will ignore checking that the range is satisfied here.

        // Check if value is negative, then convert to u32
        if value.is_negative() {
            Err(headers::Error::invalid())
        } else {
            Ok(ConfigID(value as u32))
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        if let Ok(value) = headers::HeaderValue::from_str(&format!("{}", self.0)) {
            values.extend([value]);
        } else {
            debug_assert!(false, "Encoding configid header was invalid");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConfigID;
    use headers::{Header, HeaderValue};

    #[test]
    fn positive_configid() {
        let configid_header_value = ["1777215"]
            .into_iter()
            .map(HeaderValue::from_static)
            .collect::<Vec<_>>();
        ConfigID::decode(&mut configid_header_value.iter()).unwrap();
    }

    #[test]
    fn positive_reserved() {
        let configid_header_value = &["20720000"]
            .into_iter()
            .map(HeaderValue::from_static)
            .collect::<Vec<_>>();
        ConfigID::decode(&mut configid_header_value.iter()).unwrap();
    }

    #[test]
    fn positive_lower_bound() {
        let configid_header_value = &[b"0"[..].to_vec()];

        ConfigID::decode(configid_header_value).unwrap();
    }

    #[test]
    fn positive_upper_bound() {
        let configid_header_value = &[b"2147483647"[..].to_vec()];

        ConfigID::decode(configid_header_value).unwrap();
    }

    #[test]
    fn positive_negative_zero() {
        let configid_header_value = &[b"-0"[..].to_vec()];

        ConfigID::decode(configid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_overflow() {
        let configid_header_value = &[b"2290649224"[..].to_vec()];

        ConfigID::decode(configid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_negative_overflow() {
        let configid_header_value = &[b"-2290649224"[..].to_vec()];

        ConfigID::decode(configid_header_value).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_nan() {
        let configid_header_value = &[b"2290wow649224"[..].to_vec()];

        ConfigID::decode(configid_header_value).unwrap();
    }
}
