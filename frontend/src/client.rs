use std::io;

use crate::ipc::IpcClient;

pub mod sealed {
    include!("../../common/packet_id.rs");
}

use bytes::{BufMut, Bytes, BytesMut};
pub use sealed::PacketId;

pub struct Client {
    ipc_client: IpcClient,
}

impl Client {
    pub const fn new(ipc_client: IpcClient) -> Self {
        Self { ipc_client }
    }

    pub fn request(&mut self, packet_id: PacketId, data: Bytes) -> io::Result<Bytes> {
        let mut buf = BytesMut::with_capacity(data.len() + 1);
        buf.put_u8(packet_id as u8);
        buf.put(data);
        self.ipc_client.request(buf.freeze())
    }
}
