//! Dixit: models shipped in the .app (`Contents/Resources/models`): only older full packages have them,
//! new ones download them at first launch (`base_models.rs`).
//! Same layout as the user's models dir (`<app_data>/models`): a path is looked up there first,
//! then at the same relative path in the bundle. Bundled files are read in place, never copied.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// (user models root, bundled models root)
static ROOTS: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();

pub fn init<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    use tauri::Manager;
    let (Ok(data), Ok(res)) = (app.path().app_data_dir(), app.path().resource_dir()) else {
        log::warn!("Bundled models: app data or resource dir unavailable");
        return;
    };
    let roots = (data.join("models"), res.join("models"));
    log::info!("Bundled models dir: {}", roots.1.display());
    let _ = ROOTS.set(roots);
}

/// `path` (under the user models dir) if it exists, else its bundled copy if that exists, else `path`.
pub fn resolve(path: &Path) -> PathBuf {
    match ROOTS.get() {
        Some((user, bundle)) => resolve_in(user, bundle, path),
        None => path.to_path_buf(),
    }
}

/// Whether `path` points inside the bundle (read-only: never delete or overwrite it).
pub fn is_bundled(path: &Path) -> bool {
    ROOTS.get().is_some_and(|(_, bundle)| path.starts_with(bundle))
}

fn resolve_in(user_root: &Path, bundle_root: &Path, path: &Path) -> PathBuf {
    if path.exists() {
        return path.to_path_buf();
    }
    match path.strip_prefix(user_root) {
        Ok(rel) if bundle_root.join(rel).exists() => bundle_root.join(rel),
        _ => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_in;
    use std::fs;

    #[test]
    fn user_dir_first_then_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let (user, bundle) = (tmp.path().join("user"), tmp.path().join("bundle"));
        for dir in [&user, &bundle] {
            fs::create_dir_all(dir.join("summary")).unwrap();
        }
        let rel = "summary/model.gguf";
        let user_path = user.join(rel);

        // Nowhere: the user path, where a download would land.
        assert_eq!(resolve_in(&user, &bundle, &user_path), user_path);

        // Only in the bundle: the bundled copy.
        fs::write(bundle.join(rel), b"b").unwrap();
        assert_eq!(resolve_in(&user, &bundle, &user_path), bundle.join(rel));

        // Also in the user dir: the user's copy wins.
        fs::write(&user_path, b"u").unwrap();
        assert_eq!(resolve_in(&user, &bundle, &user_path), user_path);

        // Directories (Parakeet) work the same way.
        let dir = user.join("parakeet/v3");
        fs::create_dir_all(bundle.join("parakeet/v3")).unwrap();
        assert_eq!(resolve_in(&user, &bundle, &dir), bundle.join("parakeet/v3"));

        // Paths outside the user models dir are left alone.
        let other = tmp.path().join("elsewhere/x");
        assert_eq!(resolve_in(&user, &bundle, &other), other);
    }
}
