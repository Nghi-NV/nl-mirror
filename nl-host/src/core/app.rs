//! Core application logic

use crate::core::{FrameBuffer, FrameData};
use crate::input::{map_keycode, start_input_thread, InputCommand};
use crate::network::{start_video_receiver, VideoReceiverHandle};
use crate::utils::save_screenshot_yuv;
use crate::video::{start_decoder_thread, MirrorRenderer};
use crate::{log_debug, log_error, log_info, log_verbose};
use crossbeam_channel::Sender;
use std::sync::{Arc, Mutex};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct MirrorApp {
    pub host: String,
    pub port: u16,
    pub bitrate: u32,
    pub max_size: u32,
    pub turn_screen_off: bool,
    pub frame_buffer: Arc<FrameBuffer>,
    pub renderer: Option<MirrorRenderer>,
    pub current_width: u32,
    pub current_height: u32,
    pub window: Option<Arc<winit::window::Window>>,
    pub last_count: u64,
    pub last_log: std::time::Instant,
    pub last_60s_log: std::time::Instant,
    pub threads_started: bool,
    // Input handling
    pub input_sender: Option<Sender<InputCommand>>,
    pub cursor_position: Option<(f64, f64)>,
    pub mouse_pressed: bool,
    pub drag_start: Option<(f64, f64)>,
    pub ctrl_pressed: bool,
    pub cmd_pressed: bool,
    pub shift_pressed: bool,
    // Store the last rendered frame for screenshots
    pub last_frame: Arc<Mutex<Option<FrameData>>>,
    // Video receiver handle to keep thread alive
    pub video_receiver: Option<VideoReceiverHandle>,
}

impl MirrorApp {
    pub fn new(
        host: String,
        port: u16,
        bitrate: u32,
        max_size: u32,
        turn_screen_off: bool,
    ) -> Self {
        Self {
            host,
            port,
            bitrate,
            max_size,
            turn_screen_off,
            frame_buffer: Arc::new(FrameBuffer::new()),
            renderer: None,
            current_width: 0,
            current_height: 0,
            window: None,
            last_count: 0,
            last_log: std::time::Instant::now(),
            last_60s_log: std::time::Instant::now(),
            threads_started: false,
            input_sender: None,
            cursor_position: None,
            mouse_pressed: false,
            drag_start: None,
            ctrl_pressed: false,
            cmd_pressed: false,
            shift_pressed: false,
            last_frame: Arc::new(Mutex::new(None)),
            video_receiver: None,
        }
    }

    fn window_to_video(&self, pos: (f64, f64)) -> (f32, f32) {
        let window_size = self
            .window
            .as_ref()
            .map(|w| w.inner_size())
            .unwrap_or(winit::dpi::PhysicalSize::new(1, 1));

        // Map window coordinates to video coordinates
        let x = (pos.0 / window_size.width as f64) * self.current_width as f64;
        let y = (pos.1 / window_size.height as f64) * self.current_height as f64;

        log_verbose!(
            "INPUT",
            "window_to_video: window={}x{}, video={}x{}, pos=({:.0},{:.0}) -> ({:.0},{:.0})",
            window_size.width,
            window_size.height,
            self.current_width,
            self.current_height,
            pos.0,
            pos.1,
            x,
            y
        );

        (x as f32, y as f32)
    }

    fn send_tap(&mut self, x: f32, y: f32) {
        if let Some(tx) = &self.input_sender {
            let _ = tx.try_send(InputCommand::Tap(x, y));
        }
    }

    fn send_swipe(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        if let Some(tx) = &self.input_sender {
            let _ = tx.try_send(InputCommand::Swipe(x1, y1, x2, y2, 100));
        }
    }

    fn send_long_press(&mut self, x: f32, y: f32) {
        if let Some(tx) = &self.input_sender {
            let _ = tx.try_send(InputCommand::LongPress(x, y, 500));
        }
    }

    fn get_android_clipboard(&mut self) {
        if let Some(tx) = &self.input_sender {
            // true = request device to inject COPY key before reading
            let _ = tx.try_send(InputCommand::GetClipboard(true));
        }
    }

    fn paste_to_android(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                if let Some(tx) = &self.input_sender {
                    // Use SetClipboard with paste=true for atomic operation on server side
                    // This avoids race conditions where paste happens before clipboard sets
                    let _ = tx.try_send(InputCommand::SetClipboard(text, true));
                }
            }
        }
    }

    fn send_keycode(&mut self, action: &str, keycode: i32) {
        if let Some(tx) = &self.input_sender {
            let mut meta = 0;
            if self.ctrl_pressed {
                meta |= crate::network::ControlClient::META_CTRL_ON;
            }
            if self.cmd_pressed {
                meta |= crate::network::ControlClient::META_META_ON;
            }
            if self.shift_pressed {
                meta |= crate::network::ControlClient::META_SHIFT_ON;
            }
            let _ = tx.try_send(InputCommand::Keycode(action.to_string(), keycode, meta));
        }
    }

    fn set_screen_power_mode(&mut self, mode: i32) {
        if let Some(tx) = &self.input_sender {
            // mode: 0 = OFF, 2 = NORMAL
            let _ = tx.try_send(InputCommand::SetScreenPowerMode(mode));
        }
    }
}

impl Drop for MirrorApp {
    fn drop(&mut self) {
        if self.turn_screen_off {
            log_info!("APP", "Exiting: Restoring screen power...");
            // Try to send power on command via control port directly
            // We use a new connection here to ensure it's sent even if channel is closed
            if let Ok(mut stream) = std::net::TcpStream::connect_timeout(
                &format!("{}:{}", self.host, self.port + 1)
                    .parse()
                    .unwrap_or("127.0.0.1:8889".parse().unwrap()),
                std::time::Duration::from_millis(500),
            ) {
                use std::io::Write;
                let cmd = serde_json::json!({
                    "cmd": "set_screen_power_mode",
                    "mode": 2
                });
                let _ = stream.write_all(format!("{}\n", cmd.to_string()).as_bytes());
                let _ = stream.flush();
            }
        }
    }
}

impl ApplicationHandler for MirrorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attrs = winit::window::Window::default_attributes()
            .with_title("NL-Mirror")
            .with_inner_size(winit::dpi::LogicalSize::new(360, 800));

        if let Ok(window) = event_loop.create_window(window_attrs) {
            self.window = Some(Arc::new(window));
        }

        if self.threads_started {
            log_verbose!(
                "APP",
                "resumed() called again, but threads already started - skipping"
            );
            return;
        }
        self.threads_started = true;
        self.renderer = None;
        self.current_width = 0;
        self.current_height = 0;

        // Input Thread - larger buffer for fast typing
        let (input_tx, input_rx) = crossbeam_channel::bounded::<InputCommand>(256);
        self.input_sender = Some(input_tx);
        start_input_thread(self.host.clone(), self.port + 1, input_rx);

        // Send screen off command if requested
        if self.turn_screen_off {
            log_verbose!("APP", "Requesting screen off...");
            self.set_screen_power_mode(0);
        }

        // Network -> Decoder Channel (larger buffer for high bitrate)
        let (tx, rx) = crossbeam_channel::bounded::<Vec<u8>>(256);

        log_verbose!("APP", "Starting decoder and network threads...");

        // Decoder Thread
        start_decoder_thread(rx, self.frame_buffer.clone());

        // Network Receiver Thread
        self.video_receiver = Some(start_video_receiver(
            self.host.clone(),
            self.port,
            self.bitrate,
            self.max_size,
            tx,
        ));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(r) = &mut self.renderer {
                        let _ = r.resize_surface(size.width, size.height);
                    }
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    self.mouse_pressed = true;
                    self.drag_start = self.cursor_position;
                    // Log only when debug is enabled to avoid spam
                    log_debug!("INPUT", "Mouse down at {:?}", self.cursor_position);
                }
                ElementState::Released => {
                    if let (Some(start), Some(end)) = (self.drag_start, self.cursor_position) {
                        let dx = (end.0 - start.0).abs();
                        let dy = (end.1 - start.1).abs();

                        log_verbose!(
                            "INPUT",
                            "Mouse up: start=({:.0},{:.0}), end=({:.0},{:.0})",
                            start.0,
                            start.1,
                            end.0,
                            end.1
                        );

                        let (vx1, vy1) = self.window_to_video(start);
                        let (vx2, vy2) = self.window_to_video(end);

                        if dx < 5.0 && dy < 5.0 {
                            log_verbose!("INPUT", "-> TAP at ({:.0}, {:.0})", vx1, vy1);
                            self.send_tap(vx1, vy1);
                        } else {
                            log_verbose!(
                                "INPUT",
                                "-> SWIPE from ({:.0},{:.0}) to ({:.0},{:.0})",
                                vx1,
                                vy1,
                                vx2,
                                vy2
                            );
                            self.send_swipe(vx1, vy1, vx2, vy2);
                        }
                    }
                    self.mouse_pressed = false;
                    self.drag_start = None;
                }
            },
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Right,
                ..
            } => {
                if let Some(pos) = self.cursor_position {
                    let (vx, vy) = self.window_to_video(pos);
                    self.send_long_press(vx, vy);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.ctrl_pressed = modifiers.state().control_key();
                self.cmd_pressed = modifiers.state().super_key();
                self.shift_pressed = modifiers.state().shift_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    // Update modifier state
                    if keycode == KeyCode::ControlLeft || keycode == KeyCode::ControlRight {
                        self.ctrl_pressed = event.state == ElementState::Pressed;
                    }
                    if keycode == KeyCode::SuperLeft || keycode == KeyCode::SuperRight {
                        self.cmd_pressed = event.state == ElementState::Pressed;
                    }
                    if keycode == KeyCode::ShiftLeft || keycode == KeyCode::ShiftRight {
                        self.shift_pressed = event.state == ElementState::Pressed;
                    }

                    // Revert to original: CTRL or CMD triggers shortcuts
                    let modifier_pressed = self.ctrl_pressed || self.cmd_pressed;

                    if event.state == ElementState::Pressed {
                        if modifier_pressed {
                            log_verbose!(
                                "INPUT",
                                "Modifier pressed, checking shortcut: {:?}",
                                keycode
                            );
                            match keycode {
                                KeyCode::KeyC => {
                                    log_verbose!("INPUT", "Shortcut: Copy (Ctrl+C)");
                                    self.get_android_clipboard();
                                    return;
                                }
                                KeyCode::KeyV => {
                                    log_verbose!("INPUT", "Shortcut: Paste (Ctrl+V)");
                                    self.paste_to_android();
                                    return;
                                }
                                KeyCode::KeyS => {
                                    log_verbose!("INPUT", "Shortcut: Screenshot");
                                    let last_frame_mutex = self.last_frame.clone();
                                    std::thread::spawn(move || {
                                        let pending = last_frame_mutex.lock().unwrap();
                                        if let Some(frame) = pending.clone() {
                                            save_screenshot_yuv(frame);
                                        }
                                    });
                                    return;
                                }
                                _ => {}
                            }
                        }

                        // Non-modifier shortcuts for Android navigation
                        match keycode {
                            // Escape = Android Back
                            KeyCode::Escape => {
                                log_verbose!("INPUT", "Shortcut: Back");
                                self.send_keycode("down", 4); // AKEYCODE_BACK
                                self.send_keycode("up", 4);
                                return;
                            }
                            // F1 = Home
                            KeyCode::F1 => {
                                log_verbose!("INPUT", "Shortcut: Home");
                                self.send_keycode("down", 3); // AKEYCODE_HOME
                                self.send_keycode("up", 3);
                                return;
                            }
                            // F2 = Recent Apps
                            KeyCode::F2 => {
                                log_verbose!("INPUT", "Shortcut: Recent Apps");
                                self.send_keycode("down", 187); // AKEYCODE_APP_SWITCH
                                self.send_keycode("up", 187);
                                return;
                            }
                            // F3 = Volume Down
                            KeyCode::F3 => {
                                log_verbose!("INPUT", "Shortcut: Volume Down");
                                self.send_keycode("down", 25); // AKEYCODE_VOLUME_DOWN
                                self.send_keycode("up", 25);
                                return;
                            }
                            // F4 = Volume Up
                            KeyCode::F4 => {
                                log_verbose!("INPUT", "Shortcut: Volume Up");
                                self.send_keycode("down", 24); // AKEYCODE_VOLUME_UP
                                self.send_keycode("up", 24);
                                return;
                            }
                            // F5 = Power (toggle screen)
                            KeyCode::F5 => {
                                log_verbose!("INPUT", "Shortcut: Power");
                                self.send_keycode("down", 26); // AKEYCODE_POWER
                                self.send_keycode("up", 26);
                                return;
                            }
                            // F6 = Menu
                            KeyCode::F6 => {
                                log_verbose!("INPUT", "Shortcut: Menu");
                                self.send_keycode("down", 82); // AKEYCODE_MENU
                                self.send_keycode("up", 82);
                                return;
                            }
                            // F7 = Turn OFF device screen (screen stays off, mirror continues)
                            KeyCode::F7 => {
                                log_verbose!("INPUT", "Shortcut: Screen OFF (device only)");
                                self.set_screen_power_mode(0); // OFF
                                return;
                            }
                            // F8 = Turn ON device screen
                            KeyCode::F8 => {
                                log_verbose!("INPUT", "Shortcut: Screen ON (device only)");
                                self.set_screen_power_mode(2); // ON
                                return;
                            }
                            // F9 = Swipe Up (for TikTok next video)
                            KeyCode::F9 => {
                                log_verbose!("INPUT", "Shortcut: Swipe Up");
                                let h = self.current_height as f32;
                                let w = self.current_width as f32;
                                self.send_swipe(w / 2.0, h * 0.75, w / 2.0, h * 0.25);
                                return;
                            }
                            // F10 = Swipe Down (for TikTok previous video)
                            KeyCode::F10 => {
                                log_verbose!("INPUT", "Shortcut: Swipe Down");
                                let h = self.current_height as f32;
                                let w = self.current_width as f32;
                                self.send_swipe(w / 2.0, h * 0.25, w / 2.0, h * 0.75);
                                return;
                            }
                            _ => {}
                        }
                    }

                    if let Some(android_keycode) = map_keycode(keycode) {
                        let action = if event.state == ElementState::Pressed {
                            "down"
                        } else {
                            "up"
                        };
                        self.send_keycode(action, android_keycode);
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.last_60s_log.elapsed().as_secs() >= 60 {
            log_verbose!("REN", "about_to_wait active");
            self.last_60s_log = std::time::Instant::now();
        }

        if let Some(frame) = self.frame_buffer.consume() {
            self.last_count += 1;

            // Save last frame for screenshot (use try_lock to avoid blocking render)
            if let Ok(mut last) = self.last_frame.try_lock() {
                *last = Some(frame.clone());
            }

            if frame.width != self.current_width || frame.height != self.current_height {
                log_verbose!(
                    "REN",
                    "Resolution change: {}x{} -> {}x{}",
                    self.current_width,
                    self.current_height,
                    frame.width,
                    frame.height
                );
                if let Some(window) = &self.window {
                    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(
                        frame.width,
                        frame.height,
                    ));

                    if let Ok(renderer) =
                        MirrorRenderer::new(window.clone(), frame.width, frame.height)
                    {
                        self.renderer = Some(renderer);
                        self.current_width = frame.width;
                        self.current_height = frame.height;
                    }
                }
            }

            if let Some(renderer) = &mut self.renderer {
                if let Err(e) = renderer.render_yuv_frame(&frame) {
                    log_error!("REN", "Render failed: {}", e);
                }
            }
        }

        if self.last_log.elapsed().as_secs() >= 10 {
            static mut LAST_RENDER_COUNT: u64 = 0;
            unsafe {
                let rendered = self.last_count - LAST_RENDER_COUNT;
                let fps = rendered / 10;
                let total_frames = self.frame_buffer.get_count();
                log_info!(
                    "REN",
                    "Stats: {} fps, total_frames={}, last_count={}",
                    fps,
                    total_frames,
                    self.last_count
                );
                LAST_RENDER_COUNT = self.last_count;
            }
            self.last_log = std::time::Instant::now();
        }

        if let Some(w) = &self.window {
            w.request_redraw();
        }
        _event_loop.set_control_flow(ControlFlow::Poll);
        // Removed yield_now() - it can cause frame drops
    }
}

pub fn run(
    host: String,
    port: u16,
    bitrate: u32,
    max_size: u32,
    turn_screen_off: bool,
) -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = MirrorApp::new(host, port, bitrate, max_size, turn_screen_off);
    event_loop.run_app(&mut app)?;
    Ok(())
}
