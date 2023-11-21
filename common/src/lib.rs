use num_enum::TryFromPrimitive;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum PacketId {
    Ping = 1,
    SetConfiguration = 2,
    GetConfiguration = 3,
    ClearStorage = 4,
    Find = 5,
    GetStatistics = 6,
    ResetStatistics = 7,
}
