// Trascrivi: post-call speaker attribution (who said what).
pub mod tracks;
pub mod wav;

/// File-name-safe meeting title: no path/shell-hostile characters, single spaces, max 120 chars.
pub fn sanitize_title_for_file(title: &str) -> String {
    let replaced: String = title
        .chars()
        .map(|c| if "/\\:*?\"<>|".contains(c) { '-' } else { c })
        .collect();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(120).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_title_for_file_cases() {
        assert_eq!(sanitize_title_for_file("A/B: c | d"), "A-B- c - d");
        assert_eq!(sanitize_title_for_file("  Riunione   lunga  "), "Riunione lunga");
        assert_eq!(sanitize_title_for_file(&"x".repeat(300)).chars().count(), 120);
    }
}
