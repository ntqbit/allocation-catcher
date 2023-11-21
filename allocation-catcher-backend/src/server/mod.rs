use bytes::Bytes;
pub use sealed::PacketId;
use std::io;

mod transport;

pub use transport::{serve_stream, serve_tcp};

pub trait RequestHandler: Send + Sync {
    fn handle_request(&self, packet: Bytes) -> io::Result<Bytes>;
}

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

mod sealed {
    include!("../../../common/packet_id.rs");
}
