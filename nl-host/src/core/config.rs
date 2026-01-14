//! Global configuration for nl-host

use std::sync::atomic::{AtomicBool, Ordering};

/// Global configuration flags
pub static VERBOSE: AtomicBool = AtomicBool::new(false);
pub static DEBUG: AtomicBool = AtomicBool::new(false);

/// Check if verbose logging is enabled
#[inline]
pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// Check if debug logging is enabled
#[inline]
pub fn is_debug() -> bool {
    DEBUG.load(Ordering::Relaxed)
}
