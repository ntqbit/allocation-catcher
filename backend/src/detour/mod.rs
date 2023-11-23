mod flag;
mod rtl_heap_detour;

pub use flag::{flag_set, AcquisitionGuard, DetourFlag, DetourFlagSet};
pub use rtl_heap_detour::{disable, enable, initialize, uninitialize};

#[derive(Debug)]
pub enum Error {
    CouldNotFindModule,
    CouldNotFindProc,
    HookInitializeFailed,
    HookEnableFailed,
    HookDisableFailed,
}

pub type HeapHandle = usize;

pub struct Base {
    pub heap_handle: HeapHandle,

    // Used for stack tracing and back tracing to avoid tracing the handler functions.
    pub return_address: Option<usize>,
    pub address_of_return_address: Option<usize>,
    pub stack_frame_address: Option<usize>,
}

pub struct Allocation {
    pub base: Base,
    pub size: usize,
    pub allocated_base_address: Option<usize>,
}

pub struct Reallocation {
    pub base_address: usize,
    pub allocation: Allocation,
}

pub struct Deallocation {
    pub base: Base,
    pub base_address: usize,
    pub success: bool,
}

pub trait AllocationHandler: Sync {
    fn on_allocation(&self, allocation: Allocation);

    fn on_deallocation(&self, deallocation: Deallocation);

    fn on_reallocation(&self, reallocation: Reallocation);
}

pub struct NoopAllocationHandler;

impl AllocationHandler for NoopAllocationHandler {
    fn on_allocation(&self, _allocation: Allocation) {}

    fn on_deallocation(&self, _deallocation: Deallocation) {}

    fn on_reallocation(&self, _reallocation: Reallocation) {}
}

static mut ALLOCATION_HANDLER: &'static dyn AllocationHandler = &NoopAllocationHandler;

// SAFETY: must never be called while detour is enabled
pub unsafe fn set_allocation_handler(handler: &'static dyn AllocationHandler) {
    ALLOCATION_HANDLER = handler;
}

pub unsafe fn allocation_handler() -> &'static dyn AllocationHandler {
    ALLOCATION_HANDLER
}
