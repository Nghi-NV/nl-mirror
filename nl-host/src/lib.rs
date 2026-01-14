//! NL-Host Library
//!
//! A library for high-speed Android mirroring and control.

#[macro_use]
pub mod core;
pub mod audio;
pub mod input;
pub mod network;
pub mod utils;
pub mod video;

// Re-export commonly used items
pub use core::run;
pub use network::ControlClient;
