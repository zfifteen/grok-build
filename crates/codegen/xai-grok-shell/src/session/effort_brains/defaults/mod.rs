//! Embedded built-in effort-brain defaults (16 protocols + rosters).
//!
//! Loaded when user/project layers are absent. Bodies are short high-signal
//! protocols (plan F14), not essays or cosplay.

use super::error::EffortBrainError;
use super::types::{
    BrainFrontmatter, BrainId, BrainSpec, CATALOG_VERSION, CatalogFile, EffortBrainConfig,
    RosterFile, RosterSelection,
};
use std::collections::BTreeMap;

macro_rules! brain_md {
    ($name:expr) => {
        include_str!(concat!("brains/", $name, ".md"))
    };
}

const CATALOG_TOML: &str = include_str!("catalog.toml");
const HEAVY_ROSTER_TOML: &str = include_str!("rosters/heavy.toml");
const EXPERT_ROSTER_TOML: &str = include_str!("rosters/expert.toml");

/// Build the built-in [`EffortBrainConfig`] from embedded strings.
pub fn built_in_effort_brain_config() -> Result<EffortBrainConfig, EffortBrainError> {
    let catalog: CatalogFile =
        toml::from_str(CATALOG_TOML).map_err(|e| EffortBrainError::CatalogParse {
            source: "builtin:catalog.toml".into(),
            detail: e.to_string(),
        })?;
    if catalog.version != CATALOG_VERSION {
        return Err(EffortBrainError::UnsupportedVersion {
            found: catalog.version,
            expected: CATALOG_VERSION,
        });
    }

    let mut brains = BTreeMap::new();
    for entry in &catalog.brains {
        if brains.contains_key(&entry.id) {
            return Err(EffortBrainError::DuplicateBrainId {
                id: entry.id.clone(),
            });
        }
        let raw =
            builtin_brain_markdown(&entry.id).ok_or_else(|| EffortBrainError::MissingFile {
                path: format!("builtin:brains/{}.md", entry.id).into(),
            })?;
        let (fm, body) = parse_brain_markdown(raw, &entry.id)?;
        if fm.id != entry.id {
            return Err(EffortBrainError::BrainIdMismatch {
                expected: entry.id.clone(),
                found: fm.id,
                source: format!("builtin:brains/{}.md", entry.id),
            });
        }
        // Catalog entry may override family/delta/contrarian/model metadata.
        let family = if !entry.family.is_empty() {
            entry.family.clone()
        } else {
            fm.family
        };
        let delta_role = if !entry.delta_role.is_empty() {
            entry.delta_role.clone()
        } else {
            fm.delta_role
        };
        let contrarian_class = entry.contrarian_class || fm.contrarian_class;
        brains.insert(
            entry.id.clone(),
            BrainSpec {
                id: BrainId(entry.id.clone()),
                family,
                delta_role,
                contrarian_class,
                forbidden: fm.forbidden,
                artifact_sections: fm.artifact_sections,
                body_markdown: body,
                model: entry.model.clone(),
            },
        );
    }

    let heavy = parse_roster(HEAVY_ROSTER_TOML, "heavy")?;
    let expert = parse_roster(EXPERT_ROSTER_TOML, "expert")?;

    let mut cfg = EffortBrainConfig {
        brains,
        expert,
        heavy,
        allow_model_overrides: catalog.defaults.allow_model_overrides,
        require_contrarian_on_heavy: catalog.defaults.require_contrarian_on_heavy,
    };
    expand_expert_pool_all(&mut cfg);
    super::validate::validate_effort_brain_config(&cfg)?;
    Ok(cfg)
}

fn parse_roster(toml_src: &str, expected_mode: &str) -> Result<RosterSelection, EffortBrainError> {
    let rf: RosterFile = toml::from_str(toml_src).map_err(|e| EffortBrainError::RosterParse {
        mode: expected_mode.into(),
        source: format!("builtin:rosters/{expected_mode}.toml"),
        detail: e.to_string(),
    })?;
    if rf.version != CATALOG_VERSION {
        return Err(EffortBrainError::UnsupportedVersion {
            found: rf.version,
            expected: CATALOG_VERSION,
        });
    }
    if rf.mode != expected_mode {
        return Err(EffortBrainError::RosterParse {
            mode: expected_mode.into(),
            source: format!("builtin:rosters/{expected_mode}.toml"),
            detail: format!("mode field is `{}`, expected `{expected_mode}`", rf.mode),
        });
    }
    match rf.selection.as_str() {
        "fixed" => Ok(RosterSelection::Fixed {
            slots: rf.slots.into_iter().map(BrainId).collect(),
        }),
        "random" => {
            let k = rf.k.unwrap_or(super::types::EXPERT_K_DEFAULT);
            let pool = match rf.pool.as_deref() {
                None | Some("all") => {
                    // Filled after catalog known — caller of random with pool=all
                    // uses all catalog ids. For builtin expert.toml we list nothing
                    // and expand below in load when needed.
                    // Here: empty means "all" sentinel; expand in built_in after brains map.
                    Vec::new()
                }
                Some(other) => {
                    return Err(EffortBrainError::RosterParse {
                        mode: expected_mode.into(),
                        source: format!("builtin:rosters/{expected_mode}.toml"),
                        detail: format!("unsupported pool value `{other}`"),
                    });
                }
            };
            Ok(RosterSelection::Random { k, pool })
        }
        other => Err(EffortBrainError::RosterParse {
            mode: expected_mode.into(),
            source: format!("builtin:rosters/{expected_mode}.toml"),
            detail: format!("unknown selection `{other}`"),
        }),
    }
}

/// Split `---\n yaml \n---\n body` frontmatter.
pub(crate) fn parse_brain_markdown(
    raw: &str,
    fallback_id: &str,
) -> Result<(BrainFrontmatter, String), EffortBrainError> {
    let raw = raw.trim_start_matches('\u{feff}');
    if !raw.starts_with("---") {
        return Err(EffortBrainError::BrainParse {
            id: fallback_id.into(),
            source: format!("builtin:brains/{fallback_id}.md"),
            detail: "missing YAML frontmatter starting with ---".into(),
        });
    }
    let rest = &raw[3..];
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let Some(end) = rest.find("\n---") else {
        return Err(EffortBrainError::BrainParse {
            id: fallback_id.into(),
            source: format!("builtin:brains/{fallback_id}.md"),
            detail: "missing closing --- for frontmatter".into(),
        });
    };
    let yaml = &rest[..end];
    let body = rest[end + 4..].trim_start_matches('\n').to_string();
    let fm: BrainFrontmatter =
        serde_yaml::from_str(yaml).map_err(|e| EffortBrainError::BrainParse {
            id: fallback_id.into(),
            source: format!("builtin:brains/{fallback_id}.md"),
            detail: e.to_string(),
        })?;
    Ok((fm, body))
}

fn builtin_brain_markdown(id: &str) -> Option<&'static str> {
    Some(match id {
        "first_principles" => brain_md!("first_principles"),
        "map_territory" => brain_md!("map_territory"),
        "circle_of_competence" => brain_md!("circle_of_competence"),
        "systems_loops" => brain_md!("systems_loops"),
        "theory_of_constraints" => brain_md!("theory_of_constraints"),
        "five_whys_root" => brain_md!("five_whys_root"),
        "second_order" => brain_md!("second_order"),
        "inversion" => brain_md!("inversion"),
        "pre_mortem" => brain_md!("pre_mortem"),
        "scientific_method" => brain_md!("scientific_method"),
        "bayesian_update" => brain_md!("bayesian_update"),
        "fermi_estimate" => brain_md!("fermi_estimate"),
        "via_negativa" => brain_md!("via_negativa"),
        "ooda_tempo" => brain_md!("ooda_tempo"),
        "steelman_dialectic" => brain_md!("steelman_dialectic"),
        "red_team" => brain_md!("red_team"),
        _ => return None,
    })
}

/// Expand Expert `pool = all` (empty pool vec) to every catalog id in stable order.
pub(crate) fn expand_expert_pool_all(cfg: &mut EffortBrainConfig) {
    #[allow(clippy::collapsible_if)]
    if let RosterSelection::Random { k: _, pool } = &mut cfg.expert {
        if pool.is_empty() {
            *pool = cfg.brains.keys().cloned().map(BrainId).collect();
        }
    }
}
