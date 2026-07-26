//! Functions to manage the main windows involved in Harper Desktop

use std::sync::Arc;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;
use tracing::error;

use crate::config::Config;

/// Window event handler implementing "keep running on close": when enabled,
/// closing a window only hides it so the app and the highlighter service stay
/// alive in the background (reachable again via dock click or the tray).
fn keep_alive_on_close<R: tauri::Runtime>(window: &WebviewWindow<R>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    let keep_running = window
        .app_handle()
        .try_state::<Arc<Mutex<Config>>>()
        .and_then(|state| state.try_lock().ok().map(|c| c.keep_running_on_close))
        // Fail open: keeping the app alive is the safer default.
        .unwrap_or(true);

    if keep_running {
        api.prevent_close();
        let _ = window.hide();
    }
}

/// Open the editor window, focusing it if it already exists.
pub fn show_editor_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("editor") {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, "editor", WebviewUrl::App("index.html".into()))
        .title("Harper")
        .inner_size(800.0, 600.0)
        .build()?;
    {
        let window_handle = window.clone();
        window.on_window_event(move |event| keep_alive_on_close(&window_handle, event));
    }
    window.set_focus()?;

    Ok(())
}

/// Open the settings window, focusing it if it already exists.
pub fn show_settings_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("Harper Settings")
        .inner_size(920.0, 680.0)
        .min_inner_size(780.0, 520.0)
        .center()
        .build()?;
    {
        let window_handle = window.clone();
        window.on_window_event(move |event| keep_alive_on_close(&window_handle, event));
    }

    Ok(())
}

/// Open the browser to an issue report page.
pub fn open_issue_report(app: &AppHandle) {
    let _ = app
        .opener()
        .open_url(
            "https://github.com/Automattic/harper/issues/new/choose",
            None::<&str>,
        )
        .inspect_err(|err| error!("failed to open issue report URL: {err}"));
}
