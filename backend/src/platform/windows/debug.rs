use std::{ffi::CStr, fmt::Write};

use winapi::um::{debugapi::OutputDebugStringA, processthreadsapi::GetCurrentThreadId};

#[allow(dead_code)]
pub fn debug_message_fmt(args: core::fmt::Arguments) {
    let thread_id = unsafe { GetCurrentThreadId() };

    let mut formatted = heapless::String::<4096>::new();
    write!(formatted, "[T:{}] {}\0", thread_id, args)
        .expect("not enough buffer size for formatting debug message");

    let str_ptr = CStr::from_bytes_with_nul(formatted.as_bytes())
        .expect("could not convert debug message into CStr")
        .as_ptr();

    unsafe {
        OutputDebugStringA(str_ptr);
    }
}
