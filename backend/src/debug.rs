use std::ffi::CString;

use winapi::um::debugapi::OutputDebugStringA;

pub fn debug_message<T: Into<Vec<u8>>>(s: T) {
    unsafe {
        let s = CString::new(s).unwrap();
        OutputDebugStringA(s.as_ptr() as _);
    }
}
