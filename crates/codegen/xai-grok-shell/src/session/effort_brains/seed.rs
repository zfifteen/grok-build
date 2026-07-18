//! Export built-in effort brains to the user's config tree for editing.

use super::error::EffortBrainError;
use std::path::{Path, PathBuf};

/// Write the shipped default catalog, rosters, and brain markdown into `root`.
///
/// Creates directories as needed. Overwrites existing files (callers should
/// only invoke when seeding a missing tree, or after explicit user request).
pub fn export_builtin_effort_brains_to(root: &Path) -> Result<(), EffortBrainError> {
    std::fs::create_dir_all(root.join("brains")).map_err(|e| EffortBrainError::Io {
        path: root.to_path_buf(),
        detail: e.to_string(),
    })?;
    std::fs::create_dir_all(root.join("rosters")).map_err(|e| EffortBrainError::Io {
        path: root.to_path_buf(),
        detail: e.to_string(),
    })?;

    write_file(
        &root.join("catalog.toml"),
        include_str!("defaults/catalog.toml"),
    )?;
    write_file(
        &root.join("rosters/heavy.toml"),
        include_str!("defaults/rosters/heavy.toml"),
    )?;
    write_file(
        &root.join("rosters/expert.toml"),
        include_str!("defaults/rosters/expert.toml"),
    )?;

    const BRAINS: &[(&str, &str)] = &[
        (
            "first_principles",
            include_str!("defaults/brains/first_principles.md"),
        ),
        (
            "map_territory",
            include_str!("defaults/brains/map_territory.md"),
        ),
        (
            "circle_of_competence",
            include_str!("defaults/brains/circle_of_competence.md"),
        ),
        (
            "systems_loops",
            include_str!("defaults/brains/systems_loops.md"),
        ),
        (
            "theory_of_constraints",
            include_str!("defaults/brains/theory_of_constraints.md"),
        ),
        (
            "five_whys_root",
            include_str!("defaults/brains/five_whys_root.md"),
        ),
        (
            "second_order",
            include_str!("defaults/brains/second_order.md"),
        ),
        ("inversion", include_str!("defaults/brains/inversion.md")),
        ("pre_mortem", include_str!("defaults/brains/pre_mortem.md")),
        (
            "scientific_method",
            include_str!("defaults/brains/scientific_method.md"),
        ),
        (
            "bayesian_update",
            include_str!("defaults/brains/bayesian_update.md"),
        ),
        (
            "fermi_estimate",
            include_str!("defaults/brains/fermi_estimate.md"),
        ),
        (
            "via_negativa",
            include_str!("defaults/brains/via_negativa.md"),
        ),
        ("ooda_tempo", include_str!("defaults/brains/ooda_tempo.md")),
        (
            "steelman_dialectic",
            include_str!("defaults/brains/steelman_dialectic.md"),
        ),
        ("red_team", include_str!("defaults/brains/red_team.md")),
    ];
    for (id, body) in BRAINS {
        write_file(&root.join(format!("brains/{id}.md")), body)?;
    }

    // Small README so operators know this is editable (no cosplay).
    write_file(
        &root.join("README.md"),
        r#"# Effort reasoning brains

These files define the 16 **reasoning protocols** used by Expert (random 4) and
Heavy (all 16). Edit method text freely; keep frontmatter `id` stable.

- Not character personas — method / forbidden moves / artifacts only.
- Heavy roster: `rosters/heavy.toml` (must list 16 ids; ≥1 contrarian_class).
- Expert: `rosters/expert.toml` (`selection = "random"`, `k = 4`).
- Optional model overrides require `allow_model_overrides = true` in `catalog.toml`
  plus `model = "..."` on a brain entry.

Seeded from product defaults. Safe to re-seed only after backing up edits.
"#,
    )?;
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<(), EffortBrainError> {
    std::fs::write(path, contents).map_err(|e| EffortBrainError::Io {
        path: path.to_path_buf(),
        detail: e.to_string(),
    })
}

/// If `$GROK_HOME/effort-brains` does not exist, seed it from built-ins.
///
/// Returns `Some(path)` when a seed was written, `None` if already present or
/// no user home, `Err` on I/O failure.
///
/// Concurrent-safe: uses exclusive `create_dir` so only one seeder writes;
/// waiters poll for `catalog.toml` briefly, then fall back to built-ins only.
pub fn ensure_user_effort_brains_seeded() -> Result<Option<PathBuf>, EffortBrainError> {
    let Some(home) = xai_grok_config::user_grok_home() else {
        return Ok(None);
    };
    let root = home.join("effort-brains");
    if root.join("catalog.toml").is_file() {
        return Ok(None);
    }
    // Ensure parent exists.
    if let Some(parent) = root.parent() {
        std::fs::create_dir_all(parent).map_err(|e| EffortBrainError::Io {
            path: parent.to_path_buf(),
            detail: e.to_string(),
        })?;
    }
    match std::fs::create_dir(&root) {
        Ok(()) => {
            export_builtin_effort_brains_to(&root)?;
            tracing::info!(path = %root.display(), "seeded effort-brains config tree from built-ins");
            Ok(Some(root))
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another process/thread is seeding or tree is partial — wait for catalog.
            for _ in 0..50 {
                if root.join("catalog.toml").is_file() {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            // Incomplete foreign seed: repair by re-export (overwrite).
            if !root.join("catalog.toml").is_file() {
                export_builtin_effort_brains_to(&root)?;
                return Ok(Some(root));
            }
            Ok(None)
        }
        Err(e) => Err(EffortBrainError::Io {
            path: root,
            detail: e.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_writes_catalog_and_16_brains() {
        let dir = std::env::temp_dir().join(format!(
            "effort-export-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        export_builtin_effort_brains_to(&dir).unwrap();
        assert!(dir.join("catalog.toml").is_file());
        assert!(dir.join("rosters/heavy.toml").is_file());
        assert!(dir.join("brains/red_team.md").is_file());
        let n = std::fs::read_dir(dir.join("brains")).unwrap().count();
        assert_eq!(n, 16);
        // reload via load layers
        let cfg =
            crate::session::effort_brains::load_effort_brain_config_from_layers(Some(&dir), None)
                .unwrap();
        assert_eq!(cfg.brain_count(), 16);
    }
}
