use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

pub fn init_tray(app: &AppHandle) -> tauri::Result<tauri::tray::TrayIcon> {
    let show_i = MenuItem::with_id(app, "show", "🖥️ Show Launcher", true, None::<&str>)?;
    let stop_i = MenuItem::with_id(app, "stop", "🛑 Stop Mirroring", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "❌ Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &stop_i, &quit_i])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "stop" => {
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_clone.state::<crate::launcher::MirrorState>();
                        let _ = crate::launcher::stop_mirror(app_clone.clone(), state).await;
                    });
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    Ok(builder.build(app)?)
}
