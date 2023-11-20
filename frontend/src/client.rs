use std::io;

use crate::{ipc::IpcClient, proto};

pub mod sealed {
    include!("../../common/packet_id.rs");
}

use bytes::{BufMut, Bytes, BytesMut};
pub use sealed::PacketId;

pub trait RequestSpec: prost::Message {
    const PACKET_ID: PacketId;

    type RESPONSE: prost::Message + Default;
}

impl RequestSpec for proto::PingRequest {
    const PACKET_ID: PacketId = PacketId::Ping;

    type RESPONSE = proto::PingResponse;
}

impl RequestSpec for proto::SetConfigurationRequest {
    const PACKET_ID: PacketId = PacketId::SetConfiguration;

    type RESPONSE = proto::SetConfigurationResponse;
}

impl RequestSpec for proto::GetConfigurationRequest {
    const PACKET_ID: PacketId = PacketId::GetConfiguration;

    type RESPONSE = proto::GetConfigurationResponse;
}

impl RequestSpec for proto::ClearStorageRequest {
    const PACKET_ID: PacketId = PacketId::ClearStorage;

    type RESPONSE = proto::ClearStorageResponse;
}

impl RequestSpec for proto::FindRequest {
    const PACKET_ID: PacketId = PacketId::Find;

    type RESPONSE = proto::FindResponse;
}

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
