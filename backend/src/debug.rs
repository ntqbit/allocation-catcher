pub use crate::platform::debug_message_fmt;

#[allow(unused_macros)]
macro_rules! debug_message {
    ($($arg:tt)*) => {
        $crate::debug::debug_message_fmt(format_args!($($arg)*))
    };
}

#[allow(unused_imports)]
pub(crate) use debug_message;
