use bytes::Bytes;

pub mod sealed {
    include!("../../common/packet_id.rs");
}

pub use sealed::PacketId;

pub trait Server: Send + Sync {
    fn request(&self, packet_id: PacketId, data: Bytes) -> Bytes;
}
