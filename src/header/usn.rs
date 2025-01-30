use headers::{Header, HeaderName, HeaderValue};

use crate::field;
use crate::FieldMap;

/// Separator for multiple key/values in header fields.
const FIELD_PAIR_SEPARATOR: &'static str = "::";

/// Represents a header which specifies a unique service name.
///
/// Field value can hold up to two `FieldMap`'s.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct USN(pub FieldMap, pub Option<FieldMap>);

impl USN {
    pub fn new(field: FieldMap, opt_field: Option<FieldMap>) -> USN {
        USN(field, opt_field)
    }
}

impl Header for USN {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("usn");
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

        let (first, second) = match partition_pairs(value.as_bytes().iter()) {
            Some((n, Some(u))) => (FieldMap::parse_bytes(&n[..]), FieldMap::parse_bytes(&u[..])),
            Some((n, None)) => (FieldMap::parse_bytes(&n[..]), None),
            None => return Err(headers::Error::invalid()),
        };

        match first {
            Some(n) => Ok(USN(n, second)),
            None => Err(headers::Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        use core::fmt::Write as _;
        let mut value = self.0.to_string();

        if let Some(ref n) = self.1 {
            value.push_str(FIELD_PAIR_SEPARATOR);
            write!(&mut value, "{}", n).expect("string can not fail IO");
        }

        if let Ok(value) = HeaderValue::from_str(&value) {
            values.extend([value]);
        } else {
            debug_assert!(false, "Encoding usn header was invalid");
        }
    }
}

fn partition_pairs<'a, I>(header_iter: I) -> Option<(Vec<u8>, Option<Vec<u8>>)>
where
    I: Iterator<Item = &'a u8>,
{
    let mut second_partition = false;
    let mut header_iter = header_iter.peekable();

    let mut last_byte = match header_iter.peek() {
        Some(&&n) => n,
        None => return None,
    };

    // Seprate field into two vecs, store separators on end of first
    let (mut first, second): (Vec<u8>, Vec<u8>) = header_iter.cloned().partition(|&n| {
        if second_partition {
            false
        } else {
            second_partition = [last_byte, n] == FIELD_PAIR_SEPARATOR.as_bytes();
            last_byte = n;

            true
        }
    });

    // Remove up to two separators from end of first
    for _ in 0..2 {
        if let Some(&n) = first[..].last() {
            if n == field::PAIR_SEPARATOR as u8 {
                first.pop();
            }
        }
    }

    match (first.is_empty(), second.is_empty()) {
        (false, false) => Some((first, Some(second))),
        (false, true) => Some((first, None)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::USN;
    use crate::FieldMap::{UPnP, Unknown, URN, UUID};

    #[test]
    fn positive_double_pair() {
        let double_pair_header = &["uuid:device-UUID::upnp:rootdevice".to_string().into_bytes()];
        let USN(first, second) = USN::parse_header(double_pair_header).unwrap();

        match first {
            UUID(n) => assert_eq!(n, "device-UUID"),
            _ => panic!("Didnt Match uuid"),
        };

        match second.unwrap() {
            UPnP(n) => assert_eq!(n, "rootdevice"),
            _ => panic!("Didnt Match upnp"),
        };
    }

    #[test]
    fn positive_single_pair() {
        let single_pair_header = &["urn:device-URN".to_string().into_bytes()];
        let USN(first, second) = USN::parse_header(single_pair_header).unwrap();

        match first {
            URN(n) => assert_eq!(n, "device-URN"),
            _ => panic!("Didnt Match urn"),
        };

        assert!(second.is_none());
    }

    #[test]
    fn positive_trailing_double_colon() {
        let trailing_double_colon_header = &["upnp:device-UPnP::".to_string().into_bytes()];
        let USN(first, second) = USN::parse_header(trailing_double_colon_header).unwrap();

        match first {
            UPnP(n) => assert_eq!(n, "device-UPnP"),
            _ => panic!("Didnt Match upnp"),
        };

        assert!(second.is_none());
    }

    #[test]
    fn positive_trailing_single_colon() {
        let trailing_single_colon_header = &["some-key:device-UPnP:".to_string().into_bytes()];
        let USN(first, second) = USN::parse_header(trailing_single_colon_header).unwrap();

        match first {
            Unknown(k, v) => {
                assert_eq!(k, "some-key");
                assert_eq!(v, "device-UPnP");
            }
            _ => panic!("Didnt Match upnp"),
        };

        assert!(second.is_none());
    }

    #[test]
    #[should_panic]
    fn negative_empty() {
        let empty_header = &["".to_string().into_bytes()];

        USN::parse_header(empty_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_colon() {
        let colon_header = &[":".to_string().into_bytes()];

        USN::parse_header(colon_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_double_colon() {
        let double_colon_header = &["::".to_string().into_bytes()];

        USN::parse_header(double_colon_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_double_colon_value() {
        let double_colon_value_header = &["uuid:::".to_string().into_bytes()];

        USN::parse_header(double_colon_value_header).unwrap();
    }
}
