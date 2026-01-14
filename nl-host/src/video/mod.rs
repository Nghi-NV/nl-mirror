//! Video module - Decoding and rendering pipeline

mod decoder;
pub mod pipeline;
mod renderer;

pub use decoder::{VideoDecoder, YuvFrame};
pub use pipeline::start_decoder_thread;
pub use renderer::MirrorRenderer;
