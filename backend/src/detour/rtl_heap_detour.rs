use retour::static_detour;
use winapi::{
    shared::{
        basetsd::SIZE_T,
        minwindef::BOOL,
        ntdef::{PVOID, ULONG},
    },
    um::{
        libloaderapi::{GetProcAddress, LoadLibraryA},
        winnt::HEAP_NO_SERIALIZE,
    },
};

use super::{
    allocation_handler, flag_set, Allocation, Base, Deallocation, DetourFlag, Error, Reallocation,
};

static_detour! {
    static RtlAllocateHeapHook: extern "system" fn(
        PVOID,
        ULONG,
        SIZE_T
    ) -> PVOID;

    static RtlReAllocateHeapHook: extern "system" fn(
        PVOID,
        ULONG,
        PVOID,
        SIZE_T
    ) -> PVOID;

    static RtlFreeHeapHook: extern "system" fn(
        PVOID,
        ULONG,
        PVOID
    ) -> BOOL;
}

#[inline(always)]
fn handle_detour<T: Copy>(
    flags: ULONG,
    forward: impl FnOnce(ULONG) -> T,
    handle: impl FnOnce(T),
) -> T {
    // Do not handle all the recursive calls to detour functions.
    let recursion_lock = flag_set().acquire(DetourFlag::Lock);

    // Call original function.
    let result = forward(flags);

    // Handle only non-recursive calls and calls without HEAP_NO_SERIALIZE flag set.
    // If flag HEAP_NO_SERIALIZE is set, then it is possible that the heap is already locked.
    // Handling this call may lock a mutex that may be already locked by antoher thread
    // waiting for heap to be unlocked. Deadlock.
    if recursion_lock.is_some() && (flags & HEAP_NO_SERIALIZE) == 0 {
        handle(result);
    }

    result
}

extern "C" {
    #[link_name = "llvm.returnaddress"]
    fn return_address(a: i32) -> *const u8;

    #[link_name = "llvm.addressofreturnaddress"]
    fn addressofreturnaddress() -> *const u8;

    #[link_name = "llvm.frameaddress"]
    fn frame_address(a: i32) -> *const u8;
}

macro_rules! heap_base {
    ($heap_handle:ident) => {
        Base {
            heap_handle: $heap_handle as usize,
            return_address: unsafe { Some(return_address(0) as usize) },
            address_of_return_address: unsafe { Some(addressofreturnaddress() as usize) },
            stack_frame_address: unsafe { Some(frame_address(0) as usize) },
        }
    };
}

#[allow(non_snake_case)]
fn RtlAllocateHeapDetour(HeapHandle: PVOID, Flags: ULONG, Size: SIZE_T) -> PVOID {
    let base = heap_base!(HeapHandle);

    handle_detour(
        Flags,
        |flags| RtlAllocateHeapHook.call(HeapHandle, flags, Size),
        |base_address| {
            unsafe { allocation_handler() }.on_allocation(Allocation {
                base,
                size: Size as usize,
                allocated_base_address: if base_address.is_null() {
                    None
                } else {
                    Some(base_address as usize)
                },
            });
        },
    )
}

#[allow(non_snake_case)]
fn RtlReAllocateHeapDetour(
    HeapHandle: PVOID,
    Flags: ULONG,
    BaseAddress: PVOID,
    Size: SIZE_T,
) -> PVOID {
    let base = heap_base!(HeapHandle);

    handle_detour(
        Flags,
        |flags| RtlReAllocateHeapHook.call(HeapHandle, flags, BaseAddress, Size),
        |base_address| {
            unsafe { allocation_handler() }.on_reallocation(Reallocation {
                base_address: BaseAddress as usize,
                allocation: Allocation {
                    base,
                    size: Size as usize,
                    allocated_base_address: if base_address.is_null() {
                        None
                    } else {
                        Some(base_address as usize)
                    },
                },
            });
        },
    )
}

#[allow(non_snake_case)]
fn RtlFreeHeapDetour(HeapHandle: PVOID, Flags: ULONG, BaseAddress: PVOID) -> BOOL {
    let base = heap_base!(HeapHandle);

    handle_detour(
        Flags,
        |flags| RtlFreeHeapHook.call(HeapHandle, flags, BaseAddress),
        |success| {
            unsafe { allocation_handler() }.on_deallocation(Deallocation {
                base,
                base_address: BaseAddress as usize,
                success: success != 0,
            });
        },
    )
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
    let rtl_reallocate_heap_proc =
        GetProcAddress(ntdll_module, b"RtlReAllocateHeap\0".as_ptr() as _);
    let rtl_free_heap_proc = GetProcAddress(ntdll_module, b"RtlFreeHeap\0".as_ptr() as _);

    // Check for errors
    if rtl_allocate_heap_proc.is_null()
        || rtl_reallocate_heap_proc.is_null()
        || rtl_free_heap_proc.is_null()
    {
        return Err(Error::CouldNotFindProc);
    }

    // Initialize hooks
    RtlAllocateHeapHook
        .initialize(
            core::mem::transmute(rtl_allocate_heap_proc),
            RtlAllocateHeapDetour,
        )
        .and_then(|_| {
            RtlReAllocateHeapHook.initialize(
                core::mem::transmute(rtl_reallocate_heap_proc),
                RtlReAllocateHeapDetour,
            )
        })
        .and_then(|_| {
            RtlFreeHeapHook.initialize(core::mem::transmute(rtl_free_heap_proc), RtlFreeHeapDetour)
        })
        .map_err(|_| Error::HookInitializeFailed)?;

    Ok(())
}

// SAFETY: `initialize` must be called before this method is called
pub unsafe fn enable() -> Result<(), Error> {
    Ok(())
        .and_then(|_| RtlAllocateHeapHook.enable())
        .and_then(|_| RtlReAllocateHeapHook.enable())
        .and_then(|_| RtlFreeHeapHook.enable())
        .map_err(|_| Error::HookEnableFailed)
}

pub unsafe fn disable() -> Result<(), Error> {
    Ok(())
        .and_then(|_| RtlAllocateHeapHook.disable())
        .and_then(|_| RtlReAllocateHeapHook.disable())
        .and_then(|_| RtlFreeHeapHook.disable())
        .map_err(|_| Error::HookDisableFailed)
}

pub unsafe fn uninitialize() -> Result<(), Error> {
    Ok(())
}
