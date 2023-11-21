use winapi::{
    shared::minwindef::{BOOL, HINSTANCE},
    um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
};

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _module: HINSTANCE,
    reason: u32,
    _reserved: *mut winapi::ctypes::c_void,
) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => crate::initialize(),
        DLL_PROCESS_DETACH => crate::deinitialize(),
        _ => {}
    }

    1
}
