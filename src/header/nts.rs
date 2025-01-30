use headers::{Header, HeaderName, HeaderValue};

const ALIVE_HEADER: &'static str = "ssdp:alive";
const UPDATE_HEADER: &'static str = "ssdp:update";
const BYEBYE_HEADER: &'static str = "ssdp:byebye";

/// Represents a header which specifies a notification sub type.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum NTS {
    /// An entity is announcing itself to the network.
    Alive,
    /// An entity is updating its presence on the network. Introduced in UPnP 1.0.
    ///
    /// Contrary to it's name, an update message will only appear when some UPnP
    /// enabled interface is added to an already existing UPnP device on a network.
    Update,
    /// An entity is removing itself from the network.
    ByeBye,
}

impl Header for NTS {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("nts");
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

        if value.as_bytes() == ALIVE_HEADER.as_bytes() {
            Ok(NTS::Alive)
        } else if value.as_bytes() == UPDATE_HEADER.as_bytes() {
            Ok(NTS::Update)
        } else if value.as_bytes() == BYEBYE_HEADER.as_bytes() {
            Ok(NTS::ByeBye)
        } else {
            Err(headers::Error::invalid())
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = HeaderValue::from_static(match *self {
            NTS::Alive => ALIVE_HEADER,
            NTS::Update => UPDATE_HEADER,
            NTS::ByeBye => BYEBYE_HEADER,
        });

        values.extend([value]);
    }
}

#[cfg(test)]
mod tests {
    use super::NTS;

    #[test]
    fn positive_alive() {
        let alive_header = &[b"ssdp:alive"[..].to_vec()];

        match NTS::parse_header(alive_header) {
            Ok(NTS::Alive) => (),
            _ => panic!("Didn't Match With NTS::Alive"),
        };
    }

    #[test]
    fn positive_update() {
        let update_header = &[b"ssdp:update"[..].to_vec()];

        match NTS::parse_header(update_header) {
            Ok(NTS::Update) => (),
            _ => panic!("Didn't Match With NTS::Update"),
        };
    }

    #[test]
    fn positive_byebye() {
        let byebye_header = &[b"ssdp:byebye"[..].to_vec()];

        match NTS::parse_header(byebye_header) {
            Ok(NTS::ByeBye) => (),
            _ => panic!("Didn't Match With NTS::ByeBye"),
        };
    }

    #[test]
    #[should_panic]
    fn negative_alive_extra() {
        let alive_extra_header = &[b"ssdp:alive_someotherbytes"[..].to_vec()];

        NTS::parse_header(alive_extra_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_unknown() {
        let unknown_header = &[b"ssdp:somestring"[..].to_vec()];

        NTS::parse_header(unknown_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_empty() {
        let empty_header = &[b""[..].to_vec()];

        NTS::parse_header(empty_header).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_no_value() {
        let no_value_header = &[b"ssdp:"[..].to_vec()];

        NTS::parse_header(no_value_header).unwrap();
    }
}
