#[derive(Debug, Clone, Default)]
pub struct ServiceConfig {
    pub description: String,
    pub exec_start: String,
    pub working_directory: Option<String>,
    pub user: Option<String>,
    pub env: Vec<(String, String)>,
    pub restart_sec: u32,
}

/// Renders a `.service` unit file with defaults that matter, not just
/// the bare minimum that parses: `Restart=on-failure` (a crashed service
/// staying down is the single most common systemd deploy mistake),
/// `NoNewPrivileges`/`ProtectSystem=strict`/`ProtectHome=true` (real
/// hardening that costs nothing for the overwhelming majority of
/// services, opted out of explicitly if you actually need to write
/// outside your own working directory, not opted into by remembering to
/// add flags), and a real `User=` line when given rather than defaulting
/// to root silently.
pub fn generate_unit_file(cfg: &ServiceConfig) -> String {
    let mut out = String::new();

    out.push_str("[Unit]\n");
    out.push_str(&format!("Description={}\n", cfg.description));
    out.push_str("After=network-online.target\n");
    out.push_str("Wants=network-online.target\n\n");

    out.push_str("[Service]\n");
    out.push_str(&format!("ExecStart={}\n", cfg.exec_start));
    if let Some(dir) = &cfg.working_directory {
        out.push_str(&format!("WorkingDirectory={dir}\n"));
    }
    if let Some(user) = &cfg.user {
        out.push_str(&format!("User={user}\n"));
    }
    for (key, value) in &cfg.env {
        out.push_str(&format!("Environment=\"{key}={value}\"\n"));
    }
    out.push_str("Restart=on-failure\n");
    out.push_str(&format!("RestartSec={}\n", cfg.restart_sec));
    out.push_str("NoNewPrivileges=true\n");
    out.push_str("ProtectSystem=strict\n");
    out.push_str("ProtectHome=true\n");
    if let Some(dir) = &cfg.working_directory {
        out.push_str(&format!("ReadWritePaths={dir}\n"));
    }
    out.push('\n');

    out.push_str("[Install]\n");
    out.push_str("WantedBy=multi-user.target\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> ServiceConfig {
        ServiceConfig {
            description: "My App".to_string(),
            exec_start: "/usr/local/bin/myapp".to_string(),
            working_directory: Some("/opt/myapp".to_string()),
            user: Some("myapp".to_string()),
            env: vec![("PORT".to_string(), "8080".to_string())],
            restart_sec: 5,
        }
    }

    #[test]
    fn includes_restart_on_failure_by_default() {
        let unit = generate_unit_file(&base_config());
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("RestartSec=5"));
    }

    #[test]
    fn includes_hardening_directives_by_default() {
        let unit = generate_unit_file(&base_config());
        assert!(unit.contains("NoNewPrivileges=true"));
        assert!(unit.contains("ProtectSystem=strict"));
        assert!(unit.contains("ProtectHome=true"));
    }

    #[test]
    fn includes_the_given_user_not_defaulting_to_root_silently() {
        let unit = generate_unit_file(&base_config());
        assert!(unit.contains("User=myapp"));
    }

    #[test]
    fn omits_user_line_entirely_when_none_given_rather_than_a_blank_directive() {
        let mut cfg = base_config();
        cfg.user = None;
        let unit = generate_unit_file(&cfg);
        assert!(!unit.contains("User="));
    }

    #[test]
    fn environment_variables_are_rendered_as_separate_lines() {
        let mut cfg = base_config();
        cfg.env.push(("LOG_LEVEL".to_string(), "info".to_string()));
        let unit = generate_unit_file(&cfg);
        assert!(unit.contains(r#"Environment="PORT=8080""#));
        assert!(unit.contains(r#"Environment="LOG_LEVEL=info""#));
    }

    #[test]
    fn produces_all_three_required_sections() {
        let unit = generate_unit_file(&base_config());
        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("[Install]"));
        assert!(
            unit.contains("WantedBy=multi-user.target"),
            "without this, `systemctl enable` has nothing to hook into"
        );
    }
}
