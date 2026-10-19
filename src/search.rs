//! Locating the dynamic CLSID key in the output of `reg query <root> /s`.
//!
//! This is the port of AutoIt's `RegSearch`, minus the `reg query | findstr`
//! shell pipeline: the whole dump is scanned in memory instead.

/// The value the original script searched for with `findstr /I`.
pub const NEEDLE: &str = "cDTvBFquXk0";

/// Returns the GUID of the key that owns [`NEEDLE`].
///
/// The needle is either part of a key path itself or of one of its values, in
/// which case the owning key path is the next line above it.
pub fn guid_in_query_output(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let needle = NEEDLE.to_lowercase();

    for (index, line) in lines.iter().enumerate() {
        if !line.to_lowercase().contains(&needle) {
            continue;
        }
        // The needle may be part of the key path itself ...
        if is_key_path(line) {
            if let Some(guid) = key_guid(line) {
                return Some(guid);
            }
            continue; // a key path without a GUID, nothing to extract
        }
        // ... or live in one of its values, so walk up to the key path.
        for previous in lines[..index].iter().rev().take(64) {
            if previous.trim().is_empty() {
                break; // separated from its key by a blank line
            }
            if is_key_path(previous) {
                if let Some(guid) = key_guid(previous) {
                    return Some(guid);
                }
                break;
            }
        }
    }
    None
}

/// Extracts `{...}` from a registry key path.
pub fn key_guid(line: &str) -> Option<String> {
    let start = line.find('{')?;
    let end = line[start..].find('}')? + start;
    if end == start + 1 {
        return None;
    }
    Some(format!("{{{}}}", &line[start + 1..end]))
}

/// `reg.exe` writes in the console code page, but the needle and the GUIDs are
/// ASCII, so only a UTF-16 BOM needs special treatment.
pub fn decode_output(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|chunk| u16::from_le_bytes(*chunk))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn is_key_path(line: &str) -> bool {
    line.trim_start()
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("HKEY_"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUID: &str = "{12345678-ABCD-1234-ABCD-1234567890AB}";

    fn query(lines: &[&str]) -> String {
        lines.join("\r\n")
    }

    #[test]
    fn needle_in_a_value_points_at_the_key_above() {
        let text = query(&[
            r"HKEY_CLASSES_ROOT\CLSID\{00000000-0000-0000-0000-000000000001}",
            r"    (Default)    REG_SZ",
            "",
            &format!(r"HKEY_CLASSES_ROOT\CLSID\{GUID}"),
            r"    scansk    REG_BINARY    deadbeef",
            &format!("    Secret    REG_SZ    {NEEDLE}"),
            r"    Other    REG_SZ    something else",
        ]);
        assert_eq!(guid_in_query_output(&text), Some(GUID.to_string()));
    }

    #[test]
    fn needle_in_a_child_key_path_is_found_directly() {
        let text = query(&[
            r"HKEY_CLASSES_ROOT\CLSID\{00000000-0000-0000-0000-000000000001}",
            &format!(r"HKEY_CLASSES_ROOT\CLSID\{GUID}\InprocServer32"),
            &format!("    Path    REG_SZ    C:\\x\\{NEEDLE}.dll"),
        ]);
        assert_eq!(guid_in_query_output(&text), Some(GUID.to_string()));
    }

    #[test]
    fn the_needle_is_matched_case_insensitively() {
        let text = query(&[
            &format!(r"HKEY_CLASSES_ROOT\CLSID\{GUID}"),
            &format!("    Secret    REG_SZ    {}", NEEDLE.to_lowercase()),
        ]);
        assert_eq!(guid_in_query_output(&text), Some(GUID.to_string()));
    }

    #[test]
    fn a_key_path_without_a_guid_is_not_reported() {
        let text = query(&[
            r"HKEY_CLASSES_ROOT\CLSID",
            &format!("    Secret    REG_SZ    {NEEDLE}"),
        ]);
        assert_eq!(guid_in_query_output(&text), None);
    }

    #[test]
    fn a_match_after_a_blank_line_belongs_to_nothing() {
        let text = query(&["", &format!("    Secret    REG_SZ    {NEEDLE}")]);
        assert_eq!(guid_in_query_output(&text), None);
    }

    #[test]
    fn missing_needle_returns_nothing() {
        let text = query(&[
            &format!(r"HKEY_CLASSES_ROOT\CLSID\{GUID}"),
            r"    scansk    REG_BINARY    deadbeef",
        ]);
        assert_eq!(guid_in_query_output(&text), None);
        assert_eq!(guid_in_query_output(""), None);
    }

    #[test]
    fn extracts_the_guid_out_of_a_key_path() {
        let path = format!(r"HKEY_CLASSES_ROOT\CLSID\{GUID}");
        assert_eq!(key_guid(&path), Some(GUID.to_string()));

        let child = format!(r"HKEY_CLASSES_ROOT\WOW6432Node\CLSID\{GUID}\ProgID");
        assert_eq!(key_guid(&child), Some(GUID.to_string()));

        assert_eq!(key_guid(r"    scansk    REG_BINARY    deadbeef"), None);
        assert_eq!(key_guid(r"HKEY_CLASSES_ROOT\TypeLib"), None);
        assert_eq!(key_guid(r"HKEY_CLASSES_ROOT\CLSID\{}"), None);
    }

    #[test]
    fn decodes_reg_query_output() {
        assert_eq!(decode_output(b"HKEY_CLASSES_ROOT"), "HKEY_CLASSES_ROOT");
        let utf16: Vec<u8> = [0xFFu8, 0xFE]
            .into_iter()
            .chain("HKEY".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        assert_eq!(decode_output(&utf16), "HKEY");
    }
}
