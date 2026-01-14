//! H264 Video Decoder using OpenH264
//! Cross-platform decoder that works on macOS/Windows/Linux without system dependencies

use crate::core;
use anyhow::{anyhow, Result};
use openh264::decoder::Decoder;
use openh264::formats::YUVSource;
use std::time::Instant;

// Local logging helper - only prints when verbose/debug is enabled
macro_rules! dec_log {
    ($($arg:tt)*) => {
        if core::is_verbose() || core::is_debug() {
            eprintln!($($arg)*);
        }
    };
}

// Always log (errors, important events)
#[allow(unused_macros)]
macro_rules! dec_log_always {
    ($($arg:tt)*) => {
        eprintln!($($arg)*);
    };
}

const START_CODE: &[u8] = &[0, 0, 0, 1];
const MAX_BUFFER_SIZE: usize = 4 * 1024 * 1024; // 4MB max buffer

/// Check if NAL unit is a keyframe (IDR or SPS)
fn is_keyframe(nal_data: &[u8]) -> bool {
    if nal_data.len() > 4 {
        let nal_type = nal_data[4] & 0x1F;
        return nal_type == 5 || nal_type == 7 || nal_type == 8;
    }
    false
}

/// Decoded YUV frame data for GPU upload
pub struct YuvFrame {
    pub width: u32,
    pub height: u32,
    pub y_plane: Vec<u8>,
    pub u_plane: Vec<u8>,
    pub v_plane: Vec<u8>,
    pub y_stride: usize,
    pub uv_stride: usize,
}

pub struct VideoDecoder {
    decoder: Decoder,
    buffer: Vec<u8>,
    packet_count: u64,
    frame_count: u64,
    last_log: Instant,
    last_frame_time: Instant,
    waiting_for_keyframe: bool,
}

impl VideoDecoder {
    pub fn new() -> Result<Self> {
        let decoder = Decoder::new().map_err(|e| anyhow!("OpenH264 init failed: {:?}", e))?;

        let now = Instant::now();
        Ok(Self {
            decoder,
            buffer: Vec::with_capacity(256 * 1024),
            packet_count: 0,
            frame_count: 0,
            last_log: now,
            last_frame_time: now,
            waiting_for_keyframe: false,
        })
    }

    /// Reset decoder to recover from errors
    fn reset_decoder(&mut self) -> Result<()> {
        dec_log!("[DEC] Resetting decoder...");

        // Recreate decoder from scratch
        self.decoder = Decoder::new().map_err(|e| anyhow!("OpenH264 reset failed: {:?}", e))?;

        // Set flag to wait for keyframe after reset
        self.waiting_for_keyframe = true;

        dec_log!("[DEC] Decoder reset complete, waiting for keyframe...");
        Ok(())
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<Vec<YuvFrame>> {
        let input_size = data.len();
        self.buffer.extend_from_slice(data);
        let buffer_size_before = self.buffer.len();

        // Periodic status logging
        if self.last_log.elapsed().as_secs() >= 5 {
            let stall_ms = self.last_frame_time.elapsed().as_millis();
            dec_log!(
                "[DEC-STATS] buffer={}KB, packets={}, frames={}, last_frame_ms_ago={}, waiting_keyframe={}",
                self.buffer.len() / 1024,
                self.packet_count,
                self.frame_count,
                stall_ms,
                self.waiting_for_keyframe
            );
            self.last_log = Instant::now();

            // Watchdog: if no frame for 2 seconds, reset decoder
            if stall_ms > 2000 {
                dec_log!(
                    "[DEC] WATCHDOG: No frames for {}ms, resetting decoder",
                    stall_ms
                );
                let _ = self.reset_decoder();
                self.last_frame_time = Instant::now();
                return Ok(Vec::new());
            }
        }

        // Prevent buffer overflow
        if self.buffer.len() > MAX_BUFFER_SIZE {
            println!(
                "Buffer overflow ({}KB), resetting decoder",
                self.buffer.len() / 1024
            );
            let _ = self.reset_decoder();
            let keep_start = self.buffer.len().saturating_sub(256 * 1024);
            self.buffer.drain(0..keep_start);
            self.last_frame_time = Instant::now();
            return Ok(Vec::new());
        }

        let mut decoded_frames = Vec::new();

        loop {
            // Find start code
            let start_pos = match self.buffer.windows(4).position(|w| w == START_CODE) {
                Some(pos) => pos,
                None => {
                    if self.buffer.len() > 3 {
                        let keep = self.buffer.len() - 3;
                        self.buffer.drain(0..keep);
                    }
                    break;
                }
            };

            // Discard garbage before start code
            if start_pos > 0 {
                self.buffer.drain(0..start_pos);
                continue;
            }

            // Find next start code (end of this NAL unit)
            let next_start = self.buffer[4..]
                .windows(4)
                .position(|w| w == START_CODE)
                .map(|p| p + 4);

            match next_start {
                Some(end_pos) => {
                    // Extract complete NAL unit
                    let nal_unit: Vec<u8> = self.buffer.drain(0..end_pos).collect();
                    self.packet_count += 1;

                    // If waiting for keyframe, skip non-keyframes
                    if self.waiting_for_keyframe {
                        if is_keyframe(&nal_unit) {
                            dec_log!(
                                "[DEC] Found keyframe (packet {}), resuming decode",
                                self.packet_count
                            );
                            self.waiting_for_keyframe = false;
                        } else {
                            if self.packet_count % 100 == 0 {
                                dec_log!(
                                    "[DEC] Skipping non-keyframe NAL (packet {}), waiting for keyframe",
                                    self.packet_count
                                );
                            }
                            continue;
                        }
                    }

                    // Decode this NAL unit
                    let nal_size = nal_unit.len();
                    let frames_before = decoded_frames.len();
                    if let Err(e) = self.decode_nal(&nal_unit, &mut decoded_frames) {
                        if self.packet_count % 50 == 0 {
                            dec_log!(
                                "[DEC] ERROR: Decode NAL error (packet {}, size={}): {}, resetting decoder",
                                self.packet_count, nal_size, e
                            );
                            let _ = self.reset_decoder();
                            self.last_frame_time = Instant::now();
                            break;
                        } else if self.packet_count % 10 == 0 {
                            dec_log!(
                                "[DEC] Decode NAL error (packet {}): {}",
                                self.packet_count,
                                e
                            );
                        }
                    } else {
                        let frames_after = decoded_frames.len();
                        if frames_after > frames_before && self.packet_count % 200 == 0 {
                            dec_log!(
                                "[DEC] Decoded NAL #{} ({} bytes) -> {} frames",
                                self.packet_count,
                                nal_size,
                                frames_after - frames_before
                            );
                        }
                    }
                }
                None => {
                    break;
                }
            }
        }

        // Update last frame time if we produced frames
        let frames_produced = decoded_frames.len();
        if frames_produced > 0 {
            self.last_frame_time = Instant::now();
            if self.packet_count % 300 == 0 {
                dec_log!(
                    "[DEC] Produced {} frames from {} bytes input (buffer: {}KB -> {}KB)",
                    frames_produced,
                    input_size,
                    buffer_size_before / 1024,
                    self.buffer.len() / 1024
                );
            }
        } else if input_size > 0 && self.packet_count % 100 == 0 {
            dec_log!(
                "[DEC] WARNING: No frames produced from {} bytes input (buffer: {}KB, packets={})",
                input_size,
                self.buffer.len() / 1024,
                self.packet_count
            );
        }

        Ok(decoded_frames)
    }

    fn decode_nal(&mut self, nal_data: &[u8], frames_out: &mut Vec<YuvFrame>) -> Result<()> {
        // Decode NAL unit with OpenH264
        match self.decoder.decode(nal_data) {
            Ok(Some(yuv)) => {
                self.frame_count += 1;

                let (width, height) = yuv.dimensions();
                let width = width as u32;
                let height = height as u32;

                // Get YUV planes and strides
                let y_plane = yuv.y();
                let u_plane = yuv.u();
                let v_plane = yuv.v();
                let (y_stride, u_stride, _v_stride) = yuv.strides();

                // Copy planes (stride-aware) - GPU will handle conversion
                let h = height as usize;
                let w = width as usize;
                let uv_h = h / 2;
                let uv_w = w / 2;

                // Copy Y plane (packed, no stride padding)
                let mut y_packed = Vec::with_capacity(w * h);
                for row in 0..h {
                    y_packed.extend_from_slice(&y_plane[row * y_stride..row * y_stride + w]);
                }

                // Copy U plane (packed)
                let mut u_packed = Vec::with_capacity(uv_w * uv_h);
                for row in 0..uv_h {
                    u_packed.extend_from_slice(&u_plane[row * u_stride..row * u_stride + uv_w]);
                }

                // Copy V plane (packed)
                let mut v_packed = Vec::with_capacity(uv_w * uv_h);
                for row in 0..uv_h {
                    v_packed.extend_from_slice(&v_plane[row * u_stride..row * u_stride + uv_w]);
                }

                frames_out.push(YuvFrame {
                    width,
                    height,
                    y_plane: y_packed,
                    u_plane: u_packed,
                    v_plane: v_packed,
                    y_stride: w,
                    uv_stride: uv_w,
                });
            }
            Ok(None) => {
                // No frame produced yet (need more NAL units)
            }
            Err(e) => {
                return Err(anyhow!("OpenH264 decode failed: {:?}", e));
            }
        }

        Ok(())
    }
}
