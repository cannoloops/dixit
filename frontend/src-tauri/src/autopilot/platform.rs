//! Pure classifiers: which app a Core Audio client belongs to, and what a window title says.

pub use super::tracker::Platform;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppKind {
    Teams,
    Zoom,
    /// A browser; the payload is the main app's bundle id (to read its window titles).
    Browser(&'static str),
}

const BROWSERS: &[&str] = &[
    "com.google.Chrome",
    "ai.perplexity.comet",
    "com.operasoftware.Opera",
    "com.microsoft.edgemac",
    "com.brave.Browser",
    "company.thebrowser.Browser",
    "com.apple.Safari",
];

/// Main-app bundle ids of the supported browsers.
pub fn browser_bundles() -> &'static [&'static str] {
    BROWSERS
}

pub fn platform_for_bundle(bundle_id: &str) -> Option<AppKind> {
    if bundle_id.starts_with("com.microsoft.teams2") {
        let noise = [".notificationcenter", ".widgetextension", ".teamsswitcher"];
        return (!noise.iter().any(|n| bundle_id.contains(n))).then_some(AppKind::Teams);
    }
    if bundle_id.starts_with("us.zoom.") {
        return Some(AppKind::Zoom);
    }
    // Safari captures the mic from WebKit helper processes.
    if bundle_id.starts_with("com.apple.WebKit.") {
        return Some(AppKind::Browser("com.apple.Safari"));
    }
    BROWSERS
        .iter()
        .find(|b| bundle_id == **b || bundle_id.starts_with(&format!("{b}.")))
        .map(|b| AppKind::Browser(b))
}

fn after_meet_dash(title: &str) -> Option<&str> {
    let rest = title.strip_prefix("Meet")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('-').or_else(|| rest.strip_prefix('–'))?;
    Some(rest.trim())
}

pub fn is_meet_window(title: &str) -> bool {
    after_meet_dash(title).is_some() || title.contains("meet.google.com")
}

pub fn is_zoom_meeting_window(title: &str) -> bool {
    ["Zoom Meeting", "Riunione Zoom", "Zoom Webinar", "Webinar Zoom"]
        .iter()
        .any(|t| title.contains(t))
}

/// Which meeting platform holds the mic. A browser counts as Meet when a Meet window is the
/// active tab, or when a Meet session is already running (the user may have switched tab).
pub fn mic_platform(users: &[AppKind], meet_window: bool, session: Option<Platform>) -> Option<Platform> {
    if users.contains(&AppKind::Teams) {
        Some(Platform::Teams)
    } else if users.contains(&AppKind::Zoom) {
        Some(Platform::Zoom)
    } else if users.iter().any(|k| matches!(k, AppKind::Browser(_)))
        && (meet_window || session == Some(Platform::Meet))
    {
        Some(Platform::Meet)
    } else {
        None
    }
}

/// Meeting title from a window title, or None when the title is generic.
pub fn meeting_title(platform: Platform, window_title: &str) -> Option<String> {
    let title = match platform {
        Platform::Teams => {
            // The compact meeting view prefixes its own name: the meeting comes next.
            const VIEWS: &[&str] = &["Visualizzazione compatta delle riunioni", "Compact meeting view"];
            let mut parts = window_title.split(" | ").map(str::trim);
            let mut first = parts.next()?;
            if VIEWS.iter().any(|v| first.eq_ignore_ascii_case(v)) {
                first = parts.next()?;
            }
            const GENERIC: &[&str] = &[
                "Microsoft Teams", "Chat", "Calendario", "Calendar", "Attività", "Activity",
                "Teams", "Team", "Chiamate", "Calls", "OneDrive", "App", "Apps",
            ];
            if GENERIC.iter().any(|g| first.eq_ignore_ascii_case(g)) {
                return None;
            }
            first
        }
        Platform::Meet => after_meet_dash(window_title)?,
        Platform::Zoom => return None,
    };
    (!title.is_empty()).then(|| title.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_bundles() {
        assert_eq!(platform_for_bundle("com.microsoft.teams2"), Some(AppKind::Teams));
        assert_eq!(platform_for_bundle("com.microsoft.teams2.modulehost"), Some(AppKind::Teams));
        assert_eq!(platform_for_bundle("com.microsoft.teams2.notificationcenter"), None);
        assert_eq!(platform_for_bundle("com.microsoft.teams2.widgetextension"), None);
        assert_eq!(platform_for_bundle("us.zoom.xos"), Some(AppKind::Zoom));
        assert_eq!(platform_for_bundle("com.google.Chrome.helper"), Some(AppKind::Browser("com.google.Chrome")));
        assert_eq!(platform_for_bundle("com.apple.WebKit.GPU"), Some(AppKind::Browser("com.apple.Safari")));
        assert_eq!(platform_for_bundle("com.apple.Music"), None);
    }

    #[test]
    fn teams_titles() {
        assert_eq!(meeting_title(Platform::Teams, "Sprint review | Microsoft Teams").as_deref(), Some("Sprint review"));
        assert_eq!(
            meeting_title(Platform::Teams, "Riunione con Rossi Mario | Dixit | Microsoft Teams").as_deref(),
            Some("Riunione con Rossi Mario")
        );
        assert_eq!(meeting_title(Platform::Teams, "Chat | Microsoft Teams"), None);
        assert_eq!(meeting_title(Platform::Teams, "Calendario | Microsoft Teams"), None);
        assert_eq!(meeting_title(Platform::Teams, "Microsoft Teams"), None);
        // New Teams compact meeting view (seen live): view name first, then the meeting.
        assert_eq!(
            meeting_title(
                Platform::Teams,
                "Visualizzazione compatta delle riunioni | Formazione Beta | Acme S.p.A. | mario@example.com"
            )
            .as_deref(),
            Some("Formazione Beta")
        );
        assert_eq!(
            meeting_title(Platform::Teams, "Compact meeting view | Daily | Contoso | a@b.c").as_deref(),
            Some("Daily")
        );
    }

    #[test]
    fn meet_and_zoom_titles() {
        assert_eq!(meeting_title(Platform::Meet, "Meet – Daily standup").as_deref(), Some("Daily standup"));
        assert_eq!(meeting_title(Platform::Meet, "Meet - abc-defg-hij").as_deref(), Some("abc-defg-hij"));
        assert_eq!(meeting_title(Platform::Zoom, "Zoom Meeting"), None);
        assert!(is_meet_window("Meet - abc-defg-hij"));
        assert!(!is_meet_window("Meeting notes - Google Docs"));
        assert!(is_meet_window("https://meet.google.com/abc-defg-hij"));
    }

    #[test]
    fn zoom_meeting_windows() {
        assert!(is_zoom_meeting_window("Zoom Meeting"));
        assert!(is_zoom_meeting_window("Riunione Zoom"));
        assert!(is_zoom_meeting_window("Zoom Webinar"));
        assert!(!is_zoom_meeting_window("Zoom Workplace"));
    }

    #[test]
    fn mic_platform_rules() {
        let browser = AppKind::Browser("com.google.Chrome");
        assert_eq!(mic_platform(&[browser, AppKind::Teams], false, None), Some(Platform::Teams));
        assert_eq!(mic_platform(&[AppKind::Zoom], false, None), Some(Platform::Zoom));
        assert_eq!(mic_platform(&[browser], true, None), Some(Platform::Meet));
        assert_eq!(mic_platform(&[browser], false, None), None);
        // Meet session running, user switched to another tab: still Meet.
        assert_eq!(mic_platform(&[browser], false, Some(Platform::Meet)), Some(Platform::Meet));
        assert_eq!(mic_platform(&[], false, Some(Platform::Meet)), None);
    }
}
