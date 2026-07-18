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
    /// Present when installer wrote `state=rolled-back` after `--rollback`.
    pub state: Option<String>,
    pub note: Option<String>,
    pub raw: String,
}

impl InstallVersionStamp {
    /// True when this stamp is a post-rollback marker (not a real commit SHA).
    pub fn is_rolled_back(&self) -> bool {
        self.state.as_deref() == Some("rolled-back")
            || self.git.as_deref() == Some("rollback-from-prev")
            || self.git.as_deref() == Some("(unknown)")
                && self
                    .note
                    .as_ref()
                    .is_some_and(|n| n.to_ascii_lowercase().contains("rollback"))
    }
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

/// Sibling `powergrok.prev` next to the running executable (rollback source).
pub fn prev_binary_beside_current_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let candidate = dir.join("powergrok.prev");
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
        } else if let Some(rest) = line.strip_prefix("state=") {
            stamp.state = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("note=") {
            stamp.note = Some(rest.to_string());
        }
    }
    Some(stamp)
}

/// Best-effort stamp for the running process.
pub fn current_install_version() -> Option<InstallVersionStamp> {
    let path = version_path_beside_current_exe()?;
    read_install_version_file(&path)
}

/// Detect `auto_update = false` the same way the installer greps seed config.
///
/// Strips trailing `# comments`, trims, then matches `auto_update = false`
/// (optional spaces). Comment-only lines and `auto_update = true` do not match.
pub fn config_has_auto_update_false(text: &str) -> bool {
    for line in text.lines() {
        let bare = strip_toml_line_comment(line).trim();
        if bare.is_empty() {
            continue;
        }
        if auto_update_assignment_is_false(bare) {
            return true;
        }
    }
    false
}

/// True if any non-comment line assigns `auto_update` (any value).
pub fn config_has_auto_update_key(text: &str) -> bool {
    for line in text.lines() {
        let bare = strip_toml_line_comment(line).trim();
        if bare.is_empty() {
            continue;
        }
        if bare
            .split_once('=')
            .is_some_and(|(k, _)| k.trim() == "auto_update")
        {
            return true;
        }
    }
    false
}

fn strip_toml_line_comment(line: &str) -> &str {
    // TOML full-line and trailing comments start with `#` outside strings.
    // Seed config is simple keys — strip from first `#`.
    line.split_once('#').map(|(a, _)| a).unwrap_or(line)
}

fn auto_update_assignment_is_false(bare: &str) -> bool {
    let Some((key, value)) = bare.split_once('=') else {
        return false;
    };
    if key.trim() != "auto_update" {
        return false;
    }
    let v = value.trim().trim_matches('"').trim_matches('\'');
    v.eq_ignore_ascii_case("false")
}

/// Human-readable install identity for `/install-status` and doctor surfaces.
pub fn format_install_status_report() -> String {
    let mut lines = Vec::new();
    lines.push("Power Grok install identity".to_string());
    lines.push(format!("  product: {}", crate::branding::product_name()));
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
            if st.is_rolled_back() {
                lines.push(
                    "  state: rolled-back (restored from powergrok.prev — not a commit SHA)".into(),
                );
                if let Some(n) = &st.note {
                    lines.push(format!("  note: {n}"));
                }
                lines.push(
                    "  tip: re-run ./scripts/install-powergrok.sh after git pull for a real git= SHA"
                        .into(),
                );
            } else {
                if let Some(g) = &st.git {
                    lines.push(format!("  git: {g}"));
                }
                if let Some(b) = &st.branch {
                    lines.push(format!("  branch: {b}"));
                }
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

    match prev_binary_beside_current_exe() {
        Some(p) => lines.push(format!(
            "  powergrok.prev: present ({}) — rollback available",
            p.display()
        )),
        None => lines.push(
            "  powergrok.prev: none (rollback available only after an upgrade install)".into(),
        ),
    }

    // auto_update probe from GROK_HOME when set.
    if let Ok(home) = std::env::var("GROK_HOME") {
        let cfg = PathBuf::from(&home).join("config.toml");
        if cfg.is_file() {
            if let Ok(text) = fs::read_to_string(&cfg) {
                if config_has_auto_update_false(&text) {
                    lines
                        .push("  auto_update: false (expected for source-built Power Grok)".into());
                } else if config_has_auto_update_key(&text) {
                    lines.push("  auto_update: present but not false — check config.toml".into());
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
        assert!(
            st.version_line
                .as_deref()
                .unwrap_or("")
                .contains("Power Grok")
        );
        assert!(!st.is_rolled_back());
    }

    #[test]
    fn parse_rolled_back_stamp() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("VERSION");
        fs::write(
            &p,
            "state=rolled-back\ngit=(unknown)\nnote=restored from powergrok.prev\nbuilt_at=2026-07-18T00:00:00Z\n",
        )
        .unwrap();
        let st = read_install_version_file(&p).unwrap();
        assert!(st.is_rolled_back());
        assert_eq!(st.state.as_deref(), Some("rolled-back"));
    }

    #[test]
    fn auto_update_false_strict() {
        assert!(config_has_auto_update_false("[cli]\nauto_update = false\n"));
        assert!(config_has_auto_update_false("auto_update=false\n"));
        assert!(config_has_auto_update_false(
            "  auto_update = false  # seed\n"
        ));
        // Comment-only must not count as false.
        assert!(!config_has_auto_update_false("# auto_update was false\n"));
        // true with "false" in comment must not count.
        assert!(!config_has_auto_update_false(
            "auto_update = true # was false\n"
        ));
        assert!(config_has_auto_update_key(
            "auto_update = true # was false\n"
        ));
        assert!(!config_has_auto_update_key("# auto_update was false\n"));
    }

    #[test]
    fn format_report_mentions_lifecycle() {
        let out = format_install_status_report();
        assert!(out.contains("install-powergrok.sh"), "{out}");
        assert!(out.contains("LIFECYCLE"), "{out}");
        assert!(out.contains("project_dirname"), "{out}");
        assert!(out.contains("powergrok.prev"), "{out}");
    }
}
