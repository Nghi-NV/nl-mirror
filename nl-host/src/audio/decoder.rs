//! Audio decoder
//!
//! Converts raw PCM i16 to f32 for playback.

use super::AudioPacket;
use crossbeam_channel::{Receiver, Sender};
use std::thread::{self, JoinHandle};

/// Start the audio decoder thread (PCM i16 -> f32 conversion)
pub fn start_audio_decoder(rx: Receiver<AudioPacket>, tx: Sender<Vec<f32>>) -> JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(packet) = rx.recv() {
            // Convert PCM i16 (little-endian, Android native) to f32
            let samples_i16: Vec<i16> = packet
                .data
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();

            let samples_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();

            // Send to playback
            if tx.try_send(samples_f32).is_err() {
                // Playback buffer full, drop samples to avoid latency buildup
            }
        }
    })
}
