//! Utils module - Utility functions
//!
//! Contains screenshot saving and other helpers.

mod screenshot;

#[allow(unused_imports)]
pub use screenshot::save_screenshot;
pub use screenshot::save_screenshot_yuv;
