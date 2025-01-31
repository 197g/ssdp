use log::Log;

use ssdp::header::{HeaderMut, Man, MX, ST};
use ssdp::message::{Multicast, SearchRequest};

struct SimpleLogger;

impl Log for SimpleLogger {
    fn flush(&self) {}

    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!("{} - {}", record.level(), record.args());
        }
    }
}

fn main() {
    log::set_logger(&SimpleLogger).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    // Create Our Search Request
    let mut request = SearchRequest::new();

    // Set Our Desired Headers (Not Verified By The Library)
    request.set(Man);
    request.set(MX(5));
    request.set(ST::All);

    // Collect Our Responses
    let responses = request.multicast().unwrap().into_iter().collect::<Vec<_>>();

    for (response, peer) in responses {
        println!("{peer}: {response:?}");
    }
}
