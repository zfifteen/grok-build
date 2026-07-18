//! Validation for resolved effort-brain configs (plan F8, F11, F13).

use super::error::EffortBrainError;
use super::types::{BRAIN_COUNT_HEAVY, EXPERT_K_DEFAULT, EffortBrainConfig, RosterSelection};

/// Validate a fully merged [`EffortBrainConfig`].
///
/// # Checks (v1)
/// - Heavy fixed slots length == 16
/// - Expert random k == 4 and pool.len() >= k
/// - Every roster id exists in catalog
/// - If `require_contrarian_on_heavy`, ≥1 slot brain has `contrarian_class`
pub fn validate_effort_brain_config(cfg: &EffortBrainConfig) -> Result<(), EffortBrainError> {
    validate_roster(
        "heavy",
        &cfg.heavy,
        &cfg.brains,
        cfg.require_contrarian_on_heavy,
    )?;
    validate_roster("expert", &cfg.expert, &cfg.brains, false)?;
    Ok(())
}

pub(crate) fn validate_roster(
    roster_name: &str,
    selection: &RosterSelection,
    brains: &std::collections::BTreeMap<String, super::types::BrainSpec>,
    require_contrarian: bool,
) -> Result<(), EffortBrainError> {
    match selection {
        RosterSelection::Fixed { slots } => {
            if roster_name == "heavy" && slots.len() != BRAIN_COUNT_HEAVY {
                return Err(EffortBrainError::HeavySlotCount {
                    found: slots.len(),
                    expected: BRAIN_COUNT_HEAVY,
                });
            }
            let mut any_contrarian = false;
            for id in slots {
                let spec =
                    brains
                        .get(id.as_str())
                        .ok_or_else(|| EffortBrainError::UnknownBrainId {
                            id: id.as_str().to_string(),
                            roster: roster_name.to_string(),
                        })?;
                if spec.contrarian_class {
                    any_contrarian = true;
                }
            }
            if require_contrarian && !any_contrarian {
                return Err(EffortBrainError::MissingContrarianClass);
            }
        }
        RosterSelection::Random { k, pool } => {
            if *k != EXPERT_K_DEFAULT {
                return Err(EffortBrainError::ExpertK {
                    found: *k,
                    expected: EXPERT_K_DEFAULT,
                });
            }
            if pool.len() < *k {
                return Err(EffortBrainError::ExpertPoolTooSmall {
                    pool: pool.len(),
                    k: *k,
                });
            }
            for id in pool {
                if !brains.contains_key(id.as_str()) {
                    return Err(EffortBrainError::UnknownBrainId {
                        id: id.as_str().to_string(),
                        roster: roster_name.to_string(),
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::effort_brains::defaults::built_in_effort_brain_config;

    #[test]
    fn built_in_config_validates() {
        let cfg = built_in_effort_brain_config().expect("defaults build");
        validate_effort_brain_config(&cfg).expect("defaults valid");
    }
}
