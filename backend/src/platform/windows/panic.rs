use std::{panic::PanicInfo, ffi::CString};

use winapi::um::winuser::MessageBoxA;

pub fn handle_panic(panic_info: &PanicInfo) {
    if let Ok(cstring) = CString::new(panic_info.to_string()) {
        unsafe {
            MessageBoxA(0 as _, cstring.as_ptr() as _, b"PANIC\0".as_ptr() as _, 0);
        }
    } else {
        unsafe {
            MessageBoxA(
                0 as _,
                b"panic info contained a nul byte\0".as_ptr() as _,
                b"PANIC\0".as_ptr() as _,
                0,
            );
        }
    }
}
