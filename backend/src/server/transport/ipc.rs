use std::io;

use interprocess::local_socket::{LocalSocketListener, ToLocalSocketName};

use super::stream::serve_stream_client;
use crate::server::RequestHandler;

pub fn serve_ipc<'a>(
    name: impl ToLocalSocketName<'a>,
    request_handler: &'static dyn RequestHandler,
) -> io::Result<()> {
    let sock = LocalSocketListener::bind(name)?;

    loop {
        let stream = sock.accept()?;
        std::thread::spawn(|| serve_stream_client(stream, request_handler));
    }
}
