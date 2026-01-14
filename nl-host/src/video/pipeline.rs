//! Video decoding pipeline

use crate::core::{FrameBuffer, FrameData};
use crate::video::{VideoDecoder, YuvFrame};
use crate::{log_error, log_verbose};
use crossbeam_channel::Receiver;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Start the decoder thread that processes H264 data and produces frames
pub fn start_decoder_thread(
    rx: Receiver<Vec<u8>>,
    frame_buffer: Arc<FrameBuffer>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut decoder = match VideoDecoder::new() {
            Ok(d) => d,
            Err(e) => {
                log_error!("DEC", "Init failed: {}", e);
                return;
            }
        };

        let mut frame_count = 0u64;
        let mut recv_count = 0u64;
        let start = std::time::Instant::now();
        let mut last_log = std::time::Instant::now();
        let mut last_heartbeat = std::time::Instant::now();

        loop {
            // Heartbeat every 5 seconds
            if last_heartbeat.elapsed().as_secs() >= 5 {
                log_verbose!("DEC", "Heartbeat: waiting for data...");
                last_heartbeat = std::time::Instant::now();
            }

            match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(data) => {
                    recv_count += 1;
                    let data_size = data.len();

                    if recv_count % 100 == 0 {
                        log_verbose!("DEC", "Recv #{}: {} bytes", recv_count, data_size);
                    }

                    match decoder.decode(&data) {
                        Ok(frames) => {
                            process_decoded_frames(
                                frames,
                                &frame_buffer,
                                &mut frame_count,
                                recv_count,
                                data_size,
                                &start,
                            );
                        }
                        Err(e) => {
                            if recv_count % 50 == 0 {
                                log_verbose!("DEC", "Error: {}", e);
                            }
                        }
                    }

                    // Stats every 10 seconds
                    if last_log.elapsed().as_secs() >= 10 {
                        let fps = frame_count as f64 / start.elapsed().as_secs_f64();
                        log_verbose!("DEC", "{} frames, {:.1} fps avg", frame_count, fps);
                        last_log = std::time::Instant::now();
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    log_verbose!("DEC", "Channel disconnected");
                    break;
                }
            }
        }
    })
}

fn process_decoded_frames(
    frames: Vec<YuvFrame>,
    frame_buffer: &Arc<FrameBuffer>,
    frame_count: &mut u64,
    recv_count: u64,
    data_size: usize,
    start: &std::time::Instant,
) {
    let frames_count = frames.len();
    let mut frames_stored = 0;

    for yuv in frames {
        *frame_count += 1;
        frames_stored += 1;

        let frame = FrameData {
            width: yuv.width,
            height: yuv.height,
            y_plane: Arc::new(yuv.y_plane),
            u_plane: Arc::new(yuv.u_plane),
            v_plane: Arc::new(yuv.v_plane),
            y_stride: yuv.y_stride,
            uv_stride: yuv.uv_stride,
        };

        let skipped = frame_buffer.push(frame);
        if skipped && *frame_count % 100 == 0 {
            log_verbose!("DEC", "Frame skipped at #{}", *frame_count);
        }

        // Progress log every 600 frames (~10s at 60fps)
        if *frame_count % 600 == 0 {
            let elapsed = start.elapsed().as_secs();
            log_verbose!("DEC", "Frame #{} at {}s", *frame_count, elapsed);
        }
    }

    if frames_stored == 0 && recv_count % 50 == 0 {
        log_verbose!("DEC", "No frames from {} bytes", data_size);
    } else if recv_count % 200 == 0 && frames_count > 0 {
        log_verbose!("DEC", "{} bytes -> {} frames", data_size, frames_stored);
    }
}
