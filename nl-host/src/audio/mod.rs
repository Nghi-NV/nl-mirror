//! Audio streaming module
//!
//! Receives OPUS-encoded audio from Android device and plays via cpal.

mod decoder;
mod playback;
mod receiver;

pub use decoder::start_audio_decoder;
pub use playback::start_audio_playback;
pub use receiver::start_audio_receiver;

use crossbeam_channel::bounded;

/// Start the complete audio pipeline
pub fn start_audio_pipeline(host: String, port: u16) {
    // Receiver -> Decoder channel (encoded packets)
    let (encoded_tx, encoded_rx) = bounded::<AudioPacket>(64);

    // Decoder -> Playback channel (PCM samples)
    let (pcm_tx, pcm_rx) = bounded::<Vec<f32>>(64);

    // Start pipeline threads
    start_audio_receiver(host, port, encoded_tx);
    start_audio_decoder(encoded_rx, pcm_tx);
    start_audio_playback(pcm_rx);
}

/// Encoded audio packet from network
#[derive(Clone)]
pub struct AudioPacket {
    pub pts: u64,
    pub data: Vec<u8>,
}

/// Audio stream header
pub struct AudioHeader {
    pub sample_rate: u32,
    pub channels: u8,
    pub codec_type: u8, // 1 = OPUS
}
