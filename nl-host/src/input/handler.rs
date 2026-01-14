//! Input command processing

use crate::network::ControlClient;
use crate::{log_error, log_verbose};
use crossbeam_channel::Receiver;
use std::thread::{self, JoinHandle};

/// Input commands sent from UI to background thread
#[derive(Debug)]
pub enum InputCommand {
    Tap(f32, f32),
    Swipe(f32, f32, f32, f32, u64),
    LongPress(f32, f32, u64),
    Keycode(String, i32, i32),
    GetClipboard(bool),
    SetClipboard(String, bool),
    InjectText(String), // type text directly
    SetScreenPowerMode(i32),
}

/// Start the input handler thread that processes commands non-blocking
pub fn start_input_thread(host: String, port: u16, rx: Receiver<InputCommand>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut client = {
            let mut delay_ms = 500u64;
            loop {
                match ControlClient::connect(&host, port) {
                    Ok(c) => {
                        log_verbose!("INPUT", "Connected to {}:{}", host, port);
                        break c;
                    }
                    Err(e) => {
                        log_verbose!(
                            "INPUT",
                            "Connect to {}:{} failed: {}, retrying in {}ms...",
                            host,
                            port,
                            e,
                            delay_ms
                        );
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        // Exponential backoff: 500ms -> 1s -> 2s -> 4s -> 5s (max)
                        delay_ms = (delay_ms * 2).min(5000);
                    }
                }
            }
        };

        // Increase timeout for control commands (clipboard can be slow)
        if let Err(e) = client.set_timeout(std::time::Duration::from_millis(1000)) {
            log_error!("INPUT", "Failed to set timeout: {}", e);
        }

        while let Ok(cmd) = rx.recv() {
            process_command(&mut client, cmd);
        }
        log_verbose!("INPUT", "Thread exiting");
    })
}

fn process_command(client: &mut ControlClient, cmd: InputCommand) {
    match cmd {
        InputCommand::Tap(x, y) => {
            if let Err(e) = client.tap(x, y) {
                log_verbose!("INPUT", "Tap failed: {}", e);
            }
        }
        InputCommand::Swipe(x1, y1, x2, y2, duration) => {
            if let Err(e) = client.swipe(x1, y1, x2, y2, duration) {
                log_verbose!("INPUT", "Swipe failed: {}", e);
            }
        }
        InputCommand::LongPress(x, y, duration) => {
            if let Err(e) = client.long_press(x, y, duration) {
                log_verbose!("INPUT", "Long press failed: {}", e);
            }
        }
        InputCommand::Keycode(action, keycode, meta) => {
            if let Err(e) = client.inject_keycode(&action, keycode, meta) {
                log_verbose!("INPUT", "Keycode failed: {}", e);
            }
        }
        InputCommand::GetClipboard(copy) => {
            match client.get_clipboard(copy) {
                Ok(response) => {
                    // Parse JSON response: {"cmd": "get_clipboard", "text": "..."}
                    // Find "text": " pattern and extract value
                    if let Some(text_start) = response.find("\"text\": \"") {
                        let rest = &response[text_start + 9..]; // Skip "text": "
                                                                // Find closing quote (not escaped)
                        let mut end_idx = 0;
                        let mut chars = rest.chars().peekable();
                        let mut escaped = false;
                        while let Some(c) = chars.next() {
                            if escaped {
                                escaped = false;
                            } else if c == '\\' {
                                escaped = true;
                            } else if c == '"' {
                                break;
                            }
                            end_idx += c.len_utf8();
                        }

                        if end_idx > 0 {
                            let text = &rest[..end_idx];
                            let unescaped = text
                                .replace("\\n", "\n")
                                .replace("\\\"", "\"")
                                .replace("\\\\", "\\");
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                let _ = clipboard.set_text(unescaped.clone());
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        }
        InputCommand::SetClipboard(text, paste) => {
            if let Err(e) = client.set_clipboard(&text, paste) {
                log_verbose!("CLIPBOARD", "Set failed: {}", e);
            }
        }
        InputCommand::InjectText(text) => {
            if let Err(e) = client.inject_text(&text) {
                log_verbose!("INPUT", "Inject text failed: {}", e);
            }
        }
        InputCommand::SetScreenPowerMode(mode) => {
            if let Err(e) = client.set_screen_power_mode(mode) {
                log_verbose!("INPUT", "Set Power Mode failed: {}", e);
            }
        }
    }
}
