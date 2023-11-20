use std::{
    io::{Read, Write},
    sync::Arc,
};

use bytes::{Bytes, BytesMut};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};

pub trait RequestHandler: Send + Sync {
    fn handle_request(&self, packet: Bytes) -> Bytes;
}

pub struct IpcServer {
    request_handler: Arc<dyn RequestHandler>,
}

impl IpcServer {
    pub const fn new(request_handler: Arc<dyn RequestHandler>) -> Self {
        Self { request_handler }
    }

    pub fn serve(self) {
        // TODO: remove unwrap
        let sock = LocalSocketListener::bind("allocation-catcher").unwrap();

        loop {
            // TODO: remove unwrap
            let stream = sock.accept().unwrap();
            let client = IpcClientServer::new(stream, self.request_handler.clone());
            std::thread::spawn(|| client.serve());
        }
    }
}

pub struct IpcClientServer {
    stream: LocalSocketStream,
    request_handler: Arc<dyn RequestHandler>,
}

impl IpcClientServer {
    pub const fn new(stream: LocalSocketStream, request_handler: Arc<dyn RequestHandler>) -> Self {
        Self {
            stream,
            request_handler,
        }
    }

    pub fn serve(mut self) {
        let mut packet_length_buf = [0u8; 2];

        loop {
            // TODO: remove unwrap
            self.stream.read_exact(&mut packet_length_buf).unwrap();

            let packet_length = u16::from_be_bytes(packet_length_buf) as usize;
            let mut packet = BytesMut::zeroed(packet_length);

            // TODO: remove unwrap
            self.stream.read_exact(&mut packet).unwrap();
            assert!(packet.len() == packet_length);

            let response = self.request_handler.handle_request(packet.freeze());

            // TODO: remove unwrap
            self.stream
                .write(&(response.len() as u16).to_be_bytes())
                .unwrap();
            // TODO: remove unwrap
            self.stream.write(response.as_ref()).unwrap();
        }
    }
}
