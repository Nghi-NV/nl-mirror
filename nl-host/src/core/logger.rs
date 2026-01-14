//! Configurable logging macros for nl-host
//!
//! Logs only print when corresponding config flag is enabled.

/// Debug log - only prints when debug mode is enabled
#[macro_export]
macro_rules! log_debug {
    ($tag:expr, $($arg:tt)*) => {
        if $crate::core::is_debug() {
            eprintln!("[DEBUG][{}] {}", $tag, format!($($arg)*));
        }
    };
}

/// Verbose log - only prints when verbose mode is enabled
#[macro_export]
macro_rules! log_verbose {
    ($tag:expr, $($arg:tt)*) => {
        if $crate::core::is_verbose() {
            eprintln!("[{}] {}", $tag, format!($($arg)*));
        }
    };
}

/// Info log - always prints
#[macro_export]
macro_rules! log_info {
    ($tag:expr, $($arg:tt)*) => {
        eprintln!("[{}] {}", $tag, format!($($arg)*));
    };
}

/// Warning log - always prints with WARN prefix
#[macro_export]
macro_rules! log_warn {
    ($tag:expr, $($arg:tt)*) => {
        eprintln!("[WARN][{}] {}", $tag, format!($($arg)*));
    };
}

/// Error log - always prints with ERROR prefix
#[macro_export]
macro_rules! log_error {
    ($tag:expr, $($arg:tt)*) => {
        eprintln!("[ERROR][{}] {}", $tag, format!($($arg)*));
    };
}
