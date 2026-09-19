use crate::parse::UnitFile;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub message: &'static str,
}

fn get<'a>(unit: &'a UnitFile, section: &str, key: &str) -> Option<&'a str> {
    unit.get(section)?
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn has_key(unit: &UnitFile, section: &str, key: &str) -> bool {
    unit.get(section)
        .is_some_and(|pairs| pairs.iter().any(|(k, _)| k == key))
}

/// Checks a parsed unit file for the mistakes that actually bite in
/// practice — not full systemd spec validation (this doesn't know every
/// directive's valid values), the specific handful of gaps that make a
/// service silently unreliable or unnecessarily privileged.
pub fn validate(unit: &UnitFile) -> Vec<Issue> {
    let mut issues = Vec::new();

    if !unit.contains_key("Service") {
        issues.push(Issue {
            severity: Severity::Error,
            message: "no [Service] section at all",
        });
        return issues; // nothing else below is checkable without one
    }

    match get(unit, "Service", "ExecStart") {
        None => issues.push(Issue {
            severity: Severity::Error,
            message: "[Service] has no ExecStart= — systemd has nothing to run",
        }),
        Some(path) if !path.starts_with('/') && !path.starts_with('!') => issues.push(Issue {
            severity: Severity::Error,
            message: "ExecStart= must be an absolute path — systemd does not search $PATH",
        }),
        _ => {}
    }

    if !has_key(unit, "Service", "Restart") {
        issues.push(Issue {
            severity: Severity::Warning,
            message: "no Restart= directive — a crashed service will just stay down instead of being restarted",
        });
    }

    if !has_key(unit, "Service", "User") {
        issues.push(Issue {
            severity: Severity::Warning,
            message: "no User= directive — this runs as root, more privilege than almost any service needs",
        });
    }

    if !unit.contains_key("Install") || !has_key(unit, "Install", "WantedBy") {
        issues.push(Issue {
            severity: Severity::Warning,
            message: "no [Install] WantedBy= — `systemctl enable` will have nothing to hook into",
        });
    }

    if !has_key(unit, "Unit", "Description") {
        issues.push(Issue {
            severity: Severity::Warning,
            message: "no Description= — `systemctl status` output will be uninformative",
        });
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_unit_file;

    #[test]
    fn a_well_formed_unit_file_has_no_issues() {
        let content = "\
[Unit]
Description=My App
[Service]
ExecStart=/usr/local/bin/myapp
Restart=on-failure
User=myapp
[Install]
WantedBy=multi-user.target
";
        assert!(validate(&parse_unit_file(content)).is_empty());
    }

    #[test]
    fn missing_service_section_is_a_single_fatal_error_not_a_flood() {
        let content = "[Unit]\nDescription=x\n";
        let issues = validate(&parse_unit_file(content));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
    }

    #[test]
    fn missing_exec_start_is_an_error() {
        let content = "[Unit]\nDescription=x\n[Service]\nRestart=on-failure\n";
        let issues = validate(&parse_unit_file(content));
        assert!(issues
            .iter()
            .any(|i| i.severity == Severity::Error && i.message.contains("ExecStart")));
    }

    #[test]
    fn relative_exec_start_path_is_an_error() {
        let content = "[Service]\nExecStart=myapp\n";
        let issues = validate(&parse_unit_file(content));
        assert!(issues.iter().any(|i| i.message.contains("absolute path")));
    }

    #[test]
    fn missing_restart_directive_is_a_warning() {
        let content =
            "[Service]\nExecStart=/bin/myapp\nUser=x\n[Install]\nWantedBy=multi-user.target\n";
        let issues = validate(&parse_unit_file(content));
        assert!(issues.iter().any(|i| i.message.contains("Restart=")));
    }

    #[test]
    fn missing_user_is_flagged_as_running_privileged() {
        let content = "[Service]\nExecStart=/bin/myapp\nRestart=on-failure\n[Install]\nWantedBy=multi-user.target\n";
        let issues = validate(&parse_unit_file(content));
        assert!(issues.iter().any(|i| i.message.contains("root")));
    }

    #[test]
    fn missing_install_section_is_flagged() {
        let content = "[Service]\nExecStart=/bin/myapp\nRestart=on-failure\nUser=x\n";
        let issues = validate(&parse_unit_file(content));
        assert!(issues
            .iter()
            .any(|i| i.message.contains("systemctl enable")));
    }
}
