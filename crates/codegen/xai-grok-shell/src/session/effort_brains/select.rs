//! Select which brain ids run for Expert (random k) vs Heavy (fixed).

use super::error::EffortBrainError;
use super::types::{BrainId, EffortBrainConfig, RosterSelection};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

/// Env var for deterministic Expert draws (plan F10).
pub const EFFORT_BRAIN_SEED_ENV: &str = "GROK_EFFORT_BRAIN_SEED";

/// Resolve ordered brain ids for a team run under `cfg`.
pub fn select_brain_ids(
    cfg: &EffortBrainConfig,
    mode: crate::session::effort_mode::EffortMode,
) -> Result<Vec<BrainId>, EffortBrainError> {
    use crate::session::effort_mode::EffortMode;
    let selection = match mode {
        EffortMode::Normal => {
            return Err(EffortBrainError::RosterParse {
                mode: "normal".into(),
                source: "select".into(),
                detail: "no brain roster for Normal mode".into(),
            });
        }
        EffortMode::Expert => &cfg.expert,
        EffortMode::Heavy => &cfg.heavy,
    };
    match selection {
        RosterSelection::Fixed { slots } => Ok(slots.clone()),
        RosterSelection::Random { k, pool } => {
            if pool.len() < *k {
                return Err(EffortBrainError::ExpertPoolTooSmall {
                    pool: pool.len(),
                    k: *k,
                });
            }
            let mut pool = pool.clone();
            match effort_brain_seed() {
                Some(seed) => {
                    let mut rng = StdRng::seed_from_u64(seed);
                    pool.shuffle(&mut rng);
                }
                None => {
                    let mut rng = rand::rng();
                    pool.shuffle(&mut rng);
                }
            }
            pool.truncate(*k);
            Ok(pool)
        }
    }
}

fn effort_brain_seed() -> Option<u64> {
    std::env::var(EFFORT_BRAIN_SEED_ENV)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::effort_brains::load_effort_brain_config_from_layers;
    use crate::session::effort_mode::EffortMode;

    #[test]
    fn heavy_returns_all_16_fixed_order() {
        let cfg = load_effort_brain_config_from_layers(None, None).unwrap();
        let ids = select_brain_ids(&cfg, EffortMode::Heavy).unwrap();
        assert_eq!(ids.len(), 16);
        assert_eq!(ids[0].as_str(), "first_principles");
        assert_eq!(ids[15].as_str(), "red_team");
        let again = select_brain_ids(&cfg, EffortMode::Heavy).unwrap();
        assert_eq!(ids, again);
    }

    #[test]
    fn expert_seed_reproducible_and_unique() {
        let cfg = load_effort_brain_config_from_layers(None, None).unwrap();
        unsafe { std::env::set_var(EFFORT_BRAIN_SEED_ENV, "42") };
        let a = select_brain_ids(&cfg, EffortMode::Expert).unwrap();
        let b = select_brain_ids(&cfg, EffortMode::Expert).unwrap();
        unsafe { std::env::remove_var(EFFORT_BRAIN_SEED_ENV) };
        assert_eq!(a.len(), 4);
        assert_eq!(a, b);
        let mut sorted = a.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 4, "must be unique");
    }
}
