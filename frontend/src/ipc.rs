use std::io::{Read, Write};

use bytes::{Bytes, BytesMut};
use interprocess::local_socket::LocalSocketStream;

pub struct IpcClient {
    stream: LocalSocketStream,
}

impl IpcClient {
    pub fn connect(name: &str) -> Result<IpcClient, ()> {
        if let Ok(stream) = LocalSocketStream::connect(name) {
            Ok(Self { stream })
        } else {
            Err(())
        }
    }

    pub fn request(&mut self, packet: Bytes) -> Bytes {
        // TODO: remove unwraps
        self.stream
            .write(&(packet.len() as u16).to_be_bytes())
            .unwrap();
        self.stream.write(packet.as_ref()).unwrap();

        let mut buf = [0u8; 2];
        self.stream.read_exact(&mut buf).unwrap();

        let response_len = u16::from_be_bytes(buf) as usize;
        let mut response = BytesMut::zeroed(response_len);
        self.stream.read_exact(&mut response).unwrap();

        response.freeze()
    }
}
