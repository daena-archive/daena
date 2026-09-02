pub fn current() -> &'static str {
    env!("DAENA_APP_VERSION")
}

pub fn normalize_override(raw: &str) -> Option<&str> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_strips_prefix_and_whitespace() {
        assert_eq!(
            normalize_override(" v0.1.0-alpha.2 "),
            Some("0.1.0-alpha.2")
        );
        assert_eq!(normalize_override("0.2.0"), Some("0.2.0"));
        assert_eq!(normalize_override("  "), None);
        assert_eq!(normalize_override("v"), None);
    }

    #[test]
    fn compiled_version_is_semver() {
        assert!(
            crate::app_update::parse_version(current()).is_some(),
            "DAENA_APP_VERSION must be semver, got {}",
            current()
        );
    }
}
