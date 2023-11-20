#![feature(type_alias_impl_trait)]
#![feature(btreemap_alloc)]
#![feature(allocator_api)]

mod allocation_handler;
mod allocations_storage;
mod debug;
mod detour;
mod dllmain;
mod ipc;
mod platform;
mod proto;
mod server;
mod state;
