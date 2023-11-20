use std::io::{self, Read, Write};

use bytes::BytesMut;

use crate::server::RequestHandler;

pub fn serve_stream_client<S: Read + Write>(
    mut stream: S,
    request_handler: &'static dyn RequestHandler,
) {
    loop {
        if let Err(_) = serve_stream_client_once(&mut stream, request_handler) {
            break;
        }
    }
}

pub fn serve_stream_client_once<S: Read + Write>(
    stream: &mut S,
    request_handler: &'static dyn RequestHandler,
) -> io::Result<()> {
    let mut packet_length_buf = [0u8; 2];

    stream.read_exact(&mut packet_length_buf)?;

    let packet_length = u16::from_be_bytes(packet_length_buf) as usize;
    let mut packet = BytesMut::zeroed(packet_length);

    stream.read_exact(&mut packet)?;
    assert!(packet.len() == packet_length);

    let response = request_handler.handle_request(packet.freeze())?;
    stream.write(&(response.len() as u16).to_be_bytes())?;
    stream.write(response.as_ref())?;
    Ok(())
}
