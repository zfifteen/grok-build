//! Pure data types for effort reasoning brains.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Heavy fixed team size / catalog size (product DoD).
pub const BRAIN_COUNT_HEAVY: usize = 16;
/// Expert sample size (product DoD).
pub const EXPERT_K_DEFAULT: usize = 4;
/// Supported catalog schema version.
pub const CATALOG_VERSION: u32 = 1;

/// Stable brain identifier (e.g. `first_principles`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BrainId(pub String);

impl BrainId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BrainId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for BrainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One reasoning protocol (method + constraints + artifact shape).
///
/// Not a persona. Value is the **delta** this method produces vs other brains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrainSpec {
    pub id: BrainId,
    pub family: String,
    pub delta_role: String,
    pub contrarian_class: bool,
    pub forbidden: Vec<String>,
    pub artifact_sections: Vec<String>,
    /// Markdown body after frontmatter (Method / Stop rules / …).
    pub body_markdown: String,
    /// Optional model override; only honored if config allows.
    pub model: Option<String>,
}

/// How a mode picks brains for a team run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RosterSelection {
    /// Heavy (and debug fixed Expert): exact ordered slot list.
    Fixed { slots: Vec<BrainId> },
    /// Expert v1: uniform sample of `k` from `pool` without replacement.
    Random { k: usize, pool: Vec<BrainId> },
}

/// Which effort mode a roster file targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterMode {
    Expert,
    Heavy,
}

/// Fully resolved, validated brain configuration ready for brief building.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffortBrainConfig {
    /// id → spec (merged layers).
    pub brains: BTreeMap<String, BrainSpec>,
    pub expert: RosterSelection,
    pub heavy: RosterSelection,
    pub allow_model_overrides: bool,
    pub require_contrarian_on_heavy: bool,
}

impl EffortBrainConfig {
    /// Look up a brain by id string.
    pub fn get(&self, id: &str) -> Option<&BrainSpec> {
        self.brains.get(id)
    }

    pub fn brain_count(&self) -> usize {
        self.brains.len()
    }
}

// ── Wire formats (TOML) ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CatalogFile {
    pub version: u32,
    #[serde(default)]
    pub defaults: CatalogDefaults,
    #[serde(default)]
    pub brains: Vec<CatalogBrainEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct CatalogDefaults {
    #[serde(default)]
    pub allow_model_overrides: bool,
    #[serde(default = "default_true")]
    pub require_contrarian_on_heavy: bool,
    #[serde(default = "default_expert_k")]
    pub expert_k: usize,
}

fn default_true() -> bool {
    true
}
fn default_expert_k() -> usize {
    EXPERT_K_DEFAULT
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CatalogBrainEntry {
    pub id: String,
    pub file: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub delta_role: String,
    #[serde(default)]
    pub contrarian_class: bool,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RosterFile {
    pub version: u32,
    pub mode: String,
    pub selection: String,
    #[serde(default)]
    pub k: Option<usize>,
    #[serde(default)]
    pub pool: Option<String>,
    #[serde(default)]
    pub slots: Vec<String>,
}

/// YAML frontmatter on a brain markdown file.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BrainFrontmatter {
    pub id: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub delta_role: String,
    #[serde(default)]
    pub contrarian_class: bool,
    #[serde(default)]
    pub forbidden: Vec<String>,
    #[serde(default)]
    pub artifact_sections: Vec<String>,
}
