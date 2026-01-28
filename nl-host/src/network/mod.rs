//! Network module - Communication with Android device

pub mod binary_control;
mod control;
pub mod stream;

pub use binary_control::BinaryControlClient;
pub use control::ControlClient;
pub use stream::{start_video_receiver, VideoReceiverHandle};
