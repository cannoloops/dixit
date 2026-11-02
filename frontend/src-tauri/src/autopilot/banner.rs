//! Small always-on-top banner shown when a call recording starts, with "Annulla" and, for Teams,
//! the explicit offer to start Teams' own transcription (visible to everyone).

use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "banner";
const WIDTH: f64 = 380.0;
const HEIGHT: f64 = 96.0;
/// One more line: the Teams transcription offer and its warning.
const HEIGHT_TEAMS: f64 = 124.0;
// ponytail: fixed lifetime; long enough to react, short enough not to linger over the call.
const LIFETIME: Duration = Duration::from_secs(20);

pub fn show<R: Runtime>(app: &AppHandle<R>, title: &str, offer_teams: bool) {
    close(app);
    let query: String = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("title", title)
        .append_pair("teams", if offer_teams { "1" } else { "0" })
        .finish();
    let height = if offer_teams { HEIGHT_TEAMS } else { HEIGHT };
    let (x, y) = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let size = m.size().to_logical::<f64>(m.scale_factor());
            (size.width - WIDTH - 16.0, 40.0)
        })
        .unwrap_or((40.0, 40.0));

    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(format!("banner.html?{query}").into()))
        .title("Dixit")
        .inner_size(WIDTH, height)
        .position(x, y)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .focused(false)
        .skip_taskbar(true)
        .transparent(true)
        .visible_on_all_workspaces(true)
        .build();
    match built {
        Ok(_) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(LIFETIME).await;
                close(&app);
            });
        }
        Err(e) => log::warn!("Auto-start banner failed: {e}"),
    }
}

pub fn close<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
}

#[tauri::command]
pub async fn autopilot_banner_close<R: Runtime>(app: AppHandle<R>) {
    close(&app);
}
