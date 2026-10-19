//! Release versions and the few fields we read out of a GitHub release
//! document.
//!
//! Everything here is pure string handling so it can be unit tested on any
//! platform; the WinHTTP transport lives in [`crate::update`].

/// A release version as `major.minor.patch`.
pub type Version = (u64, u64, u64);

/// Parses a release tag into a comparable version.
///
/// Accepts the shapes GitHub tags may have: `v21.0.0`, `21.0.0`, `21.0`, `21`
/// and prerelease/build suffixes such as `v21.0.1-beta.1`.
pub fn parse(tag: &str) -> Option<Version> {
    let trimmed = tag.trim();
    let number_part = trimmed
        .trim_start_matches('v')
        .split(['-', '+'])
        .next()
        .unwrap_or("");

    let mut parts = number_part.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

/// `true` when `latest` is a strictly newer version than `current`.
///
/// Anything that does not parse as a version is never considered an update,
/// so a malformed feed can never trigger a bogus upgrade prompt.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse(latest), parse(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

/// The `tag_name` of a GitHub release document, e.g. `"v21.0.0"`.
pub fn tag_name(json: &str) -> Option<&str> {
    string_field(json, "tag_name")
}

/// The `html_url` of a GitHub release document, e.g. the release page.
pub fn html_url(json: &str) -> Option<&str> {
    string_field(json, "html_url")
}

/// Minimal reader for a flat `"key": "value"` pair in a JSON document.
///
/// The GitHub release document nests nothing interesting inside the keys we
/// read, so a real JSON parser would only add dependencies to a tool whose
/// whole point is to stay a single small executable.
fn string_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let start = json.find(&needle)? + needle.len();
    let rest = json[start..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let value = rest.strip_prefix('"')?;
    let end = value.find('"')?;
    Some(&value[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_usual_tag_shapes() {
        assert_eq!(parse("v21.0.0"), Some((21, 0, 0)));
        assert_eq!(parse("21.0.0"), Some((21, 0, 0)));
        assert_eq!(parse("21.0"), Some((21, 0, 0)));
        assert_eq!(parse("21"), Some((21, 0, 0)));
        assert_eq!(parse("  v21.2.3  "), Some((21, 2, 3)));
    }

    #[test]
    fn prerelease_suffixes_are_ignored() {
        assert_eq!(parse("v21.0.1-beta.1"), Some((21, 0, 1)));
        assert_eq!(parse("21.0.1+build5"), Some((21, 0, 1)));
    }

    #[test]
    fn rubbish_is_not_a_version() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("latest"), None);
        assert_eq!(parse("v21.x.0"), None);
    }

    #[test]
    fn only_a_higher_version_is_an_update() {
        assert!(is_newer("v21.0.1", "21.0.0"));
        assert!(is_newer("v22", "21.0.0"));
        assert!(is_newer("v21.1.0", "21.0.9"));
        assert!(!is_newer("v21.0.0", "21.0.0"));
        assert!(!is_newer("v20.9.9", "21.0.0"));
        assert!(!is_newer("v21.0.0-rc.1", "21.0.0"));
    }

    #[test]
    fn a_broken_feed_never_reports_an_update() {
        assert!(!is_newer("", "21.0.0"));
        assert!(!is_newer("maintenance", "21.0.0"));
    }

    /// A trimmed copy of what `GET /releases/latest` returns.
    const SAMPLE: &str = r#"{
      "url": "https://api.github.com/repos/anhkhoakz/idm-let-me-try-more/releases/1",
      "html_url": "https://github.com/anhkhoakz/idm-let-me-try-more/releases/tag/v22.1.0",
      "tag_name": "v22.1.0",
      "name": "idm-trial-reset v22.1.0",
      "draft": false,
      "prerelease": false,
      "assets": [
        {
          "url": "https://api.github.com/repos/anhkhoakz/idm-let-me-try-more/releases/assets/7",
          "name": "idm-trial-reset-v22.1.0-windows-x64.zip",
          "browser_download_url": "https://github.com/anhkhoakz/idm-let-me-try-more/releases/download/v22.1.0/idm-trial-reset-v22.1.0-windows-x64.zip"
        }
      ]
    }"#;

    #[test]
    fn reads_the_tag_name_out_of_a_release_document() {
        assert_eq!(tag_name(SAMPLE), Some("v22.1.0"));
    }

    #[test]
    fn reads_the_html_url_out_of_a_release_document() {
        assert_eq!(
            html_url(SAMPLE),
            Some("https://github.com/anhkhoakz/idm-let-me-try-more/releases/tag/v22.1.0")
        );
    }

    #[test]
    fn missing_fields_are_none() {
        assert_eq!(tag_name("{}"), None);
        assert_eq!(tag_name("Not Found"), None);
        assert_eq!(html_url(r#"{"tag_name": "v1.0.0"}"#), None);
    }

    #[test]
    fn the_api_error_document_is_not_a_release() {
        // What GitHub answers with when the repository has no release yet.
        let body =
            r#"{"message": "Not Found", "documentation_url": "https://docs.github.com/rest"}"#;
        assert_eq!(tag_name(body), None);
    }
}
