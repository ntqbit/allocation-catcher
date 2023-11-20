use std::io;

use bytes::{BufMut, Bytes, BytesMut};

use crate::transport::Transport;

mod sealed {
    include!("../../common/packet_id.rs");
}

pub use sealed::PacketId;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

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
    transport: Box<dyn Transport>,
}

impl Client {
    pub const fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }

    pub fn request(&mut self, packet_id: PacketId, data: Bytes) -> io::Result<Bytes> {
        let mut buf = BytesMut::with_capacity(data.len() + 1);
        buf.put_u8(packet_id as u8);
        buf.put(data);
        self.transport.request(buf.freeze())
    }
}
