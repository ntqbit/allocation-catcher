use bytes::Bytes;
use std::io;

mod transport;

pub use common::{proto, PacketId};
pub use transport::{
    serve_stream, serve_stream_client, serve_stream_client_once, serve_tcp, TransportListener,
};

pub trait RequestHandler: Send + Sync {
    fn handle_request(&self, packet: Bytes) -> io::Result<Bytes>;
}
