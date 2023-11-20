use std::{
    io,
    net::{TcpListener, ToSocketAddrs},
};

use crate::server::RequestHandler;

use super::stream::serve_stream_client;

pub fn serve_tcp(
    addr: impl ToSocketAddrs,
    request_handler: &'static dyn RequestHandler,
) -> io::Result<()> {
    let sock = TcpListener::bind(addr)?;

    loop {
        let (stream, _socket_addr) = sock.accept()?;
        std::thread::spawn(move || serve_stream_client(stream, request_handler));
    }
}
