mod ipc;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

mod sealed {
    include!("../../../common/packet_id.rs");
}

pub use ipc::{serve_ipc, RequestHandler};
pub use sealed::PacketId;
