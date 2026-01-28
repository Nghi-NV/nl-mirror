//! Binary input command processing
//!
//! Uses low-latency binary protocol instead of JSON for faster touch events.

use crate::network::BinaryControlClient;
use crossbeam_channel::Receiver;
use std::thread::{self, JoinHandle};

/// Input commands sent from UI to background thread
#[derive(Debug)]
pub enum BinaryInputCommand {
    TouchDown(f32, f32),
    TouchMove(f32, f32),
    TouchUp(f32, f32),
    Tap(f32, f32),
    Swipe(f32, f32, f32, f32, u64),
    LongPress(f32, f32, u64),
    PressKey(i32),
    InjectText(String),
    SetClipboard(String, bool),
    SetScreenPower(bool),
}

/// Start the binary input handler thread
pub fn start_binary_input_thread(
    host: String,
    port: u16,
    rx: Receiver<BinaryInputCommand>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut client = {
            let mut delay_ms = 100u64;
            loop {
                match BinaryControlClient::connect(&host, port) {
                    Ok(c) => {
                        println!("[BINARY_INPUT] Connected to {}:{}", host, port);
                        break c;
                    }
                    Err(e) => {
                        if delay_ms < 2000 {
                            eprintln!(
                                "[BINARY_INPUT] Connect failed: {}, retrying in {}ms...",
                                e, delay_ms
                            );
                        }
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        delay_ms = (delay_ms * 2).min(2000);
                    }
                }
            }
        };

        while let Ok(cmd) = rx.recv() {
            process_binary_command(&mut client, cmd);
        }
        println!("[BINARY_INPUT] Thread exiting");
    })
}

fn process_binary_command(client: &mut BinaryControlClient, cmd: BinaryInputCommand) {
    match cmd {
        BinaryInputCommand::TouchDown(x, y) => {
            let _ = client.touch_down(x, y);
        }
        BinaryInputCommand::TouchMove(x, y) => {
            let _ = client.touch_move(x, y);
        }
        BinaryInputCommand::TouchUp(x, y) => {
            let _ = client.touch_up(x, y);
        }
        BinaryInputCommand::Tap(x, y) => {
            let _ = client.tap(x, y);
        }
        BinaryInputCommand::Swipe(x1, y1, x2, y2, duration) => {
            let _ = client.swipe(x1, y1, x2, y2, duration);
        }
        BinaryInputCommand::LongPress(x, y, duration) => {
            let _ = client.long_press(x, y, duration);
        }
        BinaryInputCommand::PressKey(keycode) => {
            let _ = client.press_key(keycode);
        }
        BinaryInputCommand::InjectText(text) => {
            let _ = client.inject_text(&text);
        }
        BinaryInputCommand::SetClipboard(text, paste) => {
            let _ = client.set_clipboard(&text, paste);
        }
        BinaryInputCommand::SetScreenPower(on) => {
            let _ = client.set_screen_power(on);
        }
    }
}
