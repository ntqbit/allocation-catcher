use std::{
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
};

use bytes::{Bytes, BytesMut};

pub fn stream_request<S: Read + Write>(stream: &mut S, packet: Bytes) -> io::Result<Bytes> {
    stream.write(&(packet.len() as u32).to_be_bytes())?;
    stream.write(packet.as_ref())?;

    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf)?;

    let response_len = u32::from_be_bytes(buf) as usize;
    let mut response = BytesMut::zeroed(response_len);
    stream.read_exact(&mut response)?;

    Ok(response.freeze())
}

pub fn connect_tcp(addrs: impl ToSocketAddrs) -> io::Result<TcpStream> {
    TcpStream::connect(addrs)
}

pub trait Transport {
    fn request(&mut self, packet: Bytes) -> io::Result<Bytes>;
}

impl<S: Read + Write> Transport for S {
    fn request(&mut self, packet: Bytes) -> io::Result<Bytes> {
        stream_request(self, packet)
    }
}
