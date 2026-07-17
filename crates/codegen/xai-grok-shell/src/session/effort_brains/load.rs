//! Load and merge effort-brain configuration layers.
//!
//! # Layer order (plan F6)
//! 1. Built-in defaults ([`super::defaults`])
//! 2. `$GROK_HOME/effort-brains/` (user) — merge-by-id when present
//! 3. Project `.powergrok/effort-brains/` — merge-by-id when present
//!
//! Higher layers win for the same brain id. Roster files in higher layers
//! replace the corresponding mode roster entirely when present.

use super::defaults::{expand_expert_pool_all, parse_brain_markdown};
use super::error::EffortBrainError;
use super::types::{
    BrainId, BrainSpec, CatalogFile, EffortBrainConfig, RosterFile, RosterSelection,
    CATALOG_VERSION,
};
use super::validate::validate_effort_brain_config;
use std::path::{Path, PathBuf};

/// Load the effective effort-brain config for this process.
///
/// Uses built-ins always; merges `$GROK_HOME/effort-brains` when that directory
/// exists; does not yet auto-discover project root (pass via
/// [`load_effort_brain_config_from_layers`] from session code later).
pub fn load_effort_brain_config() -> Result<EffortBrainConfig, EffortBrainError> {
    // First-run: seed editable copies under $GROK_HOME/effort-brains when missing.
    let _ = super::seed::ensure_user_effort_brains_seeded();
    let user = xai_grok_config::user_grok_home().map(|h| h.join("effort-brains"));
    let user_ref = user.as_deref().filter(|p| p.is_dir());
    load_effort_brain_config_from_layers(user_ref, None)
}

/// Testable load: optional user/project roots.
pub fn load_effort_brain_config_from_layers(
    user_root: Option<&Path>,
    project_root: Option<&Path>,
) -> Result<EffortBrainConfig, EffortBrainError> {
    let mut cfg = super::defaults::built_in_effort_brain_config()?;
    if let Some(root) = user_root {
        merge_layer(&mut cfg, root, "user")?;
    }
    if let Some(root) = project_root {
        merge_layer(&mut cfg, root, "project")?;
    }
    expand_expert_pool_all(&mut cfg);
    validate_effort_brain_config(&cfg)?;
    Ok(cfg)
}

fn merge_layer(
    cfg: &mut EffortBrainConfig,
    root: &Path,
    layer_name: &str,
) -> Result<(), EffortBrainError> {
    let catalog_path = root.join("catalog.toml");
    if catalog_path.is_file() {
        let text = std::fs::read_to_string(&catalog_path).map_err(|e| EffortBrainError::Io {
            path: catalog_path.clone(),
            detail: e.to_string(),
        })?;
        let catalog: CatalogFile = toml::from_str(&text).map_err(|e| EffortBrainError::CatalogParse {
            source: catalog_path.display().to_string(),
            detail: e.to_string(),
        })?;
        if catalog.version != CATALOG_VERSION {
            return Err(EffortBrainError::UnsupportedVersion {
                found: catalog.version,
                expected: CATALOG_VERSION,
            });
        }
        cfg.allow_model_overrides = catalog.defaults.allow_model_overrides;
        cfg.require_contrarian_on_heavy = catalog.defaults.require_contrarian_on_heavy;

        for entry in catalog.brains {
            let file_path = root.join(&entry.file);
            let raw = std::fs::read_to_string(&file_path).map_err(|e| EffortBrainError::Io {
                path: file_path.clone(),
                detail: e.to_string(),
            })?;
            let (fm, body) = parse_brain_markdown(&raw, &entry.id).map_err(|e| match e {
                EffortBrainError::BrainParse { id, detail, .. } => EffortBrainError::BrainParse {
                    id,
                    source: file_path.display().to_string(),
                    detail,
                },
                other => other,
            })?;
            if fm.id != entry.id {
                return Err(EffortBrainError::BrainIdMismatch {
                    expected: entry.id.clone(),
                    found: fm.id,
                    source: file_path.display().to_string(),
                });
            }
            let family = if !entry.family.is_empty() {
                entry.family
            } else {
                fm.family
            };
            let delta_role = if !entry.delta_role.is_empty() {
                entry.delta_role
            } else {
                fm.delta_role
            };
            let contrarian_class = entry.contrarian_class || fm.contrarian_class;
            cfg.brains.insert(
                entry.id.clone(),
                BrainSpec {
                    id: BrainId(entry.id),
                    family,
                    delta_role,
                    contrarian_class,
                    forbidden: fm.forbidden,
                    artifact_sections: fm.artifact_sections,
                    body_markdown: body,
                    model: entry.model,
                },
            );
        }
    }

    // Optional roster overrides (full replace per mode file).
    try_merge_roster(cfg, &root.join("rosters/heavy.toml"), "heavy", layer_name)?;
    try_merge_roster(cfg, &root.join("rosters/expert.toml"), "expert", layer_name)?;
    Ok(())
}

fn try_merge_roster(
    cfg: &mut EffortBrainConfig,
    path: &Path,
    mode: &str,
    _layer_name: &str,
) -> Result<(), EffortBrainError> {
    if !path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(path).map_err(|e| EffortBrainError::Io {
        path: path.to_path_buf(),
        detail: e.to_string(),
    })?;
    let rf: RosterFile = toml::from_str(&text).map_err(|e| EffortBrainError::RosterParse {
        mode: mode.into(),
        source: path.display().to_string(),
        detail: e.to_string(),
    })?;
    if rf.version != CATALOG_VERSION {
        return Err(EffortBrainError::UnsupportedVersion {
            found: rf.version,
            expected: CATALOG_VERSION,
        });
    }
    let selection = match rf.selection.as_str() {
        "fixed" => RosterSelection::Fixed {
            slots: rf.slots.into_iter().map(BrainId).collect(),
        },
        "random" => {
            let k = rf.k.unwrap_or(super::types::EXPERT_K_DEFAULT);
            let pool = match rf.pool.as_deref() {
                None | Some("all") => Vec::new(), // expanded later
                Some(list) if list.contains(',') => list
                    .split(',')
                    .map(|s| BrainId(s.trim().to_string()))
                    .filter(|b| !b.0.is_empty())
                    .collect(),
                Some(other) => {
                    return Err(EffortBrainError::RosterParse {
                        mode: mode.into(),
                        source: path.display().to_string(),
                        detail: format!("unsupported pool `{other}`"),
                    });
                }
            };
            RosterSelection::Random { k, pool }
        }
        other => {
            return Err(EffortBrainError::RosterParse {
                mode: mode.into(),
                source: path.display().to_string(),
                detail: format!("unknown selection `{other}`"),
            });
        }
    };
    match mode {
        "heavy" => cfg.heavy = selection,
        "expert" => cfg.expert = selection,
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_builtins_only() {
        let cfg = load_effort_brain_config_from_layers(None, None).expect("load");
        assert_eq!(cfg.brain_count(), 16);
        match &cfg.heavy {
            RosterSelection::Fixed { slots } => assert_eq!(slots.len(), 16),
            _ => panic!("heavy must be fixed"),
        }
        match &cfg.expert {
            RosterSelection::Random { k, pool } => {
                assert_eq!(*k, 4);
                assert_eq!(pool.len(), 16);
            }
            _ => panic!("expert must be random"),
        }
        assert!(cfg.get("red_team").unwrap().contrarian_class);
        assert!(cfg.get("inversion").unwrap().contrarian_class);
        assert!(cfg.get("pre_mortem").unwrap().contrarian_class);
        assert!(!cfg.get("first_principles").unwrap().contrarian_class);
    }

    #[test]
    fn project_override_wins_on_brain_body() {
        let dir = tempfile_dir("effort-brains-proj");
        let brains = dir.join("brains");
        std::fs::create_dir_all(&brains).unwrap();
        std::fs::write(
            dir.join("catalog.toml"),
            r#"
version = 1
[defaults]
require_contrarian_on_heavy = true
expert_k = 4
[[brains]]
id = "first_principles"
file = "brains/first_principles.md"
family = "foundations"
delta_role = "regenerate_solution_space"
contrarian_class = false
"#,
        )
        .unwrap();
        std::fs::write(
            brains.join("first_principles.md"),
            r#"---
id: first_principles
family: foundations
delta_role: regenerate_solution_space
contrarian_class: false
forbidden:
  - x
artifact_sections:
  - y
---

# Method
OVERRIDE_MARKER_PROJECT_LAYER
"#,
        )
        .unwrap();

        let cfg = load_effort_brain_config_from_layers(None, Some(&dir)).expect("load");
        assert!(
            cfg.get("first_principles")
                .unwrap()
                .body_markdown
                .contains("OVERRIDE_MARKER_PROJECT_LAYER"),
            "project body should win"
        );
        // other brains still present from defaults
        assert_eq!(cfg.brain_count(), 16);
    }

    #[test]
    fn invalid_heavy_roster_errors() {
        let dir = tempfile_dir("effort-brains-bad-heavy");
        std::fs::create_dir_all(dir.join("rosters")).unwrap();
        std::fs::write(
            dir.join("rosters/heavy.toml"),
            r#"
version = 1
mode = "heavy"
selection = "fixed"
slots = ["first_principles", "red_team"]
"#,
        )
        .unwrap();
        let err = load_effort_brain_config_from_layers(None, Some(&dir)).unwrap_err();
        match err {
            EffortBrainError::HeavySlotCount { found, expected } => {
                assert_eq!(found, 2);
                assert_eq!(expected, 16);
            }
            other => panic!("expected HeavySlotCount, got {other}"),
        }
    }

    fn tempfile_dir(prefix: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
