// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

mod adb;
mod launcher;
mod network;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Device tracker auto-starts, pairing server is on-demand
            adb::start_device_tracker(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app_handle = window.app_handle();
                let state = app_handle.state::<launcher::MirrorState>();
                let is_mirroring = {
                    let child_guard = state.child.lock().unwrap();
                    child_guard.is_some()
                };

                if is_mirroring {
                    // Prevent the app from exiting if mirroring is active
                    api.prevent_close();
                    // Hide the window instead
                    let _ = window.hide();
                }
                // If not mirroring, allow the default CloseRequested behavior (quit)
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            adb::get_devices,
            adb::setup_forwarding,
            adb::start_server,
            adb::stop_server,
            adb::enable_wifi,
            adb::connect_wifi,
            adb::install_apk,
            adb::deploy_server,
            adb::init_session,
            launcher::start_mirror,
            launcher::stop_mirror,
            launcher::hide_launcher,
            launcher::show_launcher,
            network::get_local_ip,
            network::start_pairing_server,
            network::stop_pairing_server
        ])
        .manage(launcher::MirrorState {
            child: std::sync::Mutex::new(None),
            tray: std::sync::Mutex::new(None),
            session_id: std::sync::Mutex::new(0),
            screen_was_off: std::sync::Mutex::new(false),
            device_serial: std::sync::Mutex::new(None),
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
