use std::{ffi::CString, panic::PanicInfo};

use winapi::um::winuser::MessageBoxA;

pub fn handle_panic(panic_info: &PanicInfo) {
    let res = CString::new(panic_info.to_string());

    let str = if let Ok(cstring) = res.as_ref() {
        cstring.as_ptr()
    } else {
        b"panic info contained a nul byte\0".as_ptr() as _
    };

    unsafe {
        MessageBoxA(0 as _, str, b"Panic\0".as_ptr() as _, 0);
    }
}
