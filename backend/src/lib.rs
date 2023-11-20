#![feature(type_alias_impl_trait)]
#![feature(btreemap_alloc)]
#![feature(allocator_api)]

mod allocation_handler;
mod allocations_storage;
mod detour;
mod dllmain;
mod ipc;
mod server;
mod proto;
mod debug;
mod platform;