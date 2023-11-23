#![feature(type_alias_impl_trait)]
#![feature(link_llvm_intrinsics)]

#![allow(internal_features)]

mod debug;
mod detour;
mod handler;
mod platform;
mod state;
pub mod storage;

use static_cell::make_static;

pub use detour::AllocationHandler;
pub use handler::StorageAllocationHandler;
pub use state::{Configuration, State, StateRef, Statistics};
pub use storage::{AllocationsStorage, BtreeMapStorage};

pub fn wordsize() -> u32 {
    core::mem::size_of::<usize>() as u32
}

pub fn spawn_thread<F, T>(f: F) -> std::thread::JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(|| {
        // Disable detour calls for this thread.
        detour::flag_set()
            .acquire(detour::DetourFlag::Lock)
            .expect("detour lock must not be locked in new thread")
            .forget();

        f()
    })
}

pub struct AllocationCatcher {
    state: StateRef,
}

#[derive(Default)]
pub struct Options {
    pub initial_configuration: Option<Configuration>,
    pub storage: Option<Box<dyn AllocationsStorage>>,
}

impl AllocationCatcher {
    // SAFETY: Must be called only once
    pub unsafe fn init(options: Options) -> AllocationCatcher {
        // REQUIRED: initializes the detour lock.
        let _ack = detour::flag_set()
            .acquire(detour::DetourFlag::Lock)
            .expect("unexpectedly failed to acquire a lock during initialization");

        let configuration = options.initial_configuration.unwrap_or_default();
        let storage = options
            .storage
            .unwrap_or_else(|| Box::new(BtreeMapStorage::new()));

        let state = make_static!(State::new(configuration, storage));

        unsafe {
            detour::initialize().expect("detour initialize failed");
        }

        Self { state }
    }

    // SAFETY: must not be called when detour is enabled
    pub unsafe fn set_allocation_handler(
        &self,
        allocation_handler: &'static dyn AllocationHandler,
    ) {
        detour::set_allocation_handler(allocation_handler);
    }

    pub fn state(&self) -> StateRef {
        self.state
    }

    pub unsafe fn enable(&self) {
        detour::enable().expect("detour enable failed");
    }

    pub unsafe fn disable(&self) {
        detour::disable().expect("detour disable failed");
    }
}

impl Drop for AllocationCatcher {
    fn drop(&mut self) {
        unsafe {
            detour::disable().ok();
            detour::uninitialize().ok();
        }
    }
}
