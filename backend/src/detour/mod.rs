use lazy_static::lazy_static;

mod rtl_heap_detour;

pub use rtl_heap_detour::{disable, enable, initialize, is_enabled, is_initialized, uninitialize};

use crate::platform::TlsSlotAcquisition;

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

pub struct AcquisitionGuard<'a> {
    tls: &'a TlsSlotAcquisition,
    acquisition: usize,
}

impl<'a> AcquisitionGuard<'a> {
    pub const fn new(tls: &'a TlsSlotAcquisition, acquisition: usize) -> Self {
        Self { tls, acquisition }
    }

    pub fn forget(self) {
        core::mem::forget(self)
    }
}

impl<'a> Drop for AcquisitionGuard<'a> {
    fn drop(&mut self) {
        self.tls.release(self.acquisition);
    }
}

#[repr(usize)]
pub enum Slot {
    Alloc = 0,
    Free = 1,
}

// Prevention of recursive detour call
pub struct DetourLock {
    tls_slot_acquisition: TlsSlotAcquisition,
}

impl DetourLock {
    pub fn new() -> Self {
        Self {
            tls_slot_acquisition: TlsSlotAcquisition::new().expect("failed to allocate tls slot"),
        }
    }

    pub fn acquire_slot(&self, slot: Slot) -> Option<AcquisitionGuard> {
        let mask = 1 << (slot as usize);
        let acquisition = self.tls_slot_acquisition.acquire(mask);

        if acquisition != 0 {
            Some(AcquisitionGuard::new(
                &self.tls_slot_acquisition,
                acquisition,
            ))
        } else {
            None
        }
    }

    pub fn acquire_all(&self) -> AcquisitionGuard {
        AcquisitionGuard::new(
            &self.tls_slot_acquisition,
            self.tls_slot_acquisition.acquire(!0),
        )
    }

    pub fn is_acquired(&self) -> bool {
        self.tls_slot_acquisition.get() == !0
    }
}

lazy_static! {
    static ref DETOUR_LOCK: DetourLock = DetourLock::new();
}

pub fn lock() -> &'static DetourLock {
    &DETOUR_LOCK
}
