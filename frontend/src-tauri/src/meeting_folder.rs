//! A deleted meeting's folder on disk (audio, transcript, notes): moved to the Trash on request,
//! never removed for good, and only when it is unmistakably that meeting's own folder.

use std::path::{Path, PathBuf};

/// Why a folder was left where it is: the meeting is gone from the archive either way.
#[derive(Debug, PartialEq, Eq)]
pub enum Kept {
    /// The meeting has no folder, or it is no longer on disk.
    Missing,
    /// Not strictly inside a recordings folder (e.g. the save folder changed since): too risky.
    OutsideRecordings,
    /// Another meeting still points at the same folder.
    Shared,
}

/// The folder to move to the Trash, canonicalised, or why it stays. `roots` are the recordings
/// folders (current and default save folder); the folder must be strictly inside one of them.
pub fn trashable(folder: Option<&str>, roots: &[PathBuf], shared: bool) -> Result<PathBuf, Kept> {
    let folder = folder.map(str::trim).filter(|f| !f.is_empty()).ok_or(Kept::Missing)?;
    let folder = Path::new(folder).canonicalize().map_err(|_| Kept::Missing)?;
    if !folder.is_dir() {
        return Err(Kept::Missing);
    }
    let inside = roots
        .iter()
        .filter_map(|r| r.canonicalize().ok())
        .any(|root| folder != root && folder.starts_with(&root));
    if !inside {
        return Err(Kept::OutsideRecordings);
    }
    if shared {
        return Err(Kept::Shared);
    }
    Ok(folder)
}

/// Moves `path` to the user's Trash (Finder can put it back).
#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)] // objc 0.2's msg_send! checks a `cargo-clippy` feature
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    use objc::runtime::{Object, BOOL, NO};
    use objc::{class, msg_send, sel, sel_impl};
    use std::ffi::{CStr, CString};

    let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
    objc::rc::autoreleasepool(|| unsafe {
        let ns_path: *mut Object = msg_send![class!(NSString), stringWithUTF8String: c_path.as_ptr()];
        let url: *mut Object = msg_send![class!(NSURL), fileURLWithPath: ns_path];
        let manager: *mut Object = msg_send![class!(NSFileManager), defaultManager];
        let mut error: *mut Object = std::ptr::null_mut();
        let ok: BOOL = msg_send![manager, trashItemAtURL: url
                                          resultingItemURL: std::ptr::null_mut::<*mut Object>()
                                          error: &mut error];
        if ok != NO {
            return Ok(());
        }
        if error.is_null() {
            return Err("the Trash refused the folder".to_string());
        }
        let description: *mut Object = msg_send![error, localizedDescription];
        let text: *const std::os::raw::c_char = msg_send![description, UTF8String];
        Err(if text.is_null() {
            "the Trash refused the folder".to_string()
        } else {
            CStr::from_ptr(text).to_string_lossy().into_owned()
        })
    })
}

// ponytail: Dixit ships for macOS only; elsewhere the folder stays and the user is told so.
#[cfg(not(target_os = "macos"))]
pub fn move_to_trash(_path: &Path) -> Result<(), String> {
    Err("moving to the Trash is only supported on macOS".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_meeting_folder_inside_the_recordings_goes() {
        let root = tempfile::tempdir().unwrap();
        let meeting = root.path().join("2026-11-18 14.31 Daily");
        std::fs::create_dir(&meeting).unwrap();
        let roots = vec![root.path().to_path_buf()];
        let meeting_str = meeting.to_string_lossy().to_string();

        assert_eq!(trashable(Some(&meeting_str), &roots, false), Ok(meeting.canonicalize().unwrap()));
        // Another meeting still uses it.
        assert_eq!(trashable(Some(&meeting_str), &roots, true), Err(Kept::Shared));
        // The recordings folder itself, or anything outside it, never goes.
        let root_str = root.path().to_string_lossy().to_string();
        assert_eq!(trashable(Some(&root_str), &roots, false), Err(Kept::OutsideRecordings));
        let elsewhere = tempfile::tempdir().unwrap();
        let elsewhere_str = elsewhere.path().to_string_lossy().to_string();
        assert_eq!(trashable(Some(&elsewhere_str), &roots, false), Err(Kept::OutsideRecordings));
        // "../" cannot climb out of the recordings folder.
        let climbing = format!("{}/../{}", meeting_str, elsewhere.path().file_name().unwrap().to_string_lossy());
        assert_eq!(trashable(Some(&climbing), &roots, false), Err(Kept::Missing));
        let climbing_out = format!("{}/..", meeting_str);
        assert_eq!(trashable(Some(&climbing_out), &roots, false), Err(Kept::OutsideRecordings));
        // No folder, or gone already.
        assert_eq!(trashable(None, &roots, false), Err(Kept::Missing));
        assert_eq!(trashable(Some("  "), &roots, false), Err(Kept::Missing));
        assert_eq!(trashable(Some(&format!("{meeting_str}-gone")), &roots, false), Err(Kept::Missing));
        // A file is not a meeting folder.
        let file = root.path().join("note.txt");
        std::fs::write(&file, "x").unwrap();
        assert_eq!(trashable(Some(&file.to_string_lossy()), &roots, false), Err(Kept::Missing));
    }

    /// Puts an empty "dixit-trash-check-…" folder in this Mac's Trash:
    /// `cargo test --lib trash_for_real -- --ignored`.
    #[test]
    #[ignore]
    #[cfg(target_os = "macos")]
    fn trash_for_real() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join(format!("dixit-trash-check-{}", std::process::id()));
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("audio.mp4"), b"x").unwrap();
        move_to_trash(&folder).unwrap();
        assert!(!folder.exists());
        assert!(move_to_trash(&folder).is_err(), "a missing folder must report an error");
    }
}
