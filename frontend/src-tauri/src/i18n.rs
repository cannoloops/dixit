//! UI language for the strings the Rust core shows to the user (menu bar, notifications).
//! Italian when the first preferred macOS language is Italian, English otherwise.

use std::sync::OnceLock;

/// True if the user's preferred language is Italian. Detected once, then cached.
pub fn it() -> bool {
    static IT: OnceLock<bool> = OnceLock::new();
    *IT.get_or_init(|| preferred_language().is_some_and(|l| is_italian(&l)))
}

/// The Italian or the English variant, depending on [`it`].
pub fn tr<'a>(it_text: &'a str, en: &'a str) -> &'a str {
    pick(it(), it_text, en)
}

fn pick<'a>(it: bool, a: &'a str, b: &'a str) -> &'a str {
    if it { a } else { b }
}

/// "it", "it-IT", "it_CH", "it_IT.UTF-8" yes; "en-US", "item" no.
fn is_italian(lang: &str) -> bool {
    lang.strip_prefix("it")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['-', '_', '.', '@']))
}

/// First entry of the `defaults read -g AppleLanguages` plist array:
/// `(\n    "it-IT",\n    "en-IT"\n)`.
#[cfg(any(target_os = "macos", test))]
fn first_apple_language(defaults_output: &str) -> Option<&str> {
    defaults_output
        .lines()
        .map(|l| l.trim().trim_end_matches(',').trim_matches('"'))
        .find(|l| !l.is_empty() && *l != "(" && *l != ")")
}

// cidre (a9587fa) only exposes the current locale's language, which follows the app bundle's
// localizations, not the user's preference order: read AppleLanguages directly instead.
#[cfg(target_os = "macos")]
fn preferred_language() -> Option<String> {
    let out = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        .ok()?;
    first_apple_language(std::str::from_utf8(&out.stdout).ok()?).map(str::to_owned)
}

#[cfg(not(target_os = "macos"))]
fn preferred_language() -> Option<String> {
    std::env::var("LANG").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn italian_variants_are_recognised() {
        for l in ["it", "it-IT", "it_CH", "it_IT.UTF-8"] {
            assert!(is_italian(l), "{l}");
        }
    }

    #[test]
    fn other_languages_are_not_italian() {
        for l in ["en-US", "en", "item", "iti", "de-IT", "", "fr_IT"] {
            assert!(!is_italian(l), "{l}");
        }
    }

    #[test]
    fn pick_follows_the_flag() {
        assert_eq!(pick(true, "ciao", "hello"), "ciao");
        assert_eq!(pick(false, "ciao", "hello"), "hello");
    }

    #[test]
    fn first_language_is_parsed_from_defaults_output() {
        assert_eq!(first_apple_language("(\n    \"it-IT\",\n    \"en-IT\"\n)\n"), Some("it-IT"));
        assert_eq!(first_apple_language("(\n    en\n)\n"), Some("en"));
        assert_eq!(first_apple_language("(\n)\n"), None);
        assert_eq!(first_apple_language(""), None);
    }
}
