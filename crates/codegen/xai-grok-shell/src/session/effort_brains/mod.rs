//! Effort reasoning brains — load, validate, and select specialist protocols.
//!
//! # Product contract (plan §0)
//! - **16** reasoning protocols (not cosplay personas).
//! - **Heavy** uses all 16 in fixed order.
//! - **Expert** samples a **random 4 of 16** per team run.
//! - Config root: `$GROK_HOME/effort-brains/` with project
//!   `.powergrok/effort-brains/` merge-by-id override.
//! - Invalid config fails loud; never silently fall back to angle cycling.
//!
//! # Module layout
//! - [`types`] — pure data shapes
//! - [`error`] — load/validate failures
//! - [`defaults`] — embedded built-in catalog (Task 2)
//! - [`load`] — parse TOML/MD + merge layers (Task 3+)
//! - [`validate`] — roster/catalog invariants (Task 3)
//! - [`select`] — Expert random / Heavy fixed (later task)
//! - [`prompt`] — specialist prompt render (later task)
//!
//! This module is pure/session-adjacent: no live model I/O.

pub mod defaults;
pub mod error;
pub mod load;
pub mod prompt;
pub mod seed;
pub mod select;
pub mod types;
pub mod validate;

pub use error::EffortBrainError;
pub use load::{load_effort_brain_config, load_effort_brain_config_from_layers};
pub use prompt::{brain_description, render_specialist_prompt};
pub use seed::{ensure_user_effort_brains_seeded, export_builtin_effort_brains_to};
pub use select::{select_brain_ids, EFFORT_BRAIN_SEED_ENV};
pub use types::{
    BrainId, BrainSpec, EffortBrainConfig, RosterMode, RosterSelection, BRAIN_COUNT_HEAVY,
    EXPERT_K_DEFAULT,
};
pub use validate::validate_effort_brain_config;
