//! Opt-in project-layer bootstrap: `.grok/` → `.powergrok/` (issue #7).
//!
//! Fail-closed D7 is unchanged: powergrok never *reads* project `.grok/` at
//! runtime. This module only helps operators **create** `.powergrok/` after an
//! explicit confirmation. No automatic copy on install or first paint.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::paths::{
    is_powergrok_project_tree, project_config_dirname, should_emit_empty_project_layer_warning,
};

/// Well-known entries under a project config tree that operators commonly copy.
pub const BOOTSTRAP_CATEGORIES: &[&str] = &[
    "config.toml",
    "skills",
    "hooks",
    "agents",
    "plugins",
    "personas",
    "roles",
    "sandbox.toml",
    "lsp.json",
    "rules",
    "effort-brains",
];

/// Snapshot of whether bootstrap applies and what `.grok/` contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapStatus {
    pub workspace_root: PathBuf,
    pub product_dirname: &'static str,
    pub is_powergrok: bool,
    pub needs_bootstrap: bool,
    pub official_present: bool,
    pub power_present: bool,
    /// Categories present under `.grok/` (subset of [`BOOTSTRAP_CATEGORIES`]).
    pub available_categories: Vec<String>,
}

/// Result of an explicit bootstrap action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapReport {
    pub action: String,
    pub destination: PathBuf,
    pub copied: Vec<String>,
    pub skipped: Vec<String>,
    pub notes: Vec<String>,
}

/// Detect empty-layer / bootstrap need for `workspace_root`.
///
/// Uses the same conditions as G4 (`should_emit_empty_project_layer_warning`)
/// plus category inventory under `.grok/`.
pub fn assess_project_bootstrap(workspace_root: &Path) -> BootstrapStatus {
    let official = workspace_root.join(".grok");
    let power = workspace_root.join(".powergrok");
    let official_present = official.is_dir();
    let power_present = power.is_dir();
    let is_powergrok = is_powergrok_project_tree();
    let needs_bootstrap = should_emit_empty_project_layer_warning(workspace_root)
        || (is_powergrok && official_present && !power_present);

    let available_categories = if official_present {
        inventory_categories(&official)
    } else {
        Vec::new()
    };

    BootstrapStatus {
        workspace_root: workspace_root.to_path_buf(),
        product_dirname: project_config_dirname(),
        is_powergrok,
        needs_bootstrap,
        official_present,
        power_present,
        available_categories,
    }
}

fn inventory_categories(official_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for name in BOOTSTRAP_CATEGORIES {
        let p = official_root.join(name);
        if p.exists() {
            out.push((*name).to_string());
        }
    }
    // Also note other top-level entries (for preview honesty).
    if let Ok(rd) = fs::read_dir(official_root) {
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if !out.iter().any(|c| c == &name) {
                out.push(format!("{name} (other)"));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Human-readable status / help for `/bootstrap-project` with no mutating args.
pub fn format_bootstrap_status(status: &BootstrapStatus) -> String {
    let mut lines = Vec::new();
    lines.push("Power Grok project bootstrap".to_string());
    lines.push(format!("  workspace: {}", status.workspace_root.display()));
    lines.push(format!(
        "  product project dir: {}/",
        status.product_dirname
    ));
    lines.push(format!("  .grok/ present: {}", status.official_present));
    lines.push(format!(
        "  .powergrok/ present: {}",
        status.power_present
    ));

    if !status.is_powergrok {
        lines.push(String::new());
        lines.push(
            "This process is not running as powergrok (project tree is .grok/). \
             Bootstrap is only needed for the Power Grok binary."
                .to_string(),
        );
        return lines.join("\n");
    }

    if status.power_present {
        lines.push(String::new());
        lines.push(
            "`.powergrok/` already exists. Power Grok reads only that tree (D7 — no merge with `.grok/`)."
                .to_string(),
        );
        lines.push(
            "Optional: compare names only; do not expect runtime merge.".to_string(),
        );
        return lines.join("\n");
    }

    if !status.official_present {
        lines.push(String::new());
        lines.push(
            "No `.grok/` project tree either. Start empty with: /bootstrap-project --empty --confirm"
                .to_string(),
        );
        return lines.join("\n");
    }

    lines.push(String::new());
    lines.push(
        "Project MCP/skills/hooks look empty under Power Grok because isolation \
         uses only `.powergrok/` — not because the model failed."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("Categories found under .grok/:".to_string());
    if status.available_categories.is_empty() {
        lines.push("  (none of the common categories)".to_string());
    } else {
        for c in &status.available_categories {
            lines.push(format!("  - {c}"));
        }
    }
    lines.push(String::new());
    lines.push("Opt-in actions (nothing runs without --confirm):".to_string());
    lines.push("  /bootstrap-project --preview".to_string());
    lines.push("  /bootstrap-project --copy-all --confirm".to_string());
    lines.push(
        "  /bootstrap-project --copy skills,hooks,config.toml --confirm".to_string(),
    );
    lines.push("  /bootstrap-project --empty --confirm".to_string());
    lines.push("  /bootstrap-project --not-now   (dismiss this tip; G4 stays once/process)".to_string());
    lines.push(String::new());
    lines.push(
        "After copy: if `.powergrok/` is personal-only, add it to .gitignore \
         yourself (G7 — Power Grok never edits your gitignore)."
            .to_string(),
    );
    lines.push(
        "Docs: docs/powergrok/BUILD_PLAN.md §7.2–§7.3 (empty layer + migration)."
            .to_string(),
    );
    lines.join("\n")
}

/// Create an empty `.powergrok/` (or leave existing). Requires `confirm`.
pub fn bootstrap_start_empty(workspace_root: &Path, confirm: bool) -> io::Result<BootstrapReport> {
    require_powergrok_process()?;
    if !confirm {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing to create .powergrok/ without --confirm",
        ));
    }
    let dest = workspace_root.join(".powergrok");
    fs::create_dir_all(&dest)?;
    // Minimal marker so operators see an intentional empty layer.
    let readme = dest.join("README.powergrok-bootstrap.md");
    if !readme.exists() {
        fs::write(
            &readme,
            "# Power Grok project layer\n\n\
             Created empty via `/bootstrap-project --empty --confirm`.\n\
             Power Grok reads only this directory (not `.grok/`).\n\
             See docs/powergrok/BUILD_PLAN.md §7.2–§7.3.\n",
        )?;
    }
    Ok(BootstrapReport {
        action: "empty".into(),
        destination: dest,
        copied: vec!["README.powergrok-bootstrap.md".into()],
        skipped: vec![],
        notes: vec![
            "Empty project layer created. Add skills/MCP under .powergrok/ as needed.".into(),
            "gitignore tip (manual): echo .powergrok/ >> .gitignore  # if personal-only".into(),
        ],
    })
}

/// Full tree copy `.grok` → `.powergrok` (informed cp -R). Requires `confirm`.
/// Fails if destination already exists and is non-empty.
pub fn bootstrap_copy_all(workspace_root: &Path, confirm: bool) -> io::Result<BootstrapReport> {
    require_powergrok_process()?;
    if !confirm {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing full copy without --confirm",
        ));
    }
    let src = workspace_root.join(".grok");
    let dest = workspace_root.join(".powergrok");
    if !src.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no .grok/ directory to copy from",
        ));
    }
    if dest.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            ".powergrok/ already exists; refuse to overwrite (D7 — no merge)",
        ));
    }
    copy_dir_recursive(&src, &dest)?;
    let cats = inventory_categories(&src);
    Ok(BootstrapReport {
        action: "copy-all".into(),
        destination: dest,
        copied: cats,
        skipped: vec![],
        notes: vec![
            "Full copy complete. Power Grok still does not read project .grok/ at runtime.".into(),
            "gitignore tip (manual): echo .powergrok/ >> .gitignore  # if personal-only".into(),
        ],
    })
}

/// Selective copy of named categories from `.grok` into `.powergrok`.
/// Unknown names are skipped with notes. Requires `confirm`.
pub fn bootstrap_copy_categories(
    workspace_root: &Path,
    categories: &[&str],
    confirm: bool,
) -> io::Result<BootstrapReport> {
    require_powergrok_process()?;
    if !confirm {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing selective copy without --confirm",
        ));
    }
    if categories.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no categories specified (e.g. --copy skills,hooks,config.toml)",
        ));
    }
    let src_root = workspace_root.join(".grok");
    let dest_root = workspace_root.join(".powergrok");
    if !src_root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no .grok/ directory to copy from",
        ));
    }
    fs::create_dir_all(&dest_root)?;

    let mut copied = Vec::new();
    let mut skipped = Vec::new();
    let mut notes = Vec::new();

    for raw in categories {
        let name = raw.trim();
        if name.is_empty() {
            continue;
        }
        // Refuse path escape.
        if name.contains("..") || name.contains('/') || name.contains('\\') {
            skipped.push(name.to_string());
            notes.push(format!("skipped unsafe name: {name}"));
            continue;
        }
        let src = src_root.join(name);
        let dest = dest_root.join(name);
        if !src.exists() {
            skipped.push(name.to_string());
            notes.push(format!("not found under .grok/: {name}"));
            continue;
        }
        if dest.exists() {
            skipped.push(name.to_string());
            notes.push(format!("destination exists, left untouched: {name}"));
            continue;
        }
        if src.is_dir() {
            copy_dir_recursive(&src, &dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dest)?;
        }
        copied.push(name.to_string());
    }

    notes.push(
        "Selective copy complete. Power Grok still does not read project .grok/ at runtime."
            .into(),
    );
    notes.push(
        "gitignore tip (manual): echo .powergrok/ >> .gitignore  # if personal-only".into(),
    );

    Ok(BootstrapReport {
        action: "copy-selective".into(),
        destination: dest_root,
        copied,
        skipped,
        notes,
    })
}

/// Parse `/bootstrap-project` args into a status text or perform a mutating action.
///
/// Mutating verbs require `--confirm`. Never touches `~/.grok` or official install.
pub fn run_bootstrap_command(workspace_root: &Path, args: &str) -> String {
    let tokens = split_args(args);
    let status = assess_project_bootstrap(workspace_root);

    if tokens.is_empty() || tokens.iter().any(|t| t == "status" || t == "help" || t == "-h" || t == "--help") {
        return format_bootstrap_status(&status);
    }

    if tokens.iter().any(|t| t == "--not-now" || t == "not-now") {
        return "Bootstrap dismissed for now. Project tools stay empty until you create `.powergrok/` \
                (`/bootstrap-project --copy-all --confirm` or `--empty --confirm`). \
                G4 does not re-nag every keystroke."
            .to_string();
    }

    if tokens.iter().any(|t| t == "--preview" || t == "preview") {
        return format_bootstrap_status(&status);
    }

    let confirm = tokens.iter().any(|t| t == "--confirm" || t == "-y");

    if tokens.iter().any(|t| t == "--empty") {
        return match bootstrap_start_empty(workspace_root, confirm) {
            Ok(r) => format_report(&r),
            Err(e) => format!("bootstrap-project: {e}"),
        };
    }

    if tokens.iter().any(|t| t == "--copy-all") {
        return match bootstrap_copy_all(workspace_root, confirm) {
            Ok(r) => format_report(&r),
            Err(e) => format!("bootstrap-project: {e}"),
        };
    }

    // --copy a,b,c
    if let Some(idx) = tokens.iter().position(|t| t == "--copy") {
        let list = tokens.get(idx + 1).map(String::as_str).unwrap_or("");
        let cats: Vec<&str> = list.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
        return match bootstrap_copy_categories(workspace_root, &cats, confirm) {
            Ok(r) => format_report(&r),
            Err(e) => format!("bootstrap-project: {e}"),
        };
    }

    format!(
        "Unknown bootstrap args: {args}\n\n{}",
        format_bootstrap_status(&status)
    )
}

fn format_report(r: &BootstrapReport) -> String {
    let mut lines = vec![
        format!("bootstrap-project: {}", r.action),
        format!("  destination: {}", r.destination.display()),
    ];
    if !r.copied.is_empty() {
        lines.push(format!("  copied: {}", r.copied.join(", ")));
    }
    if !r.skipped.is_empty() {
        lines.push(format!("  skipped: {}", r.skipped.join(", ")));
    }
    for n in &r.notes {
        lines.push(format!("  note: {n}"));
    }
    lines.join("\n")
}

fn require_powergrok_process() -> io::Result<()> {
    if !is_powergrok_project_tree() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "bootstrap only applies when running as powergrok (project dirname .powergrok)",
        ));
    }
    Ok(())
}

fn split_args(args: &str) -> Vec<String> {
    args.split_whitespace().map(|s| s.to_string()).collect()
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for ent in fs::read_dir(src)? {
        let ent = ent?;
        let ty = ent.file_type()?;
        let from = ent.path();
        let to = dest.join(ent.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)?;
        } else if ty.is_symlink() {
            // Copy symlink target contents as file/dir if possible; skip broken.
            if from.is_dir() {
                copy_dir_recursive(&from, &to)?;
            } else if from.is_file() {
                fs::copy(&from, &to)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::set_project_config_dirname_for_test;
    use tempfile::TempDir;

    #[test]
    #[serial_test::serial]
    fn assess_needs_bootstrap_under_powergrok() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".grok").join("skills")).unwrap();
        fs::write(tmp.path().join(".grok/config.toml"), "[cli]\n").unwrap();
        let s = assess_project_bootstrap(tmp.path());
        assert!(s.needs_bootstrap);
        assert!(s.available_categories.iter().any(|c| c == "skills"));
        assert!(s.available_categories.iter().any(|c| c == "config.toml"));
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn copy_all_requires_confirm() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".grok")).unwrap();
        let err = bootstrap_copy_all(tmp.path(), false).unwrap_err();
        assert!(err.to_string().contains("--confirm"));
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn copy_all_creates_powergrok_and_leaves_grok() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        let skills = tmp.path().join(".grok/skills/demo");
        fs::create_dir_all(&skills).unwrap();
        fs::write(skills.join("SKILL.md"), "# demo\n").unwrap();
        fs::write(tmp.path().join(".grok/config.toml"), "x = 1\n").unwrap();

        let report = bootstrap_copy_all(tmp.path(), true).unwrap();
        assert_eq!(report.action, "copy-all");
        assert!(tmp.path().join(".powergrok/config.toml").is_file());
        assert!(tmp.path().join(".powergrok/skills/demo/SKILL.md").is_file());
        // Source intact (not moved).
        assert!(tmp.path().join(".grok/config.toml").is_file());
        // Second full copy refused.
        assert!(bootstrap_copy_all(tmp.path(), true).is_err());
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn selective_copy_only_requested() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".grok/skills/a")).unwrap();
        fs::create_dir_all(tmp.path().join(".grok/hooks")).unwrap();
        fs::write(tmp.path().join(".grok/config.toml"), "x=1\n").unwrap();

        let report =
            bootstrap_copy_categories(tmp.path(), &["skills", "config.toml"], true).unwrap();
        assert!(report.copied.contains(&"skills".into()));
        assert!(report.copied.contains(&"config.toml".into()));
        assert!(tmp.path().join(".powergrok/skills/a").is_dir());
        assert!(tmp.path().join(".powergrok/config.toml").is_file());
        assert!(!tmp.path().join(".powergrok/hooks").exists());
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn empty_creates_dir() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        let report = bootstrap_start_empty(tmp.path(), true).unwrap();
        assert!(tmp.path().join(".powergrok").is_dir());
        assert!(
            tmp.path()
                .join(".powergrok/README.powergrok-bootstrap.md")
                .is_file()
        );
        assert_eq!(report.action, "empty");
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn refuse_path_escape_in_categories() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".grok")).unwrap();
        let report = bootstrap_copy_categories(tmp.path(), &["../etc"], true).unwrap();
        assert!(report.copied.is_empty());
        assert!(report.skipped.iter().any(|s| s.contains("..")));
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn run_command_status_mentions_isolation() {
        set_project_config_dirname_for_test(Some(".powergrok"));
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".grok/skills")).unwrap();
        let out = run_bootstrap_command(tmp.path(), "");
        assert!(out.contains("isolation") || out.contains("only `.powergrok`"), "{out}");
        assert!(out.contains("--confirm"), "{out}");
        set_project_config_dirname_for_test(None);
    }

    #[test]
    #[serial_test::serial]
    fn refuse_when_not_powergrok() {
        set_project_config_dirname_for_test(Some(".grok"));
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".grok")).unwrap();
        assert!(bootstrap_copy_all(tmp.path(), true).is_err());
        set_project_config_dirname_for_test(None);
    }
}
