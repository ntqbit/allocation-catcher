use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use bytes::BytesMut;

use crate::{server::RequestHandler, spawn_thread};

fn serve_stream_client<S: Read + Write>(
    mut stream: S,
    request_handler: &'static dyn RequestHandler,
) -> io::Result<()> {
    loop {
        serve_stream_client_once(&mut stream, request_handler)?;
    }
}

fn serve_stream_client_once<S: Read + Write>(
    stream: &mut S,
    request_handler: &'static dyn RequestHandler,
) -> io::Result<()> {
    let mut packet_length_buf = [0u8; 4];

    stream.read_exact(&mut packet_length_buf)?;

    let packet_length = u32::from_be_bytes(packet_length_buf) as usize;
    let mut packet = BytesMut::zeroed(packet_length);

    stream.read_exact(&mut packet)?;
    assert!(packet.len() == packet_length);

    let response = request_handler.handle_request(packet.freeze())?;
    stream.write(&(response.len() as u32).to_be_bytes())?;
    stream.write(response.as_ref())?;
    Ok(())
}

pub trait TransportListener {
    type Stream: Read + Write + Sync + Send + 'static;

    fn accept(&self) -> io::Result<Self::Stream>;
}

impl TransportListener for TcpListener {
    type Stream = TcpStream;

    fn accept(&self) -> io::Result<Self::Stream> {
        let (stream, _sockadddr) = TcpListener::accept(&self)?;
        Ok(stream)
    }
}

pub fn serve_stream<T: TransportListener>(
    transport: T,
    request_handler: &'static dyn RequestHandler,
) -> io::Result<()> {
    loop {
        let stream = transport.accept()?;
        spawn_thread(|| {
            serve_stream_client(stream, request_handler).ok();
        });
    }
}

pub fn serve_tcp(
    addr: impl ToSocketAddrs,
    request_handler: &'static dyn RequestHandler,
) -> io::Result<()> {
    serve_stream(TcpListener::bind(addr)?, request_handler)
}
