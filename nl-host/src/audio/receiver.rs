//! Audio stream receiver
//!
//! Connects to device and receives encoded audio packets.

use super::{AudioHeader, AudioPacket};
use crossbeam_channel::Sender;
use std::io::Read;
use std::net::TcpStream;
use std::thread::{self, JoinHandle};

/// Start the audio receiver thread
pub fn start_audio_receiver(host: String, port: u16, tx: Sender<AudioPacket>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut reconnect_delay = 1;

        loop {
            match TcpStream::connect(format!("{}:{}", host, port)) {
                Ok(mut stream) => {
                    // Read header first
                    match read_header(&mut stream) {
                        Ok(_) => {
                            reconnect_delay = 1;

                            // Receive loop
                            let _ = receive_packets(&mut stream, &tx);
                        }
                        Err(e) => {
                            log_error!("AUDIO", "Failed to read header: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log_verbose!("AUDIO", "Connect failed: {}", e);
                }
            }

            thread::sleep(std::time::Duration::from_secs(reconnect_delay));
            reconnect_delay = (reconnect_delay * 2).min(10);
        }
    })
}

fn read_header(stream: &mut TcpStream) -> std::io::Result<AudioHeader> {
    let mut header_buf = [0u8; 12];
    stream.read_exact(&mut header_buf)?;

    // Verify magic "AUDIO\0"
    if &header_buf[0..6] != b"AUDIO\0" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid audio header magic",
        ));
    }

    let sample_rate =
        u32::from_be_bytes([header_buf[6], header_buf[7], header_buf[8], header_buf[9]]);
    let channels = header_buf[10];
    let codec_type = header_buf[11];

    Ok(AudioHeader {
        sample_rate,
        channels,
        codec_type,
    })
}

fn receive_packets(stream: &mut TcpStream, tx: &Sender<AudioPacket>) -> Result<(), ()> {
    let mut header_buf = [0u8; 12];

    loop {
        // Read packet header: [PTS(8)][Size(4)]
        if stream.read_exact(&mut header_buf).is_err() {
            return Err(());
        }

        let pts = u64::from_be_bytes(header_buf[0..8].try_into().unwrap());
        let size = u32::from_be_bytes(header_buf[8..12].try_into().unwrap()) as usize;

        if size > 1024 * 1024 {
            // Sanity check
            return Err(());
        }

        // Read packet data
        let mut data = vec![0u8; size];
        if stream.read_exact(&mut data).is_err() {
            return Err(());
        }

        // Send to decoder
        if tx.try_send(AudioPacket { pts, data }).is_err() {
            // Channel full, drop packet
        }
    }
}
