//! Install identity / VERSION stamp for source-built Power Grok (issue #14).
//!
//! The installer writes `$prefix/lib/powergrok/VERSION` next to the real
//! binary. When argv0 is the named binary, that file is a sibling of
//! `current_exe()`.

use std::fs;
use std::path::{Path, PathBuf};

/// Parsed fields from an install `VERSION` file (best-effort).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallVersionStamp {
    pub path: PathBuf,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub built_at: Option<String>,
    pub features: Option<String>,
    pub version_line: Option<String>,
    pub raw: String,
}

/// Locate `VERSION` beside the running executable (lib/powergrok/VERSION).
pub fn version_path_beside_current_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join("VERSION");
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

/// Read and parse a VERSION file.
pub fn read_install_version_file(path: &Path) -> Option<InstallVersionStamp> {
    let raw = fs::read_to_string(path).ok()?;
    let mut stamp = InstallVersionStamp {
        path: path.to_path_buf(),
        raw: raw.clone(),
        ..Default::default()
    };
    for line in raw.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("git=") {
            stamp.git = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("branch=") {
            stamp.branch = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("built_at=") {
            stamp.built_at = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("features=") {
            stamp.features = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("version_line=") {
            stamp.version_line = Some(rest.to_string());
        }
    }
    Some(stamp)
}

/// Best-effort stamp for the running process.
pub fn current_install_version() -> Option<InstallVersionStamp> {
    let path = version_path_beside_current_exe()?;
    read_install_version_file(&path)
}

/// Human-readable install identity for `/install-status` and doctor surfaces.
pub fn format_install_status_report() -> String {
    let mut lines = Vec::new();
    lines.push("Power Grok install identity".to_string());
    lines.push(format!(
        "  product: {}",
        crate::branding::product_name()
    ));
    lines.push(format!(
        "  project_dirname: {}",
        crate::paths::project_config_dirname()
    ));
    match std::env::var("GROK_HOME") {
        Ok(h) => lines.push(format!("  GROK_HOME: {h}")),
        Err(_) => lines.push("  GROK_HOME: (unset — official default ~/.grok may apply)".into()),
    }

    match current_install_version() {
        Some(st) => {
            lines.push(format!("  VERSION_file: {}", st.path.display()));
            if let Some(g) = &st.git {
                lines.push(format!("  git: {g}"));
            }
            if let Some(b) = &st.branch {
                lines.push(format!("  branch: {b}"));
            }
            if let Some(t) = &st.built_at {
                lines.push(format!("  built_at: {t}"));
            }
            if let Some(f) = &st.features {
                lines.push(format!("  features: {f}"));
            }
            if let Some(v) = &st.version_line {
                lines.push(format!("  version_line: {v}"));
            }
        }
        None => {
            lines.push(
                "  VERSION_file: (not found beside current_exe — dev/cargo run or non-install layout)"
                    .into(),
            );
            lines.push(
                "  tip: after install, VERSION lives at $prefix/lib/powergrok/VERSION".into(),
            );
        }
    }

    // auto_update probe from GROK_HOME when set.
    if let Ok(home) = std::env::var("GROK_HOME") {
        let cfg = PathBuf::from(&home).join("config.toml");
        if cfg.is_file() {
            if let Ok(text) = fs::read_to_string(&cfg) {
                if text.lines().any(|l| {
                    let t = l.trim();
                    t.starts_with("auto_update") && t.contains("false")
                }) {
                    lines.push("  auto_update: false (expected for source-built Power Grok)".into());
                } else if text.lines().any(|l| l.trim().starts_with("auto_update")) {
                    lines.push(
                        "  auto_update: present but not false — check config.toml".into(),
                    );
                } else {
                    lines.push("  auto_update: not set in config.toml".into());
                }
            }
        } else {
            lines.push(format!("  config: missing ({})", cfg.display()));
        }
    }

    lines.push(String::new());
    lines.push("Upgrade (operator-controlled; no official auto-update):".into());
    lines.push("  git pull origin powergrok".into());
    lines.push("  ./scripts/install-powergrok.sh".into());
    lines.push("Status / rollback:".into());
    lines.push("  ./scripts/install-powergrok.sh --status".into());
    lines.push("  ./scripts/install-powergrok.sh --status --check-freshness".into());
    lines.push("  ./scripts/install-powergrok.sh --rollback".into());
    lines.push("Runbook: docs/powergrok/LIFECYCLE.md".into());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_version_file_fields() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("VERSION");
        fs::write(
            &p,
            "git=abc123\nbranch=powergrok\nbuilt_at=2026-07-18T00:00:00Z\nfeatures=powergrok\nversion_line=Power Grok 0.1\n",
        )
        .unwrap();
        let st = read_install_version_file(&p).unwrap();
        assert_eq!(st.git.as_deref(), Some("abc123"));
        assert_eq!(st.branch.as_deref(), Some("powergrok"));
        assert_eq!(st.built_at.as_deref(), Some("2026-07-18T00:00:00Z"));
        assert_eq!(st.features.as_deref(), Some("powergrok"));
        assert!(st.version_line.unwrap().contains("Power Grok"));
    }

    #[test]
    fn format_report_mentions_lifecycle() {
        let out = format_install_status_report();
        assert!(out.contains("install-powergrok.sh"), "{out}");
        assert!(out.contains("LIFECYCLE"), "{out}");
        assert!(out.contains("project_dirname"), "{out}");
    }
}
