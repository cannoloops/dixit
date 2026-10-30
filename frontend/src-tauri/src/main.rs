#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use log;
use env_logger;

fn main() {
    // Keep an explicit RUST_LOG (e.g. `RUST_LOG=app_lib=debug pnpm run tauri:dev`).
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    let mut logger = env_logger::Builder::from_default_env();
    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    if let Some(file) = log_file() {
        logger.target(env_logger::Target::Pipe(Box::new(file)));
    }
    logger.init();

    // Async logger will be initialized lazily when first needed (after Tauri runtime starts)
    log::info!("Starting application...");
    app_lib::run();
}

/// Dixit opened from Finder has no terminal: its log goes to ~/Library/Logs/Dixit/dixit.log
/// (appended; past 20 MB the previous one becomes dixit.log.1). A hand-made path because the
/// logger starts before Tauri and its path API.
// ponytail: rotated at startup only; an app left running for weeks can outgrow it.
#[cfg(all(target_os = "macos", not(debug_assertions)))]
fn log_file() -> Option<std::fs::File> {
    let dir = std::path::PathBuf::from(std::env::var_os("HOME")?).join("Library/Logs/Dixit");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("dixit.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 20 * 1024 * 1024) {
        let _ = std::fs::rename(&path, dir.join("dixit.log.1"));
    }
    std::fs::OpenOptions::new().create(true).append(true).open(path).ok()
}
