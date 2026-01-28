//! Binary control protocol for nl-host
//!
//! Low-latency binary protocol matching nl-android's BinaryCommandServer.
//! Uses DataOutputStream format (big-endian) for all values.

use anyhow::Result;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Message types - matching nl-android's ControlMessage
#[repr(u8)]
pub enum MessageType {
    InjectKeycode = 0,
    InjectText = 1,
    InjectTouchEvent = 2,
    InjectScrollEvent = 3,
    BackOrScreenOn = 4,
    ExpandNotificationPanel = 5,
    ExpandSettingsPanel = 6,
    CollapsePanels = 7,
    GetClipboard = 8,
    SetClipboard = 9,
    SetDisplayPower = 10,
    RotateDevice = 11,
    // nl-mirror extensions
    Tap = 20,
    Swipe = 21,
    LongPress = 22,
    Key = 23,
    Hierarchy = 24,
    Stats = 25,
    StartMockLocation = 26,
    StopMockLocation = 27,
    SetLocation = 28,
}

/// Response types from server
#[repr(u8)]
pub enum ResponseType {
    Ok = 0,
    Error = 1,
    Clipboard = 2,
    Hierarchy = 3,
    Stats = 4,
}

/// BinaryControlClient for low-latency control
pub struct BinaryControlClient {
    stream: TcpStream,
    drain_running: Arc<AtomicBool>,
    drain_handle: Option<JoinHandle<()>>,
}

impl BinaryControlClient {
    /// Connect to the binary command server
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        let stream = TcpStream::connect(format!("{}:{}", host, port))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(std::time::Duration::from_millis(500)))?;

        let drain_running = Arc::new(AtomicBool::new(true));
        let drain_running_clone = drain_running.clone();
        let drain_stream = stream.try_clone()?;

        // Background thread to drain responses (for async commands)
        let drain_handle = thread::spawn(move || {
            let mut buf = [0u8; 64];
            while drain_running_clone.load(Ordering::SeqCst) {
                match (&drain_stream).read(&mut buf) {
                    Ok(0) => break,
                    Ok(_) => {} // Discard
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::TimedOut
                            && e.kind() != std::io::ErrorKind::WouldBlock
                        {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Self {
            stream,
            drain_running,
            drain_handle: Some(drain_handle),
        })
    }

    // ===== Touch Commands =====

    /// Inject a touch event (ACTION_DOWN=0, ACTION_MOVE=2, ACTION_UP=1)
    pub fn inject_touch(
        &mut self,
        action: u8,
        pointer_id: i64,
        x: i32,
        y: i32,
        screen_width: u16,
        screen_height: u16,
        pressure: f32,
    ) -> Result<()> {
        let mut buf = Vec::with_capacity(32);
        buf.push(MessageType::InjectTouchEvent as u8);
        buf.push(action);
        buf.extend_from_slice(&pointer_id.to_be_bytes());
        buf.extend_from_slice(&x.to_be_bytes());
        buf.extend_from_slice(&y.to_be_bytes());
        buf.extend_from_slice(&screen_width.to_be_bytes());
        buf.extend_from_slice(&screen_height.to_be_bytes());
        buf.extend_from_slice(&float_to_u16_fixed(pressure).to_be_bytes());
        buf.extend_from_slice(&0i32.to_be_bytes()); // actionButton
        buf.extend_from_slice(&0i32.to_be_bytes()); // buttons
        self.send(&buf)
    }

    /// Touch down at coordinates (scrcpy-compatible 32-byte format)
    pub fn touch_down(&mut self, x: f32, y: f32) -> Result<()> {
        // ACTION_DOWN = 0
        self.send_touch_event(0, x, y, 1.0)
    }

    /// Touch move at coordinates (scrcpy-compatible 32-byte format)
    pub fn touch_move(&mut self, x: f32, y: f32) -> Result<()> {
        // ACTION_MOVE = 2
        self.send_touch_event(2, x, y, 1.0)
    }

    /// Touch up at coordinates (scrcpy-compatible 32-byte format)
    pub fn touch_up(&mut self, x: f32, y: f32) -> Result<()> {
        // ACTION_UP = 1
        self.send_touch_event(1, x, y, 0.0)
    }

    /// Send touch event in scrcpy 32-byte format
    fn send_touch_event(&mut self, action: u8, x: f32, y: f32, pressure: f32) -> Result<()> {
        let mut buf = Vec::with_capacity(32);
        buf.push(MessageType::InjectTouchEvent as u8); // type
        buf.push(action); // action
        buf.extend_from_slice(&(-1i64).to_be_bytes()); // pointer_id = -1 (virtual finger)
        buf.extend_from_slice(&(x as i32).to_be_bytes()); // x
        buf.extend_from_slice(&(y as i32).to_be_bytes()); // y
        buf.extend_from_slice(&0u16.to_be_bytes()); // screen_width (0 = use actual)
        buf.extend_from_slice(&0u16.to_be_bytes()); // screen_height (0 = use actual)
        buf.extend_from_slice(&float_to_u16_fixed(pressure).to_be_bytes()); // pressure
        buf.extend_from_slice(&0i32.to_be_bytes()); // action_button
        buf.extend_from_slice(&0i32.to_be_bytes()); // buttons
        self.send(&buf)
    }

    /// Tap at coordinates
    pub fn tap(&mut self, x: f32, y: f32) -> Result<()> {
        let mut buf = Vec::with_capacity(9);
        buf.push(MessageType::Tap as u8);
        buf.extend_from_slice(&x.to_be_bytes());
        buf.extend_from_slice(&y.to_be_bytes());
        self.send(&buf)
    }

    /// Swipe from (x1, y1) to (x2, y2)
    pub fn swipe(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, duration_ms: u64) -> Result<()> {
        let mut buf = Vec::with_capacity(25);
        buf.push(MessageType::Swipe as u8);
        buf.extend_from_slice(&x1.to_be_bytes());
        buf.extend_from_slice(&y1.to_be_bytes());
        buf.extend_from_slice(&x2.to_be_bytes());
        buf.extend_from_slice(&y2.to_be_bytes());
        buf.extend_from_slice(&duration_ms.to_be_bytes());
        self.send(&buf)
    }

    /// Long press at coordinates
    pub fn long_press(&mut self, x: f32, y: f32, duration_ms: u64) -> Result<()> {
        let mut buf = Vec::with_capacity(17);
        buf.push(MessageType::LongPress as u8);
        buf.extend_from_slice(&x.to_be_bytes());
        buf.extend_from_slice(&y.to_be_bytes());
        buf.extend_from_slice(&duration_ms.to_be_bytes());
        self.send(&buf)
    }

    // ===== Keyboard Commands =====

    /// Inject a keycode event
    pub fn inject_keycode(
        &mut self,
        action: u8,
        keycode: i32,
        repeat: i32,
        meta_state: i32,
    ) -> Result<()> {
        let mut buf = Vec::with_capacity(14);
        buf.push(MessageType::InjectKeycode as u8);
        buf.push(action);
        buf.extend_from_slice(&keycode.to_be_bytes());
        buf.extend_from_slice(&repeat.to_be_bytes());
        buf.extend_from_slice(&meta_state.to_be_bytes());
        self.send(&buf)
    }

    /// Inject text
    pub fn inject_text(&mut self, text: &str) -> Result<()> {
        let bytes = text.as_bytes();
        let mut buf = Vec::with_capacity(5 + bytes.len());
        buf.push(MessageType::InjectText as u8);
        buf.extend_from_slice(&(bytes.len() as i32).to_be_bytes());
        buf.extend_from_slice(bytes);
        self.send(&buf)
    }

    /// Press a key (down + up)
    pub fn press_key(&mut self, keycode: i32) -> Result<()> {
        let mut buf = Vec::with_capacity(5);
        buf.push(MessageType::Key as u8);
        buf.extend_from_slice(&keycode.to_be_bytes());
        self.send(&buf)
    }

    // ===== Clipboard =====

    /// Set clipboard text
    pub fn set_clipboard(&mut self, text: &str, paste: bool) -> Result<()> {
        let bytes = text.as_bytes();
        let mut buf = Vec::with_capacity(14 + bytes.len());
        buf.push(MessageType::SetClipboard as u8);
        buf.extend_from_slice(&0i64.to_be_bytes()); // sequence (unused)
        buf.push(if paste { 1 } else { 0 });
        buf.extend_from_slice(&(bytes.len() as i32).to_be_bytes());
        buf.extend_from_slice(bytes);
        self.send(&buf)
    }

    // ===== Display =====

    /// Set screen power mode (true = on, false = off)
    pub fn set_screen_power(&mut self, on: bool) -> Result<()> {
        let buf = vec![MessageType::SetDisplayPower as u8, if on { 1 } else { 0 }];
        self.send(&buf)
    }

    // ===== Location =====

    /// Set mock location
    pub fn set_location(
        &mut self,
        lat: f64,
        lon: f64,
        alt: f64,
        bearing: f32,
        speed: f32,
    ) -> Result<()> {
        let mut buf = Vec::with_capacity(33);
        buf.push(MessageType::SetLocation as u8);
        buf.extend_from_slice(&lat.to_be_bytes());
        buf.extend_from_slice(&lon.to_be_bytes());
        buf.extend_from_slice(&alt.to_be_bytes());
        buf.extend_from_slice(&bearing.to_be_bytes());
        buf.extend_from_slice(&speed.to_be_bytes());
        self.send(&buf)
    }

    /// Start mock location
    pub fn start_mock_location(&mut self) -> Result<()> {
        self.send(&[MessageType::StartMockLocation as u8])
    }

    /// Stop mock location
    pub fn stop_mock_location(&mut self) -> Result<()> {
        self.send(&[MessageType::StopMockLocation as u8])
    }

    // ===== Internal =====

    fn send(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data)?;
        self.stream.flush()?;
        Ok(())
    }
}

impl Drop for BinaryControlClient {
    fn drop(&mut self) {
        self.drain_running.store(false, Ordering::SeqCst);
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        if let Some(handle) = self.drain_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Convert float (0.0-1.0) to unsigned 16-bit fixed point
fn float_to_u16_fixed(value: f32) -> i16 {
    (value.clamp(0.0, 1.0) * 65535.0) as i16
}
