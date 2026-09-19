use std::collections::BTreeMap;

/// A parsed unit file: section name -> ordered `(key, value)` pairs.
/// Systemd allows a key to repeat within a section (multiple
/// `ExecStartPre=` lines, for instance), so this is a `Vec` of pairs per
/// section, not a map that would silently drop duplicates.
pub type UnitFile = BTreeMap<String, Vec<(String, String)>>;

/// A deliberately small INI-style parser — real unit files use exactly
/// this shape (`[Section]` headers, `Key=Value` lines, `#`/`;` comments),
/// and systemd's own format is a strict enough subset of INI that a full
/// general-purpose INI crate would buy nothing a dozen lines here don't
/// already cover.
pub fn parse_unit_file(content: &str) -> UnitFile {
    let mut result = UnitFile::new();
    let mut current_section: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].to_string();
            result.entry(name.clone()).or_default();
            current_section = Some(name);
            continue;
        }
        if let (Some(section), Some((key, value))) = (&current_section, line.split_once('=')) {
            result
                .entry(section.clone())
                .or_default()
                .push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_key_value_pairs() {
        let content =
            "[Unit]\nDescription=My App\n\n[Service]\nExecStart=/bin/myapp\nRestart=on-failure\n";
        let unit = parse_unit_file(content);
        assert_eq!(
            unit["Unit"],
            vec![("Description".to_string(), "My App".to_string())]
        );
        assert_eq!(
            unit["Service"],
            vec![
                ("ExecStart".to_string(), "/bin/myapp".to_string()),
                ("Restart".to_string(), "on-failure".to_string())
            ]
        );
    }

    #[test]
    fn duplicate_keys_in_a_section_are_all_kept_not_overwritten() {
        let content = "[Service]\nExecStartPre=/bin/one\nExecStartPre=/bin/two\n";
        let unit = parse_unit_file(content);
        assert_eq!(unit["Service"].len(), 2);
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let content = "[Unit]\n# a comment\n\n; another style of comment\nDescription=X\n";
        let unit = parse_unit_file(content);
        assert_eq!(
            unit["Unit"],
            vec![("Description".to_string(), "X".to_string())]
        );
    }

    #[test]
    fn a_section_with_no_keys_still_appears_as_an_empty_entry() {
        let content = "[Install]\n";
        let unit = parse_unit_file(content);
        assert!(unit.contains_key("Install"));
        assert!(unit["Install"].is_empty());
    }

    #[test]
    fn lines_before_any_section_header_are_ignored() {
        let content = "Description=orphaned\n[Unit]\nDescription=real\n";
        let unit = parse_unit_file(content);
        assert_eq!(
            unit["Unit"],
            vec![("Description".to_string(), "real".to_string())]
        );
    }
}
