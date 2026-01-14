use local_ip_address::local_ip;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

// Global flags for controlling the server
static PAIRING_SERVER_RUNNING: AtomicBool = AtomicBool::new(false);
static PAIRING_SERVER_STOPPED: AtomicBool = AtomicBool::new(true);
// Track last successfully connected IP
static LAST_CONNECTED_IP: Mutex<Option<String>> = Mutex::new(None);

#[tauri::command]
pub fn get_local_ip() -> Result<String, String> {
    local_ip()
        .map(|ip| ip.to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_pairing_server(app: AppHandle) -> Result<(), String> {
    // Check if already running
    if PAIRING_SERVER_RUNNING.load(Ordering::SeqCst) {
        return Ok(()); // Already running
    }

    // Wait for previous server to fully stop (max 1 second)
    for _ in 0..20 {
        if PAIRING_SERVER_STOPPED.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    PAIRING_SERVER_RUNNING.store(true, Ordering::SeqCst);
    PAIRING_SERVER_STOPPED.store(false, Ordering::SeqCst);
    println!("Starting pairing server...");

    std::thread::spawn(move || {
        use std::net::TcpListener;
        use socket2::{Socket, Domain, Type};
        
        // Create socket with SO_REUSEADDR to avoid "Address already in use" on restart
        let socket = match Socket::new(Domain::IPV4, Type::STREAM, None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create socket: {}", e);
                PAIRING_SERVER_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };
        
        if let Err(e) = socket.set_reuse_address(true) {
            eprintln!("Failed to set SO_REUSEADDR: {}", e);
        }
        
        let addr: std::net::SocketAddr = "0.0.0.0:27015".parse().unwrap();
        if let Err(e) = socket.bind(&addr.into()) {
            eprintln!("Failed to bind pairing server: {}", e);
            PAIRING_SERVER_RUNNING.store(false, Ordering::SeqCst);
            return;
        }
        
        if let Err(e) = socket.listen(128) {
            eprintln!("Failed to listen: {}", e);
            PAIRING_SERVER_RUNNING.store(false, Ordering::SeqCst);
            return;
        }
        
        let listener: TcpListener = socket.into();
        listener.set_nonblocking(true).ok();

        println!("Pairing server listening on 0.0.0.0:27015");

        while PAIRING_SERVER_RUNNING.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    stream.set_nonblocking(false).ok(); // Make blocking for read/write
                    let mut buffer = [0; 512];
                    if let Ok(_) = stream.read(&mut buffer) {
                        let request = String::from_utf8_lossy(&buffer);
                        
                        // Ignore favicon
                        if request.contains("GET /favicon.ico") {
                            continue;
                        }

                        // Handle /status endpoint for polling connection state
                        if request.contains("GET /status") {
                            // Check if IP matches last connected
                            let is_connected = if let Ok(connected) = LAST_CONNECTED_IP.lock() {
                                connected.is_some()
                            } else {
                                false
                            };
                            
                            let json = if is_connected {
                                r#"{"status":"connected"}"#
                            } else {
                                r#"{"status":"pending"}"#
                            };
                            
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}", 
                                json
                            );
                            let _ = stream.write_all(response.as_bytes());
                            let _ = stream.flush();
                            continue;
                        }

                        // Handle /pair request
                        if request.contains("GET /pair") {
                            // Default to socket address
                            let mut target_ip = String::new();
                            if let Ok(addr) = stream.peer_addr() {
                                target_ip = addr.ip().to_string();
                            }

                            // Override with ?ip= parameter if present
                            if let Some(start) = request.find("?ip=") {
                                if let Some(end) = request[start..].find(' ') {
                                    let param = &request[start+4..start+end];
                                    if !param.is_empty() {
                                        target_ip = param.to_string();
                                        println!("Manual IP override: {}", target_ip);
                                    }
                                }
                            }
                            
                            println!("Pairing attempt for IP: {}", target_ip);
                            let ip_clone = target_ip.clone();
                            let display_ip = target_ip.clone();
                            let app_clone = app.clone();

                            // Build HTML Response with Same WiFi warning
                            let html_response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                                <!DOCTYPE html><html><head><meta name='viewport' content='width=device-width, initial-scale=1'>\
                                <title>NL-Mirror</title>\
                                <style>\
                                body {{ \
                                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; \
                                    background: #09090b; \
                                    color: #f4f4f5; \
                                    display: flex; \
                                    flex-direction: column; \
                                    align-items: center; \
                                    justify-content: center; \
                                    min-height: 100vh; \
                                    margin: 0; \
                                    text-align: center; \
                                    padding: 20px; \
                                    box-sizing: border-box; \
                                }}\
                                .card {{ \
                                    background: #18181b; \
                                    border: 1px solid rgba(255,255,255,0.08); \
                                    padding: 25px; \
                                    border-radius: 20px; \
                                    width: 100%; \
                                    max-width: 320px; \
                                    box-shadow: 0 10px 30px rgba(0,0,0,0.3); \
                                }}\
                                .icon-circle {{ \
                                    width: 70px; height: 70px; \
                                    background: rgba(59, 130, 246, 0.1); \
                                    border-radius: 50%; \
                                    display: flex; align-items: center; justify-content: center; \
                                    margin: 0 auto 15px; \
                                    box-shadow: 0 0 20px rgba(59, 130, 246, 0.2); \
                                }}\
                                .loading-spinner {{ \
                                    border: 3px solid rgba(59, 130, 246, 0.3); \
                                    border-top: 3px solid #3b82f6; \
                                    border-radius: 50%; \
                                    width: 35px; height: 35px; \
                                    animation: spin 1s linear infinite; \
                                }}\
                                @keyframes spin {{ 0% {{ transform: rotate(0deg); }} 100% {{ transform: rotate(360deg); }} }}\
                                h2 {{ margin: 0 0 8px; font-weight: 700; font-size: 22px; }}\
                                p {{ color: #a1a1aa; margin: 0; font-size: 14px; line-height: 1.5; }}\
                                .device-ip {{ \
                                    margin-top: 12px; \
                                    background: #27272a; \
                                    padding: 8px 12px; \
                                    border-radius: 8px; \
                                    font-family: monospace; \
                                    color: #3b82f6; \
                                    font-size: 14px; \
                                    display: inline-block; \
                                }}\
                                .warning {{ \
                                    background: rgba(251, 191, 36, 0.1); \
                                    border: 1px solid rgba(251, 191, 36, 0.3); \
                                    color: #fbbf24; \
                                    padding: 10px; \
                                    border-radius: 8px; \
                                    font-size: 12px; \
                                    margin-top: 15px; \
                                }}\
                                input {{ \
                                    background: #27272a; \
                                    border: 1px solid rgba(255,255,255,0.1); \
                                    color: white; \
                                    padding: 8px; \
                                    border-radius: 6px; \
                                    margin-top: 8px; \
                                    width: 80%; \
                                    text-align: center; \
                                }}\
                                button {{ \
                                    background: #3b82f6; \
                                    color: white; \
                                    border: none; \
                                    padding: 8px 16px; \
                                    border-radius: 6px; \
                                    margin-top: 8px; \
                                    font-weight: 600; \
                                    cursor: pointer; \
                                }}\
                                .footer {{ margin-top: 20px; font-size: 11px; color: #52525b; }}\
                                .success {{ background: rgba(16, 185, 129, 0.1); border-color: rgba(16, 185, 129, 0.3); color: #10b981; }}\
                                .success-icon {{ font-size: 40px; }}\
                                </style>\
                                <script>\
                                    function checkStatus() {{\
                                        fetch('/status').then(r => r.json()).then(data => {{\
                                            if (data.status === 'connected') {{\
                                                document.getElementById('card').innerHTML = `\
                                                    <div class='icon-circle' style='background: rgba(16, 185, 129, 0.1); box-shadow: 0 0 20px rgba(16, 185, 129, 0.2);'>\
                                                        <div class='success-icon'>✓</div>\
                                                    </div>\
                                                    <h2 style='color: #10b981'>Connected!</h2>\
                                                    <p>You can close this page now.</p>\
                                                `;\
                                                setTimeout(() => window.close(), 2000);\
                                            }} else {{\
                                                setTimeout(checkStatus, 2000);\
                                            }}\
                                        }}).catch(() => setTimeout(checkStatus, 2000));\
                                    }}\
                                    setTimeout(checkStatus, 2000);\
                                </script>\
                                </head><body>\
                                <div id='card' class='card'>\
                                    <div class='icon-circle'><div class='loading-spinner'></div></div>\
                                    <h2>Connecting...</h2>\
                                    <p>Sending connection request.</p>\
                                    <div class='device-ip'>{}</div>\
                                    \
                                    <div class='warning'>⚠️ Make sure your phone and desktop are on the <strong>same WiFi network</strong>.</div>\
                                    \
                                    <form action='/pair' method='get' style='margin-top: 20px; border-top: 1px solid rgba(255,255,255,0.1); padding-top: 15px;'>\
                                        <p style='font-size: 11px; margin-bottom: 5px; color: #a1a1aa;'>Wrong IP? Enter your phone's IP:</p>\
                                        <input type='text' name='ip' value='{}' placeholder='192.168.1.x' />\
                                        <br>\
                                        <button type='submit'>Retry</button>\
                                    </form>\
                                </div>\
                                <div class='footer'>NL-Mirror Safe Connect</div>\
                                </body></html>",
                                display_ip, display_ip
                            );
                            
                            if let Err(e) = stream.write_all(html_response.as_bytes()) {
                                eprintln!("Failed to write response: {}", e);
                            }
                            let _ = stream.flush();

                            // Trigger ADB connect in background
                            tauri::async_runtime::spawn(async move {
                                use tauri_plugin_shell::ShellExt;
                                let shell = app_clone.shell();
                                
                                println!("Attempting ADB connect to {}", ip_clone);
                                let output = shell.sidecar("adb")
                                    .expect("failed sidecar")
                                    .args(["connect", &format!("{}:5555", ip_clone)])
                                    .output()
                                    .await;

                                match output {
                                    Ok(o) => {
                                        let stdout = String::from_utf8_lossy(&o.stdout);
                                        let stderr = String::from_utf8_lossy(&o.stderr);
                                        println!("ADB Output: {} | {}", stdout.trim(), stderr.trim());

                                        // Check if actually connected (stdout contains "connected")
                                        if stdout.contains("connected") {
                                            // Record successful connection for status polling
                                            if let Ok(mut ip) = LAST_CONNECTED_IP.lock() {
                                                *ip = Some(ip_clone.clone());
                                            }
                                            let _ = app_clone.emit("pair-success", &ip_clone);
                                        } else {
                                            let msg = if stderr.is_empty() { stdout.to_string() } else { stderr.to_string() };
                                            let _ = app_clone.emit("pair-error", format!("Failed: {}", msg.trim()));
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("Failed to execute sidecar: {}", e);
                                        let _ = app_clone.emit("pair-error", format!("Execution error: {}", e));
                                    }
                                }
                            });
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Non-blocking timeout, just check the flag and continue
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("Connection failed: {}", e);
                }
            }
        }

        println!("Pairing server stopped.");
        PAIRING_SERVER_STOPPED.store(true, Ordering::SeqCst);
    });

    Ok(())
}

#[tauri::command]
pub fn stop_pairing_server() -> Result<(), String> {
    println!("Stopping pairing server...");
    PAIRING_SERVER_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}
