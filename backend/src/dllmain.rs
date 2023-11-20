use std::{
    ffi::CString,
    sync::{Arc, Mutex},
};

use crate::{
    allocation_handler::AllocationHandlerImpl,
    allocations_storage::{AllocationsStorage, StorageImpl},
    debug::debug_message,
    detour,
    ipc::{IpcServer, RequestHandler},
    proto,
    server::{PacketId, Server},
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

pub struct MyRequestHandler<T: Server> {
    server: Box<T>,
}

impl<T: Server> MyRequestHandler<T> {
    pub const fn new(server: Box<T>) -> Self {
        Self { server }
    }
}

impl<T: Server> RequestHandler for MyRequestHandler<T> {
    fn handle_request(&self, mut packet: Bytes) -> Bytes {
        let packet_id_num = packet[0];
        // TODO: handle error
        let packet_id = PacketId::try_from_primitive(packet_id_num).unwrap();

        self.server.request(packet_id, packet.split_off(1))
    }
}

pub struct MyServer {}

impl MyServer {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Server for MyServer {
    fn request(&self, packet_id: PacketId, data: Bytes) -> Bytes {
        let mut response = BytesMut::new();

        match packet_id {
            PacketId::Ping => {
                let ping_request = proto::PingRequest::decode(data).unwrap();
                let ping_response = proto::PingResponse {
                    version: 1,
                    num: ping_request.num,
                };
                ping_response.encode(&mut response).unwrap();
            }
        }

        response.freeze()
    }
}

fn initialize_detour(storage: Arc<Mutex<dyn AllocationsStorage>>) {
    unsafe {
        detour::set_allocation_handler(make_static!(AllocationHandlerImpl::new(storage)));

        debug_message("set_allocation_handler done");
        // TODO: remove unwrap
        detour::initialize().unwrap();
        debug_message("detour initialized");

        detour::enable().unwrap();
        debug_message("detour enabled");
    }
}

fn initialize() {
    debug_message("Initialize");

    let storage = Arc::new(Mutex::new(StorageImpl::new()));
    debug_message("Storage created");

    std::panic::set_hook(Box::new(|panic_info| {
        let cstring = CString::new(panic_info.to_string()).unwrap();
        unsafe {
            MessageBoxA(0 as _, cstring.as_ptr() as _, b"PANIC\0".as_ptr() as _, 0);
        }
    }));

    initialize_detour(storage.clone());
    debug_message("Detour initialized");
    debug_message("test initialized");

    let ipc_server = IpcServer::new(Arc::new(MyRequestHandler::new(Box::new(MyServer::new()))));

    debug_message("IPC server created");

    std::thread::spawn(|| ipc_server.serve());
}

fn deinitialize() {
    unsafe {
        // TODO: remove unwrap
        detour::disable().unwrap();
        detour::uninitialize().unwrap();
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
