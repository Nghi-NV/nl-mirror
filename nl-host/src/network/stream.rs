//! Network video streaming module

use crate::log_verbose;
use crossbeam_channel::Sender;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Handle for controlling the video receiver thread
pub struct VideoReceiverHandle {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl VideoReceiverHandle {
    /// Signal the receiver to stop
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Drop for VideoReceiverHandle {
    fn drop(&mut self) {
        self.stop();
        if let Some(handle) = self.handle.take() {
            // Give the thread a moment to finish
            let _ = handle.join();
        }
    }
}

/// Start the video receiver thread that connects to Android and sends data to decoder
pub fn start_video_receiver(
    host: String,
    port: u16,
    bitrate: u32,
    max_size: u32,
    tx: Sender<Vec<u8>>,
) -> VideoReceiverHandle {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    let handle = thread::spawn(move || {
        let mut reconnect_delay = 1;

        while running_clone.load(Ordering::SeqCst) {
            log_verbose!("NET", "Connecting {}:{}...", host, port);
            match TcpStream::connect(format!("{}:{}", host, port)) {
                Ok(mut stream) => {
                    log_verbose!("NET", "Connected");

                    // Handshake: Send config
                    log_verbose!(
                        "NET",
                        "Sending config: bitrate={}, max_size={}",
                        bitrate,
                        max_size
                    );
                    let handshake = format!("bitrate={}&max_size={}\n", bitrate, max_size);
                    if let Err(e) = stream.write_all(handshake.as_bytes()) {
                        log_verbose!("NET", "WARNING: Failed to send handshake: {}", e);
                    }
                    let _ = stream.flush();

                    reconnect_delay = 1;
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                    let _ = stream.set_nodelay(true);

                    if let Err(_) = receive_packets(&mut stream, &tx, &running_clone) {
                        // Connection lost, will reconnect
                    }
                }
                Err(e) => log_verbose!("NET", "Connect failed: {}", e),
            }

            if !running_clone.load(Ordering::SeqCst) {
                break;
            }

            log_verbose!("NET", "Reconnecting in {}s...", reconnect_delay);
            // Sleep in small increments to allow early exit
            for _ in 0..(reconnect_delay * 10) {
                if !running_clone.load(Ordering::SeqCst) {
                    return;
                }
                thread::sleep(std::time::Duration::from_millis(100));
            }
            reconnect_delay = (reconnect_delay * 2).min(10);
        }
        log_verbose!("NET", "Video receiver stopped");
    });

    VideoReceiverHandle {
        running,
        handle: Some(handle),
    }
}

/// Read packets from stream and send to decoder channel
fn receive_packets(
    stream: &mut TcpStream,
    tx: &Sender<Vec<u8>>,
    running: &Arc<AtomicBool>,
) -> Result<(), ()> {
    let mut total = 0u64;
    let start = std::time::Instant::now();
    let mut last_log = std::time::Instant::now();
    let mut consecutive_timeouts = 0;
    let mut header_buf = [0u8; 12];
    let mut read_count = 0u64;

    while running.load(Ordering::SeqCst) {
        read_count += 1;

        // Read 12-byte Header
        match stream.read_exact(&mut header_buf) {
            Ok(()) => {
                let _pts_flags = u64::from_be_bytes(header_buf[0..8].try_into().unwrap());
                let body_size = u32::from_be_bytes(header_buf[8..12].try_into().unwrap()) as usize;

                if body_size > 10 * 1024 * 1024 {
                    log_verbose!("NET", "ERROR: Invalid packet size: {} bytes", body_size);
                    return Err(());
                }

                // Read Body
                let mut body_buf = vec![0u8; body_size];
                match stream.read_exact(&mut body_buf) {
                    Ok(()) => {
                        total += (12 + body_size) as u64;
                        consecutive_timeouts = 0;

                        if read_count % 100 == 0 {
                            log_verbose!(
                                "NET",
                                "Packet #{}: {} bytes, total={}MB",
                                read_count,
                                body_size,
                                total / 1_048_576
                            );
                        }

                        // Send to decoder
                        match tx.try_send(body_buf) {
                            Ok(()) => {}
                            Err(crossbeam_channel::TrySendError::Full(_)) => {
                                log_verbose!("NET", "Channel full, dropping frame");
                            }
                            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                                log_verbose!("NET", "Channel disconnected");
                                return Err(());
                            }
                        }
                    }
                    Err(e) => {
                        log_verbose!("NET", "Failed to read body: {}", e);
                        return Err(());
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock
                {
                    consecutive_timeouts += 1;
                    if consecutive_timeouts > 10 {
                        return Err(());
                    }
                    continue;
                } else if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    log_verbose!("NET", "Server closed connection");
                    return Err(());
                } else {
                    log_verbose!("NET", "Failed to read header: {}", e);
                    return Err(());
                }
            }
        }

        // Stats every 10 seconds
        if last_log.elapsed().as_secs() >= 10 {
            let mbps = (total as f64 * 8.0) / (start.elapsed().as_secs_f64() * 1_000_000.0);
            log_verbose!(
                "NET",
                "Stats: {:.1}MB, {:.2}Mbps, {} packets",
                total as f64 / 1_048_576.0,
                mbps,
                read_count
            );
            last_log = std::time::Instant::now();
        }
    }
    Ok(())
}
