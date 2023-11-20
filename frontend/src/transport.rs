use std::{
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
};

use bytes::{Bytes, BytesMut};
use interprocess::local_socket::{LocalSocketStream, ToLocalSocketName};

pub fn stream_request<S: Read + Write>(stream: &mut S, packet: Bytes) -> io::Result<Bytes> {
    stream.write(&(packet.len() as u16).to_be_bytes())?;
    stream.write(packet.as_ref())?;

    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf)?;

    let response_len = u16::from_be_bytes(buf) as usize;
    let mut response = BytesMut::zeroed(response_len);
    stream.read_exact(&mut response)?;

    Ok(response.freeze())
}

pub fn connect_ipc<'a>(name: impl ToLocalSocketName<'a>) -> io::Result<LocalSocketStream> {
    LocalSocketStream::connect(name)
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
