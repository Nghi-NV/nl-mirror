use tauri_plugin_shell::ShellExt;
use serde::{Serialize, Deserialize};
use tauri::{AppHandle, Runtime, Emitter};
use tauri::Manager; // Import Manager for path access

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub serial: String,
    pub state: String,
    pub model: String,
}

#[tauri::command]
pub async fn get_devices<R: Runtime>(app: AppHandle<R>) -> Result<Vec<Device>, String> {
    let output = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["devices", "-l"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to execute adb devices".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout.lines().skip(1) {
        if line.trim().is_empty() { continue; }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let serial = parts[0].to_string();
            let state = parts[1].to_string();
            
            // Extract model if available (model:Pixel_6)
            let mut model = "Unknown".to_string();
            for part in &parts {
                if part.starts_with("model:") {
                    model = part.replace("model:", "").replace("_", " ");
                }
            }

            devices.push(Device { serial, state, model });
        }
    }

    Ok(devices)
}

#[tauri::command]
pub async fn setup_forwarding<R: Runtime>(app: AppHandle<R>, serial: String) -> Result<(), String> {
    // 1. Forward Video Port (8888)
    let _ = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "forward", "tcp:8888", "tcp:8888"])
        .output()
        .await
        .map_err(|e| format!("Forward 8888 failed: {}", e))?;

    // 2. Forward Control Port (8889)
    let _ = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "forward", "tcp:8889", "tcp:8889"])
        .output()
        .await
        .map_err(|e| format!("Forward 8889 failed: {}", e))?;

    // 3. Forward Audio Port (8890)
    let _ = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "forward", "tcp:8890", "tcp:8890"])
        .output()
        .await
        .map_err(|e| format!("Forward 8890 failed: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn deploy_server<R: Runtime>(app: AppHandle<R>, serial: String) -> Result<String, String> {
    // 1. Resolve path to bundled APK
    let resource_path = app.path().resolve("binaries/nl-mirror.apk", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve resource: {}", e))?;
        
    let apk_path_str = resource_path.to_string_lossy().to_string();

    // 2. Push APK to /data/local/tmp/
    let output = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "push", &apk_path_str, "/data/local/tmp/nl-mirror.apk"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok("Server APK Deployed".to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub async fn start_server<R: Runtime>(app: AppHandle<R>, serial: String) -> Result<(), String> {
    // 1. Kill old server (using full package name for better reliability)
    let _ = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "shell", "pkill -f dev.nl.mirror"])
        .output()
        .await; 

    // 2. Start new server
    // We run this in background (spawn) because it blocks
    let cmd = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args([
            "-s", &serial, 
            "shell", 
            "CLASSPATH=/data/local/tmp/nl-mirror.apk app_process / dev.nl.mirror.core.App"
        ]);
    
    let (mut _rx, _child) = cmd.spawn().map_err(|e| format!("Failed to start server: {}", e))?;
    
    // Give it a moment to initialize
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

#[tauri::command]
pub async fn stop_server<R: Runtime>(app: AppHandle<R>, serial: String) -> Result<(), String> {
    let _ = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "shell", "pkill -f 'app_process.*nl-mirror'"])
        .output()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn enable_wifi<R: Runtime>(app: AppHandle<R>, serial: String) -> Result<String, String> {
    // 1. Get IP Address FIRST (While USB is stable)
    // Multi-strategy with Debug Capture
    let mut ip_address = String::new();
    let mut debug_log = String::from("Debug Log:\n");

    // Strategy A: ip route get 1
    // Split arguments to avoid quoting issues
    let cmd_a = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "shell", "ip", "route", "get", "1"]);
    
    match cmd_a.output().await {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            debug_log.push_str(&format!("Cmd A (ip route):\nSTDOUT: [{}]\nSTDERR: [{}]\n", stdout.trim(), stderr.trim()));
            
            // Parse "src 192.168.1.5"
            let parts: Vec<&str> = stdout.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "src" && i + 1 < parts.len() {
                    ip_address = parts[i+1].to_string();
                    break;
                }
            }
        },
        Err(e) => debug_log.push_str(&format!("Cmd A Error: {}\n", e)),
    }

    // Strategy B: ip -f inet addr show wlan0 (Only if A failed)
    if ip_address.is_empty() {
        let cmd_b = app.shell().sidecar("adb").map_err(|e| e.to_string())?
            .args(["-s", &serial, "shell", "ip", "-f", "inet", "addr", "show", "wlan0"]);

        match cmd_b.output().await {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                debug_log.push_str(&format!("Cmd B (ip addr):\nSTDOUT: [{}]\nSTDERR: [{}]\n", stdout.trim(), stderr.trim()));
                
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    // Match "inet 192.168.1.5/24"
                    if trimmed.starts_with("inet ") {
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let ip_cidr = parts[1];
                            ip_address = ip_cidr.split('/').next().unwrap_or("").to_string();
                            if !ip_address.is_empty() { break; }
                        }
                    }
                }
            },
            Err(e) => debug_log.push_str(&format!("Cmd B Error: {}\n", e)),
        }
    }

    // Strategy C: ifconfig wlan0 (Only if A & B failed)
    if ip_address.is_empty() {
        let cmd_c = app.shell().sidecar("adb").map_err(|e| e.to_string())?
            .args(["-s", &serial, "shell", "ifconfig", "wlan0"]);
            
        match cmd_c.output().await {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                debug_log.push_str(&format!("Cmd C (ifconfig):\nSTDOUT: [{}]\nSTDERR: [{}]\n", stdout.trim(), stderr.trim()));
                
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.contains("inet addr:") {
                        let parts: Vec<&str> = trimmed.split("inet addr:").collect();
                        if parts.len() > 1 {
                            let ip_part = parts[1].split_whitespace().next().unwrap_or("");
                            if !ip_part.is_empty() { 
                                ip_address = ip_part.to_string();
                                break; 
                            }
                        }
                    }
                }
            },
            Err(e) => debug_log.push_str(&format!("Cmd C Error: {}\n", e)),
    }
    }

    // 2. Switch to TCPIP (Now safe to do)
    let output = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "tcpip", "5555"])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    
    // Return result
    if !ip_address.is_empty() {
        Ok(ip_address)
    } else {
        Ok(format!("Enabled. IP Detection Failed. {}", debug_log))
    }
}

#[tauri::command]
pub async fn connect_wifi<R: Runtime>(app: AppHandle<R>, ip: String) -> Result<String, String> {
    let output = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["connect", &format!("{}:5555", ip)])
        .output()
        .await
        .map_err(|e| e.to_string())?;
        
    let stdout = String::from_utf8_lossy(&output.stdout);
    if output.status.success() {
        Ok(stdout.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}


#[tauri::command]
pub async fn install_apk<R: Runtime>(app: AppHandle<R>, serial: String, path: String) -> Result<String, String> {
    let output = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "install", "-r", &path])
        .output()
        .await
        .map_err(|e| e.to_string())?;
        
    if output.status.success() {
        Ok("Install Success".to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub async fn init_session<R: Runtime>(app: AppHandle<R>, serial: String) -> Result<String, String> {
    // 1. Setup Forwarding (Fastest, do first)
    app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "forward", "tcp:8888", "tcp:8888"])
        .output().await.map_err(|e| e.to_string())?;
    app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "forward", "tcp:8889", "tcp:8889"])
        .output().await.map_err(|e| e.to_string())?;
    app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "forward", "tcp:8890", "tcp:8890"])
        .output().await.map_err(|e| e.to_string())?;

    // 2. Resolve Local APK
    let resource_path = app.path().resolve("binaries/nl-mirror.apk", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve resource: {}", e))?;
    let local_apk_path = resource_path.to_string_lossy().to_string();

    // 3. Smart Deploy: Check bundled APK size vs Device APK size
    let local_size = std::fs::metadata(&resource_path).map_err(|e| e.to_string())?.len();
    
    let size_output = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "shell", "stat -c %s /data/local/tmp/nl-mirror.apk"])
        .output().await.map_err(|e| e.to_string())?;
    
    let device_size_str = String::from_utf8_lossy(&size_output.stdout).trim().to_string();
    let device_size = device_size_str.parse::<u64>().unwrap_or(0);

    // Only push if sizes differ (Basic optimization)
    // For robust check we could use md5, but size is usually good enough for dev iteration
    if local_size != device_size {
         app.shell().sidecar("adb").map_err(|e| e.to_string())?
            .args(["-s", &serial, "push", &local_apk_path, "/data/local/tmp/nl-mirror.apk"])
            .output().await.map_err(|e| e.to_string())?;
    }

    // 4. Stop Old Server
    app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args(["-s", &serial, "shell", "pkill -f dev.nl.mirror"])
        .output().await.ok(); 

    // 5. Start New Server (Detached)
    // Use sh -c to ensure CLASSPATH and redirection work reliably across all shells
    let cmd = app.shell().sidecar("adb").map_err(|e| e.to_string())?
        .args([
            "-s", &serial, 
            "shell", 
            "sh", "-c", 
            "'CLASSPATH=/data/local/tmp/nl-mirror.apk app_process / dev.nl.mirror.core.App >/dev/null 2>&1 &'"
        ]);
    
    // We expect this to exit immediately because of &
    let (mut _rx, _child) = cmd.spawn().map_err(|e| format!("Failed to start server: {}", e))?;
    
    // Give it a moment to initialize
    // Optimized: Wait up to 2s, but check every 200ms
    let mut started = false;
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(200));
        
        let check_cmd = app.shell().sidecar("adb").map_err(|e| e.to_string())?
            .args(["-s", &serial, "shell", "pgrep", "-f", "dev.nl.mirror.core.App"])
            .output().await.ok();
            
        if let Some(out) = check_cmd {
             if !String::from_utf8_lossy(&out.stdout).trim().is_empty() {
                 started = true;
                 break;
             }
        }
    }

    if !started {
        // Capture Logcat to diagnose the crash (Last 50 lines)
        let log_cmd = app.shell().sidecar("adb").map_err(|e| e.to_string())?
            .args(["-s", &serial, "shell", "logcat", "-d", "-t", "50", "*:E"])
            .output().await.ok();
            
        let logs = log_cmd.map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                          .unwrap_or_else(|| "Could not capture logcat.".to_string());

        return Err(format!("Server failed to start. Crash Logs:\n{}", logs));
    }

    Ok("Session Initialized".to_string())
}

#[tauri::command]
pub fn start_device_tracker(app: AppHandle) {
    // We launch a persistent task that runs `adb track-devices` loop
    tauri::async_runtime::spawn(async move {
        // Retry loop in case adb server crashes or track-devices exits
        loop {
            let shell = app.shell();
            
            // Use sidecar command which automatically finds the bundled adb
            let command = shell.sidecar("adb")
                .expect("Failed to create sidecar command");

            // track-devices is a long-running command
            let (mut rx, _child) = match command.args(["track-devices"]).spawn() {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("Failed to spawn ADB tracker: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            };
            
            // Listen to stdout events
            use tauri_plugin_shell::process::CommandEvent;
            while let Some(event) = rx.recv().await {
                if let CommandEvent::Stdout(line_bytes) = event {
                     let line = String::from_utf8_lossy(&line_bytes);
                     if !line.trim().is_empty() {
                         // When track-devices outputs anything, the device list changed
                         let _ = app.emit("device-changed", ());
                     }
                }
                // If the process terminates, the loop breaks eventually (channel close)
            }
            
            eprintln!("ADB Tracker exited. Restarting in 2s...");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}
