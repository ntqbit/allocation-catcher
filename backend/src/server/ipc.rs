use std::io::{self, Read, Write};

use bytes::{Bytes, BytesMut};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream, ToLocalSocketName};

pub trait RequestHandler: Send + Sync {
    fn handle_request(&self, packet: Bytes) -> io::Result<Bytes>;
}

pub fn serve_ipc<'a>(
    name: impl ToLocalSocketName<'a>,
    request_handler: &'static impl RequestHandler,
) -> io::Result<()> {
    let sock = LocalSocketListener::bind(name)?;

    loop {
        let stream = sock.accept()?;
        let client = IpcClientServer::new(stream, request_handler);
        std::thread::spawn(|| client.serve());
    }
}

struct IpcClientServer<T: RequestHandler + 'static> {
    stream: LocalSocketStream,
    request_handler: &'static T,
}

impl<T: RequestHandler> IpcClientServer<T> {
    pub const fn new(stream: LocalSocketStream, request_handler: &'static T) -> Self {
        Self {
            stream,
            request_handler,
        }
    }

    pub fn serve(mut self) {
        loop {
            if let Err(_) = self.serve_iteration() {
                break;
            }
        }
    }

    fn serve_iteration(&mut self) -> io::Result<()> {
        let mut packet_length_buf = [0u8; 2];

        self.stream.read_exact(&mut packet_length_buf)?;

        let packet_length = u16::from_be_bytes(packet_length_buf) as usize;
        let mut packet = BytesMut::zeroed(packet_length);

        self.stream.read_exact(&mut packet)?;
        assert!(packet.len() == packet_length);

        let response = self.request_handler.handle_request(packet.freeze())?;
        self.stream.write(&(response.len() as u16).to_be_bytes())?;
        self.stream.write(response.as_ref())?;
        Ok(())
    }
}
