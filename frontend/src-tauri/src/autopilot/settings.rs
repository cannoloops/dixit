use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "trascrivi.json";
const STORE_KEY: &str = "autopilot";

/// User-facing Trascrivi settings (Settings → Automazione).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutopilotSettings {
    pub auto_start: bool,
    pub native_teams_transcription: bool,
    /// "Cognome Nome" as shown in Teams; label for the microphone speaker.
    pub user_display_name: String,
    // ponytail: calibration knobs for the call tracker, tuned on real calls.
    pub start_after_secs: u64,
    pub stop_after_secs: u64,
    /// Local MCP server for agents (Settings → Automazione → Agenti).
    pub mcp_enabled: bool,
}

impl Default for AutopilotSettings {
    fn default() -> Self {
        Self {
            auto_start: true,
            native_teams_transcription: true,
            user_display_name: String::new(),
            start_after_secs: 5,
            stop_after_secs: 45,
            mcp_enabled: false,
        }
    }
}

impl AutopilotSettings {
    /// Speaker label for the microphone.
    pub fn me(&self) -> String {
        let name = self.user_display_name.trim();
        if name.is_empty() { "Io".to_string() } else { name.to_string() }
    }
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> AutopilotSettings {
    app.store(STORE_FILE)
        .ok()
        .and_then(|store| store.get(STORE_KEY))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn save<R: Runtime>(app: &AppHandle<R>, settings: &AutopilotSettings) -> anyhow::Result<()> {
    let store = app.store(STORE_FILE)?;
    store.set(STORE_KEY, serde_json::to_value(settings)?);
    store.save()?;
    Ok(())
}

#[tauri::command]
pub async fn autopilot_get_settings<R: Runtime>(app: AppHandle<R>) -> AutopilotSettings {
    load(&app)
}

#[tauri::command]
pub async fn autopilot_save_settings<R: Runtime>(
    app: AppHandle<R>,
    settings: AutopilotSettings,
) -> Result<(), String> {
    save(&app, &settings).map_err(|e| e.to_string())?;
    crate::mcp::apply(settings.mcp_enabled).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn me_falls_back_to_io() {
        let mut s = AutopilotSettings::default();
        assert_eq!(s.me(), "Io");
        s.user_display_name = "   ".into();
        assert_eq!(s.me(), "Io");
        s.user_display_name = " Manuguerra Vito ".into();
        assert_eq!(s.me(), "Manuguerra Vito");
    }

    #[test]
    fn deserializes_partial_json() {
        let s: AutopilotSettings = serde_json::from_str(r#"{"auto_start":false}"#).unwrap();
        assert!(!s.auto_start);
        assert!(s.native_teams_transcription);
        assert_eq!(s.user_display_name, "");
        assert_eq!(s.start_after_secs, 5);
        assert_eq!(s.stop_after_secs, 45);
        assert!(!s.mcp_enabled);
    }
}
