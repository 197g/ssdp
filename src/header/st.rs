use headers::{Header, HeaderName, HeaderValue};

use crate::FieldMap;

const ST_ALL_VALUE: &'static str = "ssdp:all";

/// Represents a header which specifies the search target.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ST {
    All,
    Target(FieldMap),
}

impl Header for ST {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("st");
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

        if value == ST_ALL_VALUE.as_bytes() {
            Ok(ST::All)
        } else {
            FieldMap::parse_bytes(value.as_bytes())
                .map(ST::Target)
                .ok_or_else(headers::Error::invalid)
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = match *self {
            ST::All => HeaderValue::from_static(ST_ALL_VALUE),
            ST::Target(ref n) => HeaderValue::from_str(&n.to_string()).unwrap(),
        };

        values.extend([value]);
    }
}

#[cfg(test)]
mod tests {
    use super::ST;
    use crate::FieldMap;

    #[test]
    fn positive_all() {
        let st_all_header = &[b"ssdp:all"[..].to_vec()];

        match ST::parse_header(st_all_header) {
            Ok(ST::All) => (),
            _ => panic!("Failed To Match ST::All Header"),
        }
    }

    #[test]
    fn positive_field_upnp() {
        let st_upnp_root_header = &[b"upnp:some_identifier"[..].to_vec()];

        match ST::parse_header(st_upnp_root_header) {
            Ok(ST::Target(FieldMap::UPnP(_))) => (),
            _ => panic!("Failed To Match ST::Target Header To FieldMap::UPnP"),
        }
    }

    #[test]
    fn positive_field_urn() {
        let st_urn_root_header = &[b"urn:some_identifier"[..].to_vec()];

        match ST::parse_header(st_urn_root_header) {
            Ok(ST::Target(FieldMap::URN(_))) => (),
            _ => panic!("Failed To Match ST::Target Header To FieldMap::URN"),
        }
    }

    #[test]
    fn positive_field_uuid() {
        let st_uuid_root_header = &[b"uuid:some_identifier"[..].to_vec()];

        match ST::parse_header(st_uuid_root_header) {
            Ok(ST::Target(FieldMap::UUID(_))) => (),
            _ => panic!("Failed To Match ST::Target Header To FieldMap::UUID"),
        }
    }

    #[test]
    #[should_panic]
    fn negative_multiple_headers() {
        let st_multiple_headers = &[b"uuid:some_identifier"[..].to_vec(), b"ssdp:all"[..].to_vec()];

        ST::parse_header(st_multiple_headers).unwrap();
    }
}
