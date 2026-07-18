//! Error types for effort brain catalog load and validation.

use std::path::PathBuf;

/// Failure loading or validating an effort-brain configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffortBrainError {
    /// Built-in or on-disk catalog TOML could not be parsed.
    CatalogParse { source: String, detail: String },
    /// Roster TOML could not be parsed.
    RosterParse {
        mode: String,
        source: String,
        detail: String,
    },
    /// Brain markdown/frontmatter could not be parsed.
    BrainParse {
        id: String,
        source: String,
        detail: String,
    },
    /// Catalog `version` is unsupported.
    UnsupportedVersion { found: u32, expected: u32 },
    /// Roster references an id missing from the catalog.
    UnknownBrainId { id: String, roster: String },
    /// Heavy fixed roster length is not 16 (unless experimental later).
    HeavySlotCount { found: usize, expected: usize },
    /// Expert k is not 4 (v1 hard DoD).
    ExpertK { found: usize, expected: usize },
    /// Expert pool has fewer ids than k.
    ExpertPoolTooSmall { pool: usize, k: usize },
    /// Heavy requires ≥1 contrarian_class brain among slots.
    MissingContrarianClass,
    /// Frontmatter id does not match catalog entry / filename.
    BrainIdMismatch {
        expected: String,
        found: String,
        source: String,
    },
    /// Required file missing on an override layer.
    MissingFile { path: PathBuf },
    /// I/O failure reading an override layer.
    Io { path: PathBuf, detail: String },
    /// Duplicate brain id in a single catalog layer.
    DuplicateBrainId { id: String },
}

impl std::fmt::Display for EffortBrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Rich user-facing messages; implement in Phase 3 Task 1 fill-in.
        // PHASE1_SCAFFOLD: describe each variant for operators.
        match self {
            Self::CatalogParse { source, detail } => {
                write!(f, "effort-brains catalog parse failed ({source}): {detail}")
            }
            Self::RosterParse {
                mode,
                source,
                detail,
            } => write!(
                f,
                "effort-brains {mode} roster parse failed ({source}): {detail}"
            ),
            Self::BrainParse { id, source, detail } => {
                write!(
                    f,
                    "effort-brains brain `{id}` parse failed ({source}): {detail}"
                )
            }
            Self::UnsupportedVersion { found, expected } => {
                write!(
                    f,
                    "effort-brains unsupported catalog version {found} (expected {expected})"
                )
            }
            Self::UnknownBrainId { id, roster } => {
                write!(
                    f,
                    "effort-brains roster `{roster}` references unknown id `{id}`"
                )
            }
            Self::HeavySlotCount { found, expected } => {
                write!(
                    f,
                    "effort-brains heavy roster has {found} slots (expected {expected})"
                )
            }
            Self::ExpertK { found, expected } => {
                write!(
                    f,
                    "effort-brains expert k={found} (expected {expected} in v1)"
                )
            }
            Self::ExpertPoolTooSmall { pool, k } => {
                write!(f, "effort-brains expert pool size {pool} < k={k}")
            }
            Self::MissingContrarianClass => write!(
                f,
                "effort-brains heavy roster needs ≥1 contrarian_class brain (inversion|pre_mortem|red_team)"
            ),
            Self::BrainIdMismatch {
                expected,
                found,
                source,
            } => write!(
                f,
                "effort-brains id mismatch at {source}: expected `{expected}`, found `{found}`"
            ),
            Self::MissingFile { path } => {
                write!(f, "effort-brains missing file {}", path.display())
            }
            Self::Io { path, detail } => {
                write!(f, "effort-brains io error at {}: {detail}", path.display())
            }
            Self::DuplicateBrainId { id } => {
                write!(f, "effort-brains duplicate brain id `{id}`")
            }
        }
    }
}

impl std::error::Error for EffortBrainError {}
