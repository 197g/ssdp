//! Headers and primitives for parsing headers within SSDP requests.
//!
//! This module combines abstractions at both the HTTPU/HTTPMU layer and SSDP
//! layer in order to provide a cleaner interface for extending the underlying
//! HTTP parsing library.

use std::fmt::Debug;

use headers::Header;

mod bootid;
mod configid;
mod man;
mod mx;
mod nt;
mod nts;
mod searchport;
// mod securelocation;
mod st;
mod usn;

pub use self::bootid::BootID;
pub use self::configid::ConfigID;
pub use self::man::Man;
pub use self::mx::MX;
pub use self::nt::NT;
pub use self::nts::NTS;
pub use self::searchport::SearchPort;
// pub use self::securelocation::SecureLocation;
pub use self::st::ST;
pub use self::usn::USN;

// Re-exports
pub use headers::{CacheControl, Location, Server};

/// Trait for manipulating the contents of a header structure.
pub trait HeaderMut: Debug {
    /// Set a header to the given value.
    fn set<H>(&mut self, value: H)
    where
        H: Header;
}

impl<'a, T: ?Sized> HeaderMut for &'a mut T
where
    T: HeaderMut,
{
    fn set<H>(&mut self, value: H)
    where
        H: Header,
    {
        HeaderMut::set(*self, value)
    }
}

impl HeaderMut for headers::HeaderMap {
    fn set<H>(&mut self, value: H)
    where
        H: Header,
    {
        use headers::HeaderMapExt as _;
        self.remove(H::name());
        self.typed_insert(value);
    }
}

// #[cfg(test)]
// pub mod mock {
// use std::any::{Any};
// use std::borrow::{ToOwned};
// use std::clone::{Clone};
// use std::collections::{HashMap};
//
// use hyper::header::{Header, HeaderFormat};
//
// use ssdp::header::{HeaderView};
//
// #[derive(Debug)]
// pub struct MockHeaderView {
// map: HashMap<&'static str, (Box<Any>, [Vec<u8>; 1])>
// }
//
// impl MockHeaderView {
// pub fn new() -> MockHeaderView {
// MockHeaderView{ map: HashMap::new() }
// }
//
// pub fn insert<H>(&mut self, value: &str) where H: Header + HeaderFormat {
// let header_bytes = [value.to_owned().into_bytes()];
//
// let header = match H::parse_header(&header_bytes[..]) {
// Some(n) => n,
// None    => panic!("Failed To Parse value As Header!!!")
// };
//
// self.map.insert(H::header_name(), (Box::new(header), header_bytes));
// }
// }
//
// impl Clone for MockHeaderView {
// fn clone(&self) -> MockHeaderView {
// panic!("Can Not Clone A MockHeaderView")
// }
// }
//
// impl HeaderView for MockHeaderView {
// fn view<H>(&self) -> Option<&H> where H: Header + HeaderFormat {
// match self.map.get(H::header_name()) {
// Some(&(ref header, _)) => header.downcast_ref::<H>(),
// None => None
// }
// }
//
// fn view_raw(&self, name: &str) -> Option<&[Vec<u8>]> {
// match self.map.get(name) {
// Some(&(_, ref header_bytes)) => Some(header_bytes),
// None => None
// }
// }
// }
// }
