use std::sync::atomic::{AtomicBool, Ordering};

use retour::static_detour;
use winapi::{
    shared::{
        basetsd::SIZE_T,
        minwindef::BOOL,
        ntdef::{PVOID, ULONG},
    },
    um::libloaderapi::{GetProcAddress, LoadLibraryA},
};

use super::{lock, Slot};

use super::{allocation_handler, Allocation, Deallocation, Error};

static_detour! {
    static RtlAllocateHeapHook: extern "system" fn(
        PVOID,
        ULONG,
        SIZE_T
      ) -> PVOID;

    static RtlFreeHeapHook: extern "system" fn(
        PVOID,
        ULONG,
        PVOID
    ) -> BOOL;
}

static mut INITIAILIZED: AtomicBool = AtomicBool::new(false);

#[allow(non_snake_case)]
fn RtlAllocateHeapDetour(HeapHandle: PVOID, Flags: ULONG, Size: SIZE_T) -> PVOID {
    let base_address = RtlAllocateHeapHook.call(HeapHandle, Flags, Size);

    if let Some(_guard) = lock().acquire_slot(Slot::Alloc) {
        unsafe { allocation_handler() }.on_allocation(Allocation {
            heap_handle: HeapHandle as usize,
            size: Size as usize,
            allocated_base_address: if base_address.is_null() {
                None
            } else {
                Some(base_address as usize)
            },
        });
    }

    base_address
}

#[allow(non_snake_case)]
fn RtlFreeHeapDetour(HeapHandle: PVOID, Flags: ULONG, BaseAddress: PVOID) -> BOOL {
    let success = RtlFreeHeapHook.call(HeapHandle, Flags, BaseAddress);

    if let Some(_guard) = lock().acquire_slot(Slot::Free) {
        unsafe { allocation_handler() }.on_deallocation(Deallocation {
            heap_handle: HeapHandle as usize,
            base_address: BaseAddress as usize,
            success: success != 0,
        });
    }

    success
}

pub fn is_initialized() -> bool {
    unsafe { INITIAILIZED.load(Ordering::SeqCst) }
}

pub fn is_enabled() -> bool {
    RtlAllocateHeapHook.is_enabled()
}

#[allow(non_snake_case)]
pub unsafe fn initialize() -> Result<(), Error> {
    // Find ntdll
    let ntdll_module = LoadLibraryA(b"ntdll.dll\0".as_ptr() as _);
    if ntdll_module.is_null() {
        return Err(Error::CouldNotFindModule);
    }

    // Find allocate/free procedures
    let rtl_allocate_heap_proc = GetProcAddress(ntdll_module, b"RtlAllocateHeap\0".as_ptr() as _);
    let rtl_free_heap_proc = GetProcAddress(ntdll_module, b"RtlFreeHeap\0".as_ptr() as _);

    // Check for errors
    if rtl_allocate_heap_proc.is_null() || rtl_free_heap_proc.is_null() {
        return Err(Error::CouldNotFindProc);
    }

    // Initialize hooks
    RtlAllocateHeapHook
        .initialize(
            core::mem::transmute(rtl_allocate_heap_proc),
            RtlAllocateHeapDetour,
        )
        .or(Err(Error::HookInitializeFailed))?;
    RtlFreeHeapHook
        .initialize(core::mem::transmute(rtl_free_heap_proc), RtlFreeHeapDetour)
        .or(Err(Error::HookInitializeFailed))?;

    INITIAILIZED.store(true, Ordering::SeqCst);
    Ok(())
}

// SAFETY: `initialize` must be called before this method is called
pub unsafe fn enable() -> Result<(), Error> {
    RtlAllocateHeapHook
        .enable()
        .or(Err(Error::HookEnableFailed))?;
    RtlFreeHeapHook.enable().or(Err(Error::HookEnableFailed))?;
    Ok(())
}

pub unsafe fn disable() -> Result<(), Error> {
    RtlAllocateHeapHook
        .disable()
        .or(Err(Error::HookDisableFailed))?;
    RtlFreeHeapHook
        .disable()
        .or(Err(Error::HookDisableFailed))?;
    Ok(())
}

pub unsafe fn uninitialize() -> Result<(), Error> {
    Ok(())
}
