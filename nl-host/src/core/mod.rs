//! Core module - Application foundation

mod app;
mod config;
mod frame;
#[macro_use]
pub mod logger;

pub use app::run;
pub use config::{is_debug, is_verbose, VERBOSE};
pub use frame::{FrameBuffer, FrameData};
