use anyhow::Result;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// ControlClient for sending commands to nl-android.
pub struct ControlClient {
    input_stream: TcpStream,
    rpc_stream: TcpStream,
    drain_running: Arc<AtomicBool>,
    drain_handle: Option<JoinHandle<()>>,
}

impl ControlClient {
    pub const META_SHIFT_ON: i32 = 0x1; // AMETA_SHIFT_ON
    pub const META_CTRL_ON: i32 = 0x1000; // AMETA_CTRL_ON
    pub const META_META_ON: i32 = 0x10000; // AMETA_META_ON

    pub fn connect(host: &str, port: u16) -> Result<Self> {
        // 1. Input Connection (Async writes + Background Drain)
        // We use a separate connection for input to allow fire-and-forget sending
        // while a background thread continuously drains the responses from the server.
        // This prevents the TCP Receive Window from filling up (Deadlock)
        // without incurring the RTT latency of synchronous reads for every keystroke.
        let input_stream = TcpStream::connect(format!("{}:{}", host, port))?;
        input_stream.set_nodelay(true)?;

        let drain_running = Arc::new(AtomicBool::new(true));
        let drain_running_clone = drain_running.clone();
        let drain_stream = input_stream.try_clone()?;
        drain_stream.set_read_timeout(Some(std::time::Duration::from_millis(500)))?;

        let drain_handle = thread::spawn(move || {
            let mut reader = BufReader::new(drain_stream);
            let mut buf = String::new();
            while drain_running_clone.load(Ordering::SeqCst) {
                buf.clear();
                // Read and discard output to keep window open
                match reader.read_line(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(_) => {}     // Discard
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::TimedOut
                            || e.kind() == std::io::ErrorKind::WouldBlock
                        {
                            continue; // Timeout, check running flag
                        }
                        break; // Other error
                    }
                }
            }
        });

        // 2. RPC Connection (Synchronous Request-Response)
        // Used for commands that need a return value (e.g. get_clipboard)
        let rpc_stream = TcpStream::connect(format!("{}:{}", host, port))?;
        rpc_stream.set_nodelay(true)?;
        rpc_stream.set_read_timeout(Some(std::time::Duration::from_millis(500)))?;

        Ok(Self {
            input_stream,
            rpc_stream,
            drain_running,
            drain_handle: Some(drain_handle),
        })
    }

    pub fn set_timeout(&self, duration: std::time::Duration) -> Result<()> {
        self.rpc_stream.set_read_timeout(Some(duration))?;
        Ok(())
    }

    // ===== Keyboard Events =====

    /// Inject a keycode event (down/up) with meta state
    pub fn inject_keycode(&mut self, action: &str, keycode: i32, meta_state: i32) -> Result<()> {
        let cmd = format!(
            r#"{{"cmd": "keycode", "action": "{}", "keyCode": {}, "metaState": {}}}"#,
            action, keycode, meta_state
        );
        self.send_command_async(&cmd)
    }

    // ===== Clipboard =====

    pub fn set_clipboard(&mut self, text: &str, paste: bool) -> Result<()> {
        let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
        let cmd = format!(
            r#"{{"cmd": "set_clipboard", "text": "{}", "paste": {}}}"#,
            escaped, paste
        );
        self.send_command_async(&cmd)
    }

    pub fn get_clipboard(&mut self, copy: bool) -> Result<String> {
        let cmd = format!(r#"{{"cmd": "get_clipboard", "copy": {}}}"#, copy);
        self.send_command_sync(&cmd)
    }

    /// Inject text directly as key events (like scrcpy)
    pub fn inject_text(&mut self, text: &str) -> Result<()> {
        let escaped = text
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        let cmd = format!(r#"{{"cmd": "text", "text": "{}"}}"#, escaped);
        self.send_command_async(&cmd)
    }

    // ===== Touch Commands =====

    pub fn tap(&mut self, x: f32, y: f32) -> Result<()> {
        let cmd = format!(r#"{{"cmd": "tap", "x": {}, "y": {}}}"#, x, y);
        self.send_command_async(&cmd)
    }

    pub fn swipe(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, duration_ms: u64) -> Result<()> {
        let cmd = format!(
            r#"{{"cmd": "swipe", "x1": {}, "y1": {}, "x2": {}, "y2": {}, "duration": {}}}"#,
            x1, y1, x2, y2, duration_ms
        );
        self.send_command_async(&cmd)
    }

    pub fn long_press(&mut self, x: f32, y: f32, duration_ms: u64) -> Result<()> {
        let cmd = format!(
            r#"{{"cmd": "long_press", "x": {}, "y": {}, "duration": {}}}"#,
            x, y, duration_ms
        );
        self.send_command_async(&cmd)
    }

    pub fn get_hierarchy(&mut self) -> Result<String> {
        let cmd = r#"{"cmd": "hierarchy"}"#;
        self.send_command_sync(cmd)
    }

    pub fn get_stats(&mut self) -> Result<String> {
        let cmd = r#"{"cmd": "stats"}"#;
        self.send_command_sync(cmd)
    }

    pub fn set_screen_power_mode(&mut self, mode: i32) -> Result<()> {
        let cmd = format!(r#"{{"cmd": "set_screen_power_mode", "mode": {}}}"#, mode);
        self.send_command_async(&cmd)
    }

    // ===== Internal =====

    fn send_command_async(&mut self, cmd: &str) -> Result<()> {
        writeln!(self.input_stream, "{}", cmd)?;
        self.input_stream.flush()?;
        Ok(())
    }

    fn send_command_sync(&mut self, cmd: &str) -> Result<String> {
        writeln!(self.rpc_stream, "{}", cmd)?;
        self.rpc_stream.flush()?;

        let mut reader = BufReader::new(&self.rpc_stream);
        let mut response = String::new();
        reader.read_line(&mut response)?;
        Ok(response)
    }
}

impl Drop for ControlClient {
    fn drop(&mut self) {
        // Signal drain thread to stop
        self.drain_running.store(false, Ordering::SeqCst);

        // Shutdown streams to unblock any blocking reads
        let _ = self.input_stream.shutdown(std::net::Shutdown::Both);
        let _ = self.rpc_stream.shutdown(std::net::Shutdown::Both);

        // Wait for drain thread to finish
        if let Some(handle) = self.drain_handle.take() {
            let _ = handle.join();
        }
    }
}
