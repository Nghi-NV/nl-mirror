use tauri::{AppHandle, State, Manager, Emitter};
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;
use tauri_plugin_shell::{ShellExt, process::CommandChild};
use std::sync::Mutex;

// Global state to track the running sidecar process
pub struct MirrorState {
    pub child: Mutex<Option<CommandChild>>,
    pub tray: Mutex<Option<tauri::tray::TrayIcon>>,
    pub session_id: Mutex<u64>,
    pub screen_was_off: Mutex<bool>,
    pub device_serial: Mutex<Option<String>>, // Track device serial for ADB commands
}

#[tauri::command]
pub async fn start_mirror(
    app: AppHandle,
    state: State<'_, MirrorState>,
    _serial: String,
    bitrate: u32,
    max_size: u32,
    turn_screen_off: bool,
) -> Result<(), String> {
    // 1. Invalidate previous session and cleanup
    {
        let mut child_guard = state.child.lock().unwrap();
        if let Some(child) = child_guard.take() {
            let _ = child.kill();
        }
    }
    
    // Dispatch Tray cleanup to main thread to avoid macOS crash
    let app_c = app.clone();
    let _ = app.run_on_main_thread(move || {
        let state = app_c.state::<MirrorState>();
        let mut tray_guard = state.tray.lock().unwrap();
        if tray_guard.is_some() {
            let _ = tray_guard.take(); 
        }
        let _ = app_c.remove_tray_by_id("main-tray");
    });

    // Small delay to let the main thread task finish before we might create a new one
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let current_session = {
        let mut session_guard = state.session_id.lock().unwrap();
        *session_guard += 1;
        *session_guard
    };

    // 2. Prepare new sidecar
    let mut args = vec![
        "mirror".to_string(),
        "--bitrate".to_string(), bitrate.to_string(),
        "--max-size".to_string(), max_size.to_string(),
        "--audio".to_string(),
    ];

    if turn_screen_off {
        args.push("--turn-screen-off".to_string());
    }

    // Record state for stop_mirror
    *state.screen_was_off.lock().unwrap() = turn_screen_off;
    *state.device_serial.lock().unwrap() = Some(_serial.clone());

    let sidecar_command = app.shell().sidecar("nl-host").map_err(|e| e.to_string())?
        .args(args);

    // 3. Spawn and track
    let (mut rx, child) = sidecar_command.spawn().map_err(|e| format!("Failed to launch client: {}", e))?;
    
    // Store handle
    *state.child.lock().unwrap() = Some(child);

    // Monitoring process exit and output
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_shell::process::CommandEvent;
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    // Forward nl-host stdout to console
                    println!("[nl-host] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Stderr(line) => {
                    // Forward nl-host stderr to console
                    eprintln!("[nl-host] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Terminated(_) => {
                    let state = app_handle.state::<MirrorState>();
                
                    // Only cleanup if this is STILL the session that spawned this loop
                    {
                        let active_session = *state.session_id.lock().unwrap();
                        if active_session != current_session {
                            return; 
                        }
                    }

                // Cleanup
                {
                    let mut child_guard = state.child.lock().unwrap();
                    *child_guard = None;
                }
                
                let app_c = app_handle.clone();
                let _ = app_handle.run_on_main_thread(move || {
                    let state = app_c.state::<MirrorState>();
                    let mut tray_guard = state.tray.lock().unwrap();
                    let _ = tray_guard.take();
                    let _ = app_c.remove_tray_by_id("main-tray");
                });

                let _ = app_handle.emit("mirror-stopped", ());

                #[cfg(target_os = "macos")]
                let _ = app_handle.set_activation_policy(ActivationPolicy::Regular);

                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                break;
                }
                _ => {}
            }
        }
    });

    // 4. Create new tray
    let app_c = app.clone();
    let _ = app.run_on_main_thread(move || {
        let state = app_c.state::<MirrorState>();
        if let Ok(tray) = crate::tray::init_tray(&app_c) {
            *state.tray.lock().unwrap() = Some(tray);
        }
    });

    // 5. App State & Dock Icon
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(ActivationPolicy::Accessory);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    let _ = app.emit("mirror-started", ());

    Ok(())
}

#[tauri::command]
pub async fn stop_mirror(
    app: AppHandle,
    state: State<'_, MirrorState>,
) -> Result<(), String> {
    // 0. Invalidate current session
    {
        let mut session_guard = state.session_id.lock().unwrap();
        *session_guard += 1;
    }

    // 1. Restore screen via nl-host control port 8889 (BEFORE killing the process)
    // This sends set_screen_power_mode command to undo the --turn-screen-off flag
    println!("Sending screen restore command via control port...");
    let _ = std::thread::spawn(|| {
        use std::io::Write;
        if let Ok(mut stream) = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:8889".parse().unwrap(),
            std::time::Duration::from_millis(500)
        ) {
            // MODE 2 = ON (Normal mode)
            let cmd = r#"{"cmd":"set_screen_power_mode","mode":2}"#;
            let _ = stream.write_all(format!("{}\n", cmd).as_bytes());
            let _ = stream.flush();
            println!("Screen restore command sent.");
        } else {
            println!("Control port not available, will try keyevent fallback.");
        }
    }).join();

    // Small delay to let the command take effect
    std::thread::sleep(std::time::Duration::from_millis(200));

    // 2. Kill the process
    {
        let mut child_guard = state.child.lock().unwrap();
        if let Some(child) = child_guard.take() {
            let _ = child.kill();
        }
    }

    // 3. Explicitly remove Tray Icon on main thread
    let app_c = app.clone();
    let _ = app.run_on_main_thread(move || {
        let state = app_c.state::<MirrorState>();
        let mut tray_guard = state.tray.lock().unwrap();
        let _ = tray_guard.take(); 
        let _ = app_c.remove_tray_by_id("main-tray");
    });

    // 4. Notify UI
    let _ = app.emit("mirror-stopped", ());

    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(ActivationPolicy::Regular);

    // 5. Restore window visibility
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}

#[tauri::command]
pub async fn hide_launcher(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    Ok(())
}

#[tauri::command]
pub async fn show_launcher(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}
