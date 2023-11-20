use std::ffi::CString;

use winapi::um::debugapi::OutputDebugStringA;

pub fn debug_message<T: Into<Vec<u8>>>(s: T) {
    unsafe {
        let s = CString::new(s).expect("debug message must not contain a nul byte");
        OutputDebugStringA(s.as_ptr() as _);
    }
}
