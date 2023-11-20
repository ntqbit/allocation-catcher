#![feature(type_alias_impl_trait)]
#![feature(btreemap_alloc)]
#![feature(allocator_api)]

mod allocation_handler;
mod allocations_storage;
mod debug;
mod detour;
mod platform;
mod server;
mod state;

use std::{ffi::CString, io, iter, net::SocketAddr};

use crate::{
    allocation_handler::AllocationHandlerImpl,
    allocations_storage::{Address, Allocation, StorageImpl},
    debug::debug_message,
    server::{proto, serve_tcp, PacketId, RequestHandler},
    state::{Configuration, State, StateRef},
};
use bytes::{Bytes, BytesMut};
use num_enum::TryFromPrimitive;
use prost::Message;
use static_cell::make_static;
use winapi::{
    shared::minwindef::{BOOL, HINSTANCE},
    um::{
        winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        winuser::MessageBoxA,
    },
};

pub struct MyServer {
    state: StateRef,
}

impl RequestHandler for MyServer {
    fn handle_request(&self, mut packet: Bytes) -> io::Result<Bytes> {
        let packet_id_num = packet[0];
        let packet_id = PacketId::try_from_primitive(packet_id_num)
            .map_err(|_| io::Error::from(io::ErrorKind::ConnectionReset))?;
        self.request_inner(packet_id, packet.split_off(1))
            .ok_or_else(|| io::Error::from(io::ErrorKind::ConnectionReset))
    }
}

impl MyServer {
    pub const fn new(state: StateRef) -> Self {
        Self { state }
    }

    fn handle_find(&self, req: proto::FindRequest) -> Vec<proto::FoundAllocation> {
        let _ack = self.state.acquire_all();
        let storage = self.state.get_storage();

        let find_record = |record: &proto::FindRecord| {
            let allocations = if let Some(filter) = record.filter.as_ref() {
                let location = filter.location.as_ref().expect("location must be set");
                match location {
                    proto::filter::Location::Address(address) => {
                        if let Some(allocation) = storage.find(*address as Address) {
                            Box::new(iter::once(allocation))
                        } else {
                            Box::new(iter::empty()) as Box<dyn Iterator<Item = &Allocation>>
                        }
                    }
                    proto::filter::Location::Range(range) => {
                        storage.find_range(range.lower as Address, range.upper as Address)
                    }
                }
            } else {
                storage.dump()
            };

            proto::FoundAllocation {
                id: record.id,
                allocations: allocations.map(|x| x.into()).collect(),
            }
        };

        req.records.iter().map(find_record).collect()
    }

    fn request_inner(&self, packet_id: PacketId, data: Bytes) -> Option<Bytes> {
        let mut response = BytesMut::new();

        match packet_id {
            PacketId::Ping => {
                let req = proto::PingRequest::decode(data).ok()?;

                proto::PingResponse {
                    version: 1,
                    num: req.num,
                }
                .encode(&mut response)
                .ok()?;
            }
            PacketId::SetConfiguration => {
                let req = proto::SetConfigurationRequest::decode(data).ok()?;

                self.state.set_configuration(req.configuration?.into());

                proto::SetConfigurationResponse {}
                    .encode(&mut response)
                    .ok()?;
            }
            PacketId::GetConfiguration => {
                let _req = proto::GetConfigurationRequest::decode(data).ok()?;

                proto::GetConfigurationResponse {
                    configuration: Some(self.state.get_configuration().into()),
                }
                .encode(&mut response)
                .ok()?;
            }
            PacketId::ClearStorage => {
                let _req = proto::ClearStorageRequest::decode(data).ok()?;

                {
                    let _a = self.state.acquire_all();
                    self.state.get_storage().clear();
                }

                proto::ClearStorageResponse {}.encode(&mut response).ok()?;
            }
            PacketId::Find => {
                let req = proto::FindRequest::decode(data).ok()?;

                proto::FindResponse {
                    allocations: self.handle_find(req),
                }
                .encode(&mut response)
                .ok()?;
            }
        }

        Some(response.freeze())
    }
}

fn initialize_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        if let Ok(cstring) = CString::new(panic_info.to_string()) {
            unsafe {
                MessageBoxA(0 as _, cstring.as_ptr() as _, b"PANIC\0".as_ptr() as _, 0);
            }
        } else {
            unsafe {
                MessageBoxA(
                    0 as _,
                    b"panic info contained a nul byte\0".as_ptr() as _,
                    b"PANIC\0".as_ptr() as _,
                    0,
                );
            }
        }
    }));
}

fn initialize_detour(state: StateRef) {
    unsafe {
        detour::set_allocation_handler(make_static!(AllocationHandlerImpl::new(state)));

        debug_message!("set_allocation_handler done");

        detour::initialize().expect("detour initialize failed");
        debug_message!("detour initialized");

        detour::enable().expect("detour enable failed");
        debug_message!("detour enabled");
    }
}

fn initialize() {
    initialize_panic_handler();

    debug_message!("Initialize");

    let state = make_static!(State::new(
        Configuration::default(),
        Box::new(StorageImpl::new())
    ));

    initialize_detour(state);
    debug_message!("Detour initialized");

    let server = make_static!(MyServer::new(state));

    std::thread::spawn(|| serve_tcp(&SocketAddr::from(([0, 0, 0, 0], 9940)), server));
}

fn deinitialize() {
    unsafe {
        detour::disable().expect("detour disable failed");
        detour::uninitialize().expect("detour uninitialize failed");
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _module: HINSTANCE,
    reason: u32,
    _reserved: *mut winapi::ctypes::c_void,
) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => initialize(),
        DLL_PROCESS_DETACH => deinitialize(),
        _ => {}
    }

    1
}
