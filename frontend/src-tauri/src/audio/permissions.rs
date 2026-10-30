// macOS audio permissions handling
use anyhow::Result;
use log::{info, warn, error};

#[cfg(target_os = "macos")]
use std::process::Command;

/// Check if the app has Audio Capture permission (required for Core Audio taps on macOS 14.4+)
///
/// Note: Core Audio taps require NSAudioCaptureUsageDescription in Info.plist.
/// When the app first attempts to create a Core Audio tap, macOS will automatically
/// show a permission dialog to the user. If permission is denied, the tap will return
/// silence (all zeros).
///
/// This function returns true because the actual permission prompt happens automatically
/// when AudioHardwareCreateProcessTap is called by the cidre library.
#[cfg(target_os = "macos")]
pub fn check_screen_recording_permission() -> bool {
    info!("ℹ️  Core Audio tap requires Audio Capture permission (macOS 14.4+)");
    info!("📍 Permission dialog will appear automatically when recording starts");
    info!("   If already granted: System Settings → Privacy & Security → Audio Capture");

    // Always return true - the actual permission dialog is triggered by Core Audio API
    true
}

#[cfg(not(target_os = "macos"))]
pub fn check_screen_recording_permission() -> bool {
    true // Not required on other platforms
}

/// Request Audio Capture permission from the user
/// This will open System Settings to the Privacy & Security page
#[cfg(target_os = "macos")]
pub fn request_screen_recording_permission() -> Result<()> {
    info!("🔐 Opening System Settings for Audio Capture permission...");

    // Open System Settings to Privacy & Security page
    // Note: There's no direct URL for Audio Capture, so we open the main Privacy page
    let result = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security")
        .spawn();

    match result {
        Ok(_) => {
            info!("✅ Opened System Settings - navigate to Privacy & Security → Audio Capture");
            info!("👉 Please enable Audio Capture permission and restart the app");
            Ok(())
        }
        Err(e) => {
            error!("❌ Failed to open System Settings: {}", e);
            Err(anyhow::anyhow!("Failed to open System Settings: {}", e))
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn request_screen_recording_permission() -> Result<()> {
    Ok(()) // Not required on other platforms
}

/// Check and request Audio Capture permission if not granted
/// Returns true if permission is granted, false otherwise
pub fn ensure_screen_recording_permission() -> bool {
    if check_screen_recording_permission() {
        return true;
    }

    warn!("Audio Capture permission not granted - requesting...");

    if let Err(e) = request_screen_recording_permission() {
        error!("Failed to request Audio Capture permission: {}", e);
        return false;
    }

    false // Permission will be granted after restart
}

/// Tauri command to check Screen Recording permission
#[tauri::command]
pub async fn check_screen_recording_permission_command() -> bool {
    check_screen_recording_permission()
}

/// Tauri command to request Screen Recording permission
#[tauri::command]
pub async fn request_screen_recording_permission_command() -> Result<(), String> {
    request_screen_recording_permission()
        .map_err(|e| e.to_string())
}

/// Trigger system audio permission request and verify it was granted
/// Returns Ok(true) if permission granted (tap created successfully), Ok(false) if denied
#[cfg(target_os = "macos")]
pub fn trigger_system_audio_permission() -> Result<bool> {
    info!("🔐 Triggering Audio Capture permission request...");

    // Try to create a Core Audio capture - this triggers the permission dialog
    // if NSAudioCaptureUsageDescription is present in Info.plist
    // NOTE: We only create the tap, don't start streaming - similar to mic permission approach
    match crate::audio::capture::CoreAudioCapture::new() {
        Ok(_capture) => {
            info!("✅ Core Audio tap created successfully");
            // Sleep briefly to allow permission dialog to appear (if shown)
            // Similar to microphone permission handling in discovery.rs
            std::thread::sleep(std::time::Duration::from_millis(500));
            info!("✅ Audio Capture permission appears to be granted");
            // Note: On macOS, even with permission denied, tap creation may succeed
            // but audio will be silence. For onboarding, we just check tap creation.
            Ok(true)
        }
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("permission") || error_msg.contains("denied") {
                info!("🔐 Audio Capture permission denied");
                info!("👉 Please grant Audio Capture permission in System Settings");
                return Ok(false);
            }
            warn!("⚠️ Failed to create Core Audio tap: {}", e);
            // If tap creation fails for other reasons, still return false
            // as we can't verify permission status
            Ok(false)
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn trigger_system_audio_permission() -> Result<bool> {
    // System audio permissions not required on other platforms
    info!("System audio permissions not required on this platform");
    Ok(true)
}

/// Tauri command to trigger system audio permission request
/// Returns true if permission was granted (stream created), false if denied
#[tauri::command]
pub async fn trigger_system_audio_permission_command() -> Result<bool, String> {
    // Run in blocking task to avoid blocking the async runtime
    tokio::task::spawn_blocking(|| {
        trigger_system_audio_permission()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| e.to_string())
}

/// Real state of the macOS permissions Dixit needs (Settings → Generali → Permessi).
/// Values match the frontend `PermissionStatus`: "authorized", "denied", "not_determined".
#[derive(serde::Serialize)]
pub struct PermissionsStatus {
    microphone: &'static str,
    system_audio: &'static str,
    accessibility: &'static str,
}

#[cfg(target_os = "macos")]
fn microphone_status() -> &'static str {
    use cidre::av;
    match av::capture::Device::authorization_status_for_media_type(av::MediaType::audio()) {
        Ok(av::AuthorizationStatus::Authorized) => "authorized",
        Ok(av::AuthorizationStatus::NotDetermined) => "not_determined",
        _ => "denied",
    }
}

/// System audio (Core Audio tap) has no public status API: ask TCC through its private
/// preflight, as the open-source AudioCap sample does. 0 = granted, 1 = denied, else not asked yet.
// ponytail: private SPI; if Apple drops it the row reads "not asked" and its button re-requests.
#[cfg(target_os = "macos")]
fn system_audio_status() -> &'static str {
    use std::ffi::{c_char, c_int, c_void};
    extern "C" {
        fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    const RTLD_NOW: c_int = 2;
    type Preflight = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

    let service = cidre::cf::String::from_str("kTCCServiceAudioCapture");
    unsafe {
        let tcc = dlopen(c"/System/Library/PrivateFrameworks/TCC.framework/Versions/A/TCC".as_ptr(), RTLD_NOW);
        let sym = if tcc.is_null() { tcc } else { dlsym(tcc, c"TCCAccessPreflight".as_ptr()) };
        if sym.is_null() {
            return "not_determined";
        }
        let preflight: Preflight = std::mem::transmute(sym);
        match preflight(&*service as *const cidre::cf::String as *const c_void, std::ptr::null()) {
            0 => "authorized",
            1 => "denied",
            _ => "not_determined",
        }
    }
}

#[tauri::command]
pub async fn permissions_status() -> PermissionsStatus {
    #[cfg(target_os = "macos")]
    return PermissionsStatus {
        microphone: microphone_status(),
        system_audio: system_audio_status(),
        // Not trusted can't be told apart from "never asked": both need System Settings.
        accessibility: if crate::ax_util::is_trusted(false) { "authorized" } else { "denied" },
    };
    #[cfg(not(target_os = "macos"))]
    PermissionsStatus { microphone: "authorized", system_audio: "authorized", accessibility: "authorized" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_permission() {
        let has_permission = check_screen_recording_permission();
        println!("Has Screen Recording permission: {}", has_permission);
    }

    /// Calls the real APIs (TCC SPI included): must not crash and must speak the frontend's values.
    #[tokio::test]
    async fn permissions_status_uses_known_values() {
        let s = permissions_status().await;
        for v in [s.microphone, s.system_audio, s.accessibility] {
            assert!(["authorized", "denied", "not_determined"].contains(&v), "{v}");
        }
    }
}