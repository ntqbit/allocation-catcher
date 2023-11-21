use winapi::{
    shared::minwindef::{BOOL, HINSTANCE},
    um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
};

use super::panic::handle_panic;

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _module: HINSTANCE,
    reason: u32,
    _reserved: *mut winapi::ctypes::c_void,
) -> BOOL {
    match reason {
        DLL_PROCESS_ATTACH => {
            std::panic::set_hook(Box::new(|panic_info| handle_panic(panic_info)));
            allocation_catcher_backend::initialize()
        }
        DLL_PROCESS_DETACH => allocation_catcher_backend::deinitialize(),
        _ => {}
    }

    1
}
