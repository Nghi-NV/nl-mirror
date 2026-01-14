//! Network module - Communication with Android device

mod control;
pub mod stream;

pub use control::ControlClient;
pub use stream::start_video_receiver;
