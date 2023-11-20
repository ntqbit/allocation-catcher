use std::{ffi::CStr, fmt::Write};

use winapi::um::{debugapi::OutputDebugStringA, processthreadsapi::GetCurrentThreadId};

pub fn debug_message_fmt(args: core::fmt::Arguments) {
    unsafe {
        let mut formatted = heapless::String::<4096>::new();

        write!(formatted, "[T:{}] {}\0", GetCurrentThreadId(), args)
            .expect("not enough buffer size for formatting debug message");

        OutputDebugStringA(
            CStr::from_bytes_with_nul(formatted.as_bytes())
                .expect("could not convert debug message into CStr")
                .as_ptr(),
        );
    }
}

macro_rules! debug_message {
    ($($arg:tt)*) => {
        $crate::debug::debug_message_fmt(format_args!($($arg)*))
    };
}

pub(crate) use debug_message;
