mod rtl_heap_detour;

pub use rtl_heap_detour::{disable, enable, initialize, is_enabled, is_initialized, uninitialize};

#[derive(Debug)]
pub enum Error {
    CouldNotFindModule,
    CouldNotFindProc,
    HookInitializeFailed,
    HookEnableFailed,
    HookDisableFailed,
}

pub type HeapHandle = usize;

pub struct Allocation {
    pub heap_handle: HeapHandle,
    pub size: usize,
    pub allocated_base_address: Option<usize>,
}

pub struct Deallocation {
    pub heap_handle: HeapHandle,
    pub base_address: usize,
    pub success: bool,
}

pub trait AllocationHandler: Sync {
    fn on_allocation(&self, allocation: Allocation);

    fn on_deallocation(&self, deallocation: Deallocation);
}

pub struct NoopAllocationHandler;

impl AllocationHandler for NoopAllocationHandler {
    fn on_allocation(&self, _allocation: Allocation) {}

    fn on_deallocation(&self, _deallocation: Deallocation) {}
}

static mut ALLOCATION_HANDLER: &'static dyn AllocationHandler = &NoopAllocationHandler;

// SAFETY: must never be called while detour is enabled
pub unsafe fn set_allocation_handler(handler: &'static dyn AllocationHandler) {
    ALLOCATION_HANDLER = handler;
}

pub unsafe fn allocation_handler() -> &'static dyn AllocationHandler {
    ALLOCATION_HANDLER
}
