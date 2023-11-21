pub use crate::platform::debug_message_fmt;

macro_rules! debug_message {
    ($($arg:tt)*) => {
        $crate::debug::debug_message_fmt(format_args!($($arg)*))
    };
}

pub(crate) use debug_message;
