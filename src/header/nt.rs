use headers::{Header, HeaderName, HeaderValue};

use crate::FieldMap;

/// Represents a header used to specify a notification type.
///
/// Any double colons will not be processed as separate `FieldMap`'s.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct NT(pub FieldMap);

impl NT {
    pub fn new(field: FieldMap) -> NT {
        NT(field)
    }
}

impl Header for NT {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("nt");
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

        match FieldMap::parse_bytes(value.as_bytes()) {
            Some(n) => Ok(NT(n)),
            None => Err(headers::Error::invalid()),
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
    use super::NT;
    use crate::FieldMap::{UPnP, Unknown, URN, UUID};

    #[test]
    fn positive_uuid() {
        let header = "uuid:a984bc8c-aaf0-5dff-b980-00d098bda247";

        let data = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(UUID(n))) => n,
            _ => panic!("uuid Token Not Parsed"),
        };

        assert!(header.chars().skip(5).zip(data.chars()).all(|(a, b)| a == b));
    }

    #[test]
    fn positive_upnp() {
        let header = "upnp:rootdevice";

        let data = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(UPnP(n))) => n,
            _ => panic!("upnp Token Not Parsed"),
        };

        assert!(header.chars().skip(5).zip(data.chars()).all(|(a, b)| a == b));
    }

    #[test]
    fn positive_urn() {
        let header = "urn:schemas-upnp-org:device:printer:1";

        let data = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(URN(n))) => n,
            _ => panic!("urn Token Not Parsed"),
        };

        assert!(header.chars().skip(4).zip(data.chars()).all(|(a, b)| a == b));
    }

    #[test]
    fn positive_unknown() {
        let header = "max-age:1500::upnp:rootdevice";

        let (k, v) = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(Unknown(k, v))) => (k, v),
            _ => panic!("Unknown Token Not Parsed"),
        };

        let sep_iter = ":".chars();
        let mut original_iter = header.chars();
        let mut result_iter = k.chars().chain(sep_iter).chain(v.chars());

        assert!(original_iter
            .by_ref()
            .zip(result_iter.by_ref())
            .all(|(a, b)| a == b));
        assert!(result_iter.next().is_none() && original_iter.next().is_none());
    }

    #[test]
    fn positive_short_field() {
        let header = "a:a";

        let (k, v) = match NT::parse_header(&[header.to_string().into_bytes()]) {
            Ok(NT(Unknown(k, v))) => (k, v),
            _ => panic!("Unknown Short Token Not Parsed"),
        };

        let sep_iter = ":".chars();
        let mut original_iter = header.chars();
        let mut result_iter = k.chars().chain(sep_iter).chain(v.chars());

        assert!(original_iter
            .by_ref()
            .zip(result_iter.by_ref())
            .all(|(a, b)| a == b));
        assert!(result_iter.next().is_none() && original_iter.next().is_none());
    }

    #[test]
    fn positive_leading_double_colon() {
        let leading_double_colon_header = &["uuid::a984bc8c-aaf0-5dff-b980-00d098bda247"
            .to_string()
            .into_bytes()];

        let result = match NT::parse_header(leading_double_colon_header).unwrap() {
            NT(UUID(n)) => n,
            _ => panic!("NT Double Colon Failed To Parse"),
        };

        assert_eq!(result, ":a984bc8c-aaf0-5dff-b980-00d098bda247");
    }

    #[test]
    #[should_panic]
    fn negative_double_colon() {
        let double_colon_header = &["::".to_string().into_bytes()];

        NT::parse_header(double_colon_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_single_colon() {
        let single_colon_header = &[":".to_string().into_bytes()];

        NT::parse_header(single_colon_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_empty_field() {
        let empty_header = &["".to_string().into_bytes()];

        NT::parse_header(empty_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_no_colon() {
        let no_colon_header = &["some_key-some_value".to_string().into_bytes()];

        NT::parse_header(no_colon_header).unwrap();
    }
}
