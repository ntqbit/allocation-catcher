use std::io::{self, Read, Write};

use bytes::{Bytes, BytesMut};
use interprocess::local_socket::LocalSocketStream;

pub struct IpcClient {
    stream: LocalSocketStream,
}

impl IpcClient {
    pub fn connect(name: &str) -> io::Result<Self> {
        Ok(Self {
            stream: LocalSocketStream::connect(name)?,
        })
    }

    pub fn request(&mut self, packet: Bytes) -> io::Result<Bytes> {
        self.stream.write(&(packet.len() as u16).to_be_bytes())?;
        self.stream.write(packet.as_ref())?;

        let mut buf = [0u8; 2];
        self.stream.read_exact(&mut buf)?;

        let response_len = u16::from_be_bytes(buf) as usize;
        let mut response = BytesMut::zeroed(response_len);
        self.stream.read_exact(&mut response)?;

        Ok(response.freeze())
    }
}
