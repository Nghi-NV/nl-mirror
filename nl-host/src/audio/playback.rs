//! Audio playback
//!
//! Plays PCM audio samples using cpal.

use crate::log_error;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use crossbeam_channel::Receiver;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 2;
const BUFFER_SIZE: usize = 4096; // Samples to buffer before starting playback

/// Start the audio playback thread
pub fn start_audio_playback(rx: Receiver<Vec<f32>>) -> JoinHandle<()> {
    thread::spawn(move || {
        // Get default audio output device
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                log_error!("AUDIO", "No audio output device found");
                return;
            }
        };

        // Configure stream
        let config = StreamConfig {
            channels: CHANNELS,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        // Shared buffer for samples
        let buffer: Arc<Mutex<VecDeque<f32>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(BUFFER_SIZE * 4)));
        let buffer_clone = buffer.clone();

        // Build output stream
        let stream = match device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buf = buffer_clone.lock().unwrap();
                for sample in data.iter_mut() {
                    *sample = buf.pop_front().unwrap_or(0.0);
                }
            },
            |err| {
                log_error!("AUDIO", "Stream error: {}", err);
            },
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                log_error!("AUDIO", "Failed to build output stream: {}", e);
                return;
            }
        };

        // Start playback
        if let Err(e) = stream.play() {
            log_error!("AUDIO", "Failed to start playback: {}", e);
            return;
        }

        // Feed samples to buffer
        while let Ok(samples) = rx.recv() {
            let mut buf = buffer.lock().unwrap();
            for sample in samples {
                buf.push_back(sample);
            }
            // Prevent buffer from growing too large (drop old samples)
            while buf.len() > BUFFER_SIZE * 8 {
                buf.pop_front();
            }
        }
    })
}
