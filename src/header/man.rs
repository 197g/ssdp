use headers::{Header, HeaderName, HeaderValue};

const MAN_HEADER_VALUE: &'static str = "\"ssdp:discover\"";

/// Represents a header used to specify HTTP extension.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Man;

impl Header for Man {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("man");
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

        let man_bytes = MAN_HEADER_VALUE.as_bytes();
        match value {
            n if n == man_bytes => Ok(Man),
            _ => Err(headers::Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        values.extend([HeaderValue::from_static(MAN_HEADER_VALUE)]);
    }
}

#[cfg(test)]
mod tests {
    use super::Man;

    #[test]
    fn positive_man() {
        let man_header = &[b"\"ssdp:discover\""[..].to_vec()];

        Man::parse_header(man_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_wrong_case() {
        let wrong_case_man_header = &[b"\"SSDP:discover\""[..].to_vec()];

        Man::parse_header(wrong_case_man_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_missing_quotes() {
        let missing_quotes_man_header = &[b"ssdp:discover"[..].to_vec()];

        Man::parse_header(missing_quotes_man_header).unwrap();
    }
}
