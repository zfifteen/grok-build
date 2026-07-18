//! Effort mode (Expert / Heavy / Normal) session state machine and hard gates.
//!
//! Parallel to [`super::plan_mode::PlanModeTracker`]: pure FSM, no I/O.
//! SessionActor owns one tracker; slash builtins set sticky mode; hard
//! join/ledger/replace/execute rules live here for unit tests with faked
//! specialist outcomes (no live model required).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

// ── Feature flag ───────────────────────────────────────────────────────────

/// Env kill-switch for builtin `/expert` `/heavy` `/normal` registration.
/// Default **on** (unset → enabled). Set `GROK_EFFORT_MODE_BUILTINS=0` to
/// omit the three names so skills can reclaim slash resolution.
pub const EFFORT_MODE_BUILTINS_ENV: &str = "GROK_EFFORT_MODE_BUILTINS";

/// Whether effort-mode slash builtins are enabled.
pub fn effort_mode_builtins_enabled() -> bool {
    match std::env::var(EFFORT_MODE_BUILTINS_ENV) {
        Ok(v) => {
            let t = v.trim().to_ascii_lowercase();
            !(t == "0" || t == "false" || t == "off" || t == "no" || t == "disable")
        }
        Err(_) => true,
    }
}

// ── EffortMode ─────────────────────────────────────────────────────────────

/// Session-scoped effort policy, orthogonal to plan/permission modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EffortMode {
    #[default]
    Normal,
    Expert,
    Heavy,
}

impl EffortMode {
    /// Default DoD team sizes (tech-spec §6 / §10).
    pub fn team_size_default(self) -> Option<usize> {
        match self {
            Self::Normal => None,
            Self::Expert => Some(4),
            Self::Heavy => Some(16),
        }
    }

    /// Heavy requires ≥1 successful contrarian specialist.
    pub fn requires_contrarian(self) -> bool {
        matches!(self, Self::Heavy)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Expert => "expert",
            Self::Heavy => "heavy",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "expert" => Some(Self::Expert),
            "heavy" => Some(Self::Heavy),
            _ => None,
        }
    }

    /// Whether multi-agent effort runtime is active (not Normal).
    pub fn is_elevated(self) -> bool {
        !matches!(self, Self::Normal)
    }

    /// Relative cost/time class vs a single Normal analytic pass (approximate).
    ///
    /// Not dollars — metering is incomplete. Labels must stay approximate.
    pub fn cost_class(self) -> EffortCostClass {
        match self {
            Self::Normal => EffortCostClass::Normal,
            Self::Expert => EffortCostClass::ExpertMulti,
            Self::Heavy => EffortCostClass::HeavyMulti,
        }
    }
}

/// Approximate relative cost/time class for operator transparency (issue #8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffortCostClass {
    Normal,
    /// ~N=4 analytic specialists + leader synthesis.
    ExpertMulti,
    /// ~N=16 analytic specialists + leader synthesis — slow & costly.
    HeavyMulti,
}

impl EffortCostClass {
    pub fn relative_multiplier(self) -> u32 {
        match self {
            Self::Normal => 1,
            Self::ExpertMulti => 4,
            Self::HeavyMulti => 16,
        }
    }

    /// Short operator-facing class label (never claims exact $).
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "≈1× Normal analytic pass",
            Self::ExpertMulti => "≈4× analytic pass (Expert team — multi-agent)",
            Self::HeavyMulti => "≈16× analytic pass (Heavy team — slow & costly)",
        }
    }
}

/// Default join-all floor from tech-spec §6 (ms). Soft — not a hard kill.
pub const EFFORT_JOIN_TIMEOUT_FLOOR_MS: u64 = 300_000;

/// Soft budget env: max specialist N before a **warn-only** preflight note.
/// Unset / invalid → no soft-budget warning. Never crashes the session.
pub const EFFORT_SOFT_BUDGET_N_ENV: &str = "GROK_EFFORT_SOFT_BUDGET_N";

/// Parse soft budget N from env; illegal values clamp to `None` (no warn).
pub fn soft_budget_n_from_env() -> Option<usize> {
    let v = std::env::var(EFFORT_SOFT_BUDGET_N_ENV).ok()?;
    let n: usize = v.trim().parse().ok()?;
    if n == 0 {
        return None;
    }
    // Clamp absurd values so misconfig cannot hard-fail product paths.
    Some(n.min(10_000))
}

/// Format compact elapsed wall time for chrome / footers.
pub fn format_elapsed_compact(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("{m}m")
        } else {
            format!("{m}m{s:02}s")
        }
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{h}h{m:02}m")
    }
}

/// Operator-visible pre-flight summary before a multi-agent fan-out (issue #8).
///
/// Returns `None` for Normal (no scary Heavy chrome on Normal/waived paths).
pub fn format_effort_preflight(mode: EffortMode) -> Option<String> {
    let n = mode.team_size_default()?;
    let class = mode.cost_class();
    let join_floor_s = EFFORT_JOIN_TIMEOUT_FLOOR_MS / 1000;
    let mut lines = vec![
        format!(
            "**Effort pre-flight:** sticky **{mode}** will run a **multi-agent** fixed team of **N={n}** specialists (plus leader synthesis).",
            mode = match mode {
                EffortMode::Expert => "Expert",
                EffortMode::Heavy => "Heavy",
                EffortMode::Normal => return None,
            }
        ),
        format!("Relative cost/time class (approximate, not $): {}", class.label()),
        format!(
            "Join-all timeout floor ≈ {join_floor_s}s per tech-spec (wall clock can be longer with replace waves)."
        ),
        "Live chrome shows **S of N** + elapsed while pursuing. **Abort:** `/normal` (or cancel) — partial S of N, never a full-team success claim.".to_string(),
    ];
    if let Some(budget) = soft_budget_n_from_env().filter(|&b| n > b) {
        lines.push(format!(
                "**Soft budget warn:** N={n} exceeds GROK_EFFORT_SOFT_BUDGET_N={budget} (warn only — session continues)."
            ));
    }

    if mode == EffortMode::Heavy {
        lines.push(
            "First Heavy multi-agent team this session still needs `--confirm` (or GROK_HEAVY_AUTO_CONFIRM=1)."
                .into(),
        );
    }
    Some(lines.join("\n"))
}

/// One-line post-run cost/time footer for calibration (issue #8).
pub fn format_effort_run_footer(
    mode: EffortMode,
    successful: usize,
    target_n: usize,
    elapsed_secs: Option<u64>,
    partial: bool,
) -> String {
    let mode_name = match mode {
        EffortMode::Expert => "Expert",
        EffortMode::Heavy => "Heavy",
        EffortMode::Normal => "Normal",
    };
    let outcome = if partial {
        format!("partial {successful} of {target_n}")
    } else {
        format!("{successful} of {target_n} specialists")
    };
    let elapsed = elapsed_secs
        .map(|s| format!(" · {}", format_elapsed_compact(s)))
        .unwrap_or_default();
    format!(
        "Effort run: {mode_name} · {outcome}{elapsed} · cost class {} (approx; tokens/$ if metering available elsewhere)",
        mode.cost_class().label()
    )
}

impl std::fmt::Display for EffortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Pursuit / ledger ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PursuitState {
    #[default]
    Idle,
    Pursuing,
    Aborting,
    PartialReport,
    Waived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialistStatus {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
    Timeout,
    EmptyReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialistLedgerRow {
    pub slot: usize,
    pub role: String,
    pub task_id: Option<String>,
    pub status: SpecialistStatus,
    pub counts_toward_n: bool,
    pub rewaits: u32,
    pub replaces: u32,
    pub is_contrarian: bool,
    /// Writers that run after synthesis are labeled outside N.
    pub outside_n: bool,
}

impl SpecialistLedgerRow {
    pub fn new(slot: usize, role: impl Into<String>) -> Self {
        Self {
            slot,
            role: role.into(),
            task_id: None,
            status: SpecialistStatus::Pending,
            counts_toward_n: false,
            rewaits: 0,
            replaces: 0,
            is_contrarian: false,
            outside_n: false,
        }
    }
}

/// Caps from tech-spec §6 defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffortCaps {
    pub max_rewait_per_slot: u32,
    pub max_replace_per_slot: u32,
    pub max_replace_waves: u32,
}

impl Default for EffortCaps {
    fn default() -> Self {
        Self {
            max_rewait_per_slot: 1,
            max_replace_per_slot: 1,
            max_replace_waves: 2,
        }
    }
}

// ── Snapshot ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffortModeSnapshot {
    pub mode: EffortMode,
    pub pursuit: PursuitState,
    #[serde(default)]
    pub solo_waiver: bool,
    #[serde(default)]
    pub synthesis_complete: bool,
    #[serde(default)]
    pub replace_waves_used: u32,
    #[serde(default)]
    pub ledger: Vec<SpecialistLedgerRow>,
    /// Session already unlocked first Heavy multi-agent fan-out (or auto-confirm).
    #[serde(default)]
    pub heavy_unlocked: bool,
}

// ── Tracker ────────────────────────────────────────────────────────────────

/// Why elevated mode is not running a full team this turn (chrome label).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaiverReason {
    #[default]
    None,
    /// Operator `--solo` / sticky solo.
    Solo,
    /// Conservative trivial short-circuit.
    Trivial,
    /// First Heavy team spawn waiting for explicit confirm this session.
    NeedsHeavyConfirm,
}

/// Pure effort-mode session tracker (plan-mode twin).
pub struct EffortModeTracker {
    mode: EffortMode,
    pursuit: PursuitState,
    /// Per-turn / sticky solo waiver (`--solo`).
    solo_waiver: bool,
    /// Per-turn force full team even if classifier would waive trivial.
    force_team: bool,
    /// First Heavy multi-agent spawn unlocked for this session.
    heavy_unlocked: bool,
    /// Last waiver reason for chrome (Solo vs Trivial vs NeedsHeavyConfirm).
    last_waiver: WaiverReason,
    /// Set when restoring elevated sticky mode from disk (resume banner).
    resume_elevated_notice: bool,
    /// True after successful synthesis of the current team run.
    synthesis_complete: bool,
    replace_waves_used: u32,
    /// Extra wave granted by hard-stop `continue` (at most one pending wave).
    continue_wave_pending: bool,
    ledger: Vec<SpecialistLedgerRow>,
    caps: EffortCaps,
    session_dir: PathBuf,
    /// Wall clock when the current fixed-team run began (live elapsed chrome).
    team_started_at: Option<Instant>,
    /// Last completed run duration (post-run footer).
    last_run_elapsed_secs: Option<u64>,
    /// Pending one-shot preflight operator message after begin_team_run.
    pending_preflight: bool,
    /// Pending one-shot post-run footer after team terminal.
    pending_run_footer: bool,
}

impl EffortModeTracker {
    pub fn new(session_dir: PathBuf) -> Self {
        Self {
            mode: EffortMode::Normal,
            pursuit: PursuitState::Idle,
            solo_waiver: false,
            force_team: false,
            heavy_unlocked: heavy_auto_confirm_from_env(),
            last_waiver: WaiverReason::None,
            resume_elevated_notice: false,
            synthesis_complete: false,
            replace_waves_used: 0,
            continue_wave_pending: false,
            ledger: Vec::new(),
            caps: EffortCaps::default(),
            session_dir,
            team_started_at: None,
            last_run_elapsed_secs: None,
            pending_preflight: false,
            pending_run_footer: false,
        }
    }

    pub fn from_snapshot(session_dir: PathBuf, snapshot: EffortModeSnapshot) -> Self {
        let mut t = Self::new(session_dir);
        t.mode = snapshot.mode;
        // Transient pursuit does not survive restart — sticky mode does.
        // Mid-flight Pursuing/Aborting become Idle and re-gate writes.
        // Terminal pursuits keep their unlock: PartialReport / Waived always
        // unlock (abort path), and Idle restores the snapshot flag so a
        // completed full-team synthesis still allows execute after resume.
        t.pursuit = match snapshot.pursuit {
            PursuitState::Pursuing | PursuitState::Aborting => PursuitState::Idle,
            other => other,
        };
        t.solo_waiver = snapshot.solo_waiver;
        t.heavy_unlocked = snapshot.heavy_unlocked || heavy_auto_confirm_from_env();
        t.synthesis_complete = match snapshot.pursuit {
            PursuitState::PartialReport | PursuitState::Waived => true,
            PursuitState::Pursuing | PursuitState::Aborting => false,
            PursuitState::Idle => snapshot.synthesis_complete,
        };
        // Replace budgets are per team-run; resume starts a clean wave budget.
        t.replace_waves_used = 0;
        t.ledger = snapshot.ledger;
        t.resume_elevated_notice = snapshot.mode.is_elevated();
        t
    }

    pub fn snapshot(&self) -> EffortModeSnapshot {
        EffortModeSnapshot {
            mode: self.mode,
            pursuit: self.pursuit,
            solo_waiver: self.solo_waiver,
            synthesis_complete: self.synthesis_complete,
            replace_waves_used: self.replace_waves_used,
            ledger: self.ledger.clone(),
            heavy_unlocked: self.heavy_unlocked,
        }
    }

    pub fn session_dir(&self) -> &std::path::Path {
        &self.session_dir
    }

    pub fn mode(&self) -> EffortMode {
        self.mode
    }

    pub fn pursuit(&self) -> PursuitState {
        self.pursuit
    }

    pub fn solo_waiver(&self) -> bool {
        self.solo_waiver
    }

    pub fn force_team(&self) -> bool {
        self.force_team
    }

    pub fn heavy_unlocked(&self) -> bool {
        self.heavy_unlocked
    }

    pub fn last_waiver(&self) -> WaiverReason {
        self.last_waiver
    }

    pub fn take_resume_elevated_notice(&mut self) -> bool {
        let v = self.resume_elevated_notice;
        self.resume_elevated_notice = false;
        v
    }

    pub fn resume_elevated_notice(&self) -> bool {
        self.resume_elevated_notice
    }

    pub fn synthesis_complete(&self) -> bool {
        self.synthesis_complete
    }

    pub fn ledger(&self) -> &[SpecialistLedgerRow] {
        &self.ledger
    }

    pub fn caps(&self) -> EffortCaps {
        self.caps
    }

    /// Sticky mode set. Clears in-flight team when entering Normal.
    pub fn set_mode(&mut self, mode: EffortMode, solo: bool) {
        if mode == EffortMode::Normal {
            self.clear_to_normal();
            return;
        }
        self.mode = mode;
        self.solo_waiver = solo;
        self.force_team = false;
        self.last_waiver = if solo {
            WaiverReason::Solo
        } else {
            WaiverReason::None
        };
        // Entering Heavy does not auto-unlock first fan-out (unless env).
        if mode == EffortMode::Heavy && heavy_auto_confirm_from_env() {
            self.heavy_unlocked = true;
        }
        if self.pursuit == PursuitState::Pursuing || self.pursuit == PursuitState::Aborting {
            // Mode change mid-flight does not auto-start a new team.
        }
    }

    /// `/normal`: clear mode and cancel team.
    pub fn clear_to_normal(&mut self) {
        self.finish_team_timer();
        self.cancel_team();
        self.mode = EffortMode::Normal;
        self.solo_waiver = false;
        self.force_team = false;
        self.last_waiver = WaiverReason::None;
        self.resume_elevated_notice = false;
        self.pursuit = PursuitState::Idle;
        self.synthesis_complete = false;
        self.replace_waves_used = 0;
        self.continue_wave_pending = false;
        self.pending_preflight = false;
        self.ledger.clear();
    }

    pub fn set_solo_waiver(&mut self, solo: bool) {
        self.solo_waiver = solo;
        if solo {
            self.last_waiver = WaiverReason::Solo;
        }
    }

    /// Unlock first Heavy multi-agent spawn for this session (confirm gate).
    pub fn unlock_heavy(&mut self) {
        self.heavy_unlocked = true;
        if self.last_waiver == WaiverReason::NeedsHeavyConfirm {
            self.last_waiver = WaiverReason::None;
        }
    }

    /// Per-turn force full team (escape false trivial).
    pub fn set_force_team(&mut self, force: bool) {
        self.force_team = force;
        if force {
            self.solo_waiver = false;
            self.last_waiver = WaiverReason::None;
        }
    }

    /// Begin a fixed-team run for elevated modes (non-trivial, not solo).
    ///
    /// Selects reasoning brains (Expert: random 4; Heavy: all 16) and fills the
    /// ledger with brain ids as roles so spawn/join reuse the same selection.
    pub fn begin_team_run(&mut self) -> Result<(), EffortGateError> {
        if !self.mode.is_elevated() {
            return Err(EffortGateError::IdleInNormal);
        }
        if self.solo_waiver {
            self.pursuit = PursuitState::Waived;
            return Ok(());
        }
        let n = self
            .mode
            .team_size_default()
            .ok_or(EffortGateError::IdleInNormal)?;
        let cfg = crate::session::effort_brains::load_effort_brain_config()
            .map_err(|e| EffortGateError::BrainConfig(e.to_string()))?;
        let brain_ids = crate::session::effort_brains::select_brain_ids(&cfg, self.mode)
            .map_err(|e| EffortGateError::BrainConfig(e.to_string()))?;
        if brain_ids.len() != n {
            return Err(EffortGateError::BrainConfig(format!(
                "selected {} brains but mode expects N={n}",
                brain_ids.len()
            )));
        }
        self.pursuit = PursuitState::Pursuing;
        self.synthesis_complete = false;
        self.replace_waves_used = 0;
        self.continue_wave_pending = false;
        self.team_started_at = Some(Instant::now());
        self.pending_preflight = true;
        self.pending_run_footer = false;
        self.ledger.clear();
        for (i, brain_id) in brain_ids.into_iter().enumerate() {
            let spec = cfg.get(brain_id.as_str()).ok_or_else(|| {
                EffortGateError::BrainConfig(format!("missing brain `{}`", brain_id.as_str()))
            })?;
            let mut row = SpecialistLedgerRow::new(i, brain_id.as_str());
            row.is_contrarian = spec.contrarian_class;
            self.ledger.push(row);
        }
        Ok(())
    }

    /// Stop the team wall clock (abort / synthesis / clear) and stash elapsed.
    fn finish_team_timer(&mut self) {
        if let Some(start) = self.team_started_at.take() {
            self.last_run_elapsed_secs = Some(start.elapsed().as_secs());
            self.pending_run_footer = true;
        }
    }

    /// Live elapsed seconds for the in-flight team run, if any.
    pub fn team_elapsed_secs(&self) -> Option<u64> {
        self.team_started_at.map(|t| t.elapsed().as_secs())
    }

    pub fn last_run_elapsed_secs(&self) -> Option<u64> {
        self.last_run_elapsed_secs
    }

    /// Take pending pre-flight operator text (once per begin_team_run).
    pub fn take_preflight_message(&mut self) -> Option<String> {
        if !self.pending_preflight {
            return None;
        }
        self.pending_preflight = false;
        format_effort_preflight(self.mode)
    }

    /// Take pending post-run footer (once after timer stop).
    pub fn take_run_footer_message(&mut self) -> Option<String> {
        if !self.pending_run_footer {
            return None;
        }
        self.pending_run_footer = false;
        let n = self.target_n().unwrap_or(0);
        let s = self.successful_count();
        let partial = !self.synthesis_complete
            || self.pursuit == PursuitState::PartialReport
            || (n > 0 && s < n);
        Some(format_effort_run_footer(
            self.mode,
            s,
            n,
            self.last_run_elapsed_secs,
            partial,
        ))
    }

    pub fn record_outcome(
        &mut self,
        slot: usize,
        status: SpecialistStatus,
        task_id: Option<String>,
    ) -> Result<(), EffortGateError> {
        let row = self
            .ledger
            .iter_mut()
            .find(|r| r.slot == slot)
            .ok_or(EffortGateError::UnknownSlot(slot))?;
        row.status = status;
        row.task_id = task_id;
        row.counts_toward_n = matches!(status, SpecialistStatus::Success) && !row.outside_n;
        Ok(())
    }

    pub fn successful_count(&self) -> usize {
        self.ledger
            .iter()
            .filter(|r| r.counts_toward_n && !r.outside_n)
            .count()
    }

    pub fn successful_contrarian_count(&self) -> usize {
        self.ledger
            .iter()
            .filter(|r| r.counts_toward_n && r.is_contrarian && !r.outside_n)
            .count()
    }

    pub fn target_n(&self) -> Option<usize> {
        self.mode.team_size_default()
    }

    /// Whether a full-team Expert/Heavy completion may be claimed.
    pub fn can_claim_full_team(&self) -> Result<(), EffortGateError> {
        if self.pursuit == PursuitState::Waived {
            return Err(EffortGateError::WaivedNotFullTeam);
        }
        if self.pursuit == PursuitState::PartialReport {
            return Err(EffortGateError::PartialNotFullTeam {
                s: self.successful_count(),
                n: self.target_n().unwrap_or(0),
            });
        }
        if self.pursuit == PursuitState::Pursuing || self.pursuit == PursuitState::Idle {
            // Idle with elevated mode and empty ledger → no claim.
            if self.ledger.is_empty() {
                return Err(EffortGateError::NoTeamRun);
            }
        }
        let n = self.target_n().ok_or(EffortGateError::IdleInNormal)?;
        let s = self.successful_count();
        if s < n {
            return Err(EffortGateError::UnderCount {
                s,
                n,
                pursuing: self.pursuit == PursuitState::Pursuing,
            });
        }
        if self.mode.requires_contrarian() && self.successful_contrarian_count() < 1 {
            return Err(EffortGateError::MissingContrarian { s, n });
        }
        Ok(())
    }

    /// Hard-stop when replace budgets exhausted and still short while Pursuing.
    ///
    /// When `continue_wave_pending` is set, the user already bought one wave
    /// that has not yet been launched — still hard-stopped until that wave
    /// runs and re-checks.
    pub fn is_hard_stop(&self) -> bool {
        if self.pursuit != PursuitState::Pursuing {
            return false;
        }
        if self.continue_wave_pending {
            return false;
        }
        let Some(n) = self.target_n() else {
            return false;
        };
        if self.successful_count() >= n
            && (!self.mode.requires_contrarian() || self.successful_contrarian_count() >= 1)
        {
            return false;
        }
        self.replace_waves_used >= self.caps.max_replace_waves
    }

    /// User chose `continue` after hard-stop: exactly **one** additional
    /// replace wave (tech-spec §4.4). After that wave joins, if still short
    /// the runtime hard-stops **again** and the user may choose `continue`
    /// once more — each invocation buys one wave only, not unlimited free
    /// waves in a single grant.
    pub fn hard_stop_continue(&mut self) -> Result<(), EffortGateError> {
        if !self.is_hard_stop() {
            return Err(EffortGateError::NotHardStopped);
        }
        if self.continue_wave_pending {
            return Err(EffortGateError::NoContinueWave);
        }
        self.continue_wave_pending = true;
        Ok(())
    }

    /// Consume the pending continue wave (called when the extra wave is launched).
    pub fn begin_continue_wave(&mut self) -> Result<(), EffortGateError> {
        if !self.continue_wave_pending {
            return Err(EffortGateError::NoContinueWave);
        }
        self.continue_wave_pending = false;
        // Keep `replace_waves_used` at/above the hard-stop cap so that when
        // this wave joins still short, `is_hard_stop` is true again and the
        // user may grant another single continue.
        if self.replace_waves_used < self.caps.max_replace_waves {
            self.replace_waves_used = self.caps.max_replace_waves;
        }
        Ok(())
    }

    pub fn mark_replace_wave(&mut self) {
        self.replace_waves_used = self.replace_waves_used.saturating_add(1);
    }

    /// Replace one failed slot (one-for-one); respects per-slot caps.
    pub fn replace_slot(&mut self, slot: usize) -> Result<(), EffortGateError> {
        let row = self
            .ledger
            .iter_mut()
            .find(|r| r.slot == slot)
            .ok_or(EffortGateError::UnknownSlot(slot))?;
        if row.replaces >= self.caps.max_replace_per_slot {
            return Err(EffortGateError::ReplaceCap {
                slot,
                cap: self.caps.max_replace_per_slot,
            });
        }
        if matches!(row.status, SpecialistStatus::Success) {
            return Err(EffortGateError::ReplaceSuccessForbidden(slot));
        }
        row.replaces += 1;
        row.status = SpecialistStatus::Pending;
        row.counts_toward_n = false;
        row.task_id = None;
        Ok(())
    }

    /// Slot indices that are not yet counting toward N and may be replaced
    /// (terminal non-success, under per-slot replace cap).
    pub fn replaceable_slots(&self) -> Vec<usize> {
        self.ledger
            .iter()
            .filter(|r| {
                !r.outside_n
                    && !r.counts_toward_n
                    && !matches!(
                        r.status,
                        SpecialistStatus::Success
                            | SpecialistStatus::Pending
                            | SpecialistStatus::Running
                    )
                    && r.replaces < self.caps.max_replace_per_slot
            })
            .map(|r| r.slot)
            .collect()
    }

    /// Prepare one automatic replace wave: reset replaceable failed slots to
    /// Pending and increment the wave counter.
    ///
    /// Returns `true` when at least one slot is ready for re-spawn (so
    /// [`needs_mandatory_fanout`] becomes true). Returns `false` when already
    /// at hard-stop, still in-flight, already complete, or no slot can be
    /// replaced under caps.
    ///
    /// When [`continue_wave_pending`] is set (user chose hard-stop `continue`),
    /// consumes that grant and runs one extra wave even if the normal wave
    /// budget is exhausted. Continue also re-allows one replace on slots that
    /// already hit the per-slot cap so the extra wave is actually actionable.
    pub fn try_prepare_replace_wave(&mut self) -> bool {
        if self.pursuit != PursuitState::Pursuing || self.synthesis_complete {
            return false;
        }
        if !self.all_fixed_slots_terminal() {
            return false;
        }
        if self.can_claim_full_team().is_ok() {
            return false;
        }

        // Hard-stop continue buys one wave past the normal cap.
        if self.continue_wave_pending {
            if self.begin_continue_wave().is_err() {
                return false;
            }
            // Make the bought wave actionable: free one replace on terminal
            // non-success slots that already sat at the per-slot cap.
            for row in &mut self.ledger {
                if row.outside_n || row.counts_toward_n {
                    continue;
                }
                if matches!(
                    row.status,
                    SpecialistStatus::Success
                        | SpecialistStatus::Pending
                        | SpecialistStatus::Running
                ) {
                    continue;
                }
                if row.replaces >= self.caps.max_replace_per_slot {
                    row.replaces = self.caps.max_replace_per_slot.saturating_sub(1);
                }
            }
        } else if self.replace_waves_used >= self.caps.max_replace_waves {
            return false;
        }

        let slots = self.replaceable_slots();
        if slots.is_empty() {
            // Nothing left to replace under per-slot caps — force hard-stop
            // by exhausting the wave budget so UX and is_hard_stop agree.
            if self.replace_waves_used < self.caps.max_replace_waves {
                self.replace_waves_used = self.caps.max_replace_waves;
            }
            return false;
        }

        let mut any = false;
        for slot in slots {
            if self.replace_slot(slot).is_ok() {
                any = true;
            }
        }
        if any {
            // Continue waves already sit at/above the hard-stop cap; still
            // count the wave so bookkeeping stays monotonic.
            self.mark_replace_wave();
        }
        any
    }

    /// User abort: freeze ledger, no more automatic replaces.
    pub fn abort_team(&mut self) {
        if self.pursuit != PursuitState::Pursuing && self.pursuit != PursuitState::Aborting {
            return;
        }
        self.pursuit = PursuitState::Aborting;
        for row in &mut self.ledger {
            if matches!(
                row.status,
                SpecialistStatus::Pending | SpecialistStatus::Running
            ) {
                row.status = SpecialistStatus::Cancelled;
                row.counts_toward_n = false;
            }
        }
        self.pursuit = PursuitState::PartialReport;
        self.continue_wave_pending = false;
        self.finish_team_timer();
    }

    pub fn cancel_team(&mut self) {
        for row in &mut self.ledger {
            if matches!(
                row.status,
                SpecialistStatus::Pending | SpecialistStatus::Running
            ) {
                row.status = SpecialistStatus::Cancelled;
                row.counts_toward_n = false;
            }
        }
        self.continue_wave_pending = false;
    }

    /// User chose solo after hard-stop or via `--solo`.
    pub fn waive_solo(&mut self) {
        self.solo_waiver = true;
        self.pursuit = PursuitState::Waived;
        self.continue_wave_pending = false;
    }

    pub fn mark_synthesis_complete(&mut self) -> Result<(), EffortGateError> {
        self.can_claim_full_team()?;
        self.synthesis_complete = true;
        self.pursuit = PursuitState::Idle;
        self.finish_team_timer();
        Ok(())
    }

    /// Partial synthesis after abort (S of N allowed).
    pub fn mark_partial_synthesis(&mut self) {
        self.synthesis_complete = true;
        if self.pursuit != PursuitState::Waived {
            self.pursuit = PursuitState::PartialReport;
        }
        self.finish_team_timer();
    }

    /// Execute/write tools allowed only after synthesis for elevated team runs.
    ///
    /// Sticky Expert/Heavy without `--solo` / Waived **blocks writes until
    /// synthesis completes**, even when the ledger is still empty (team not
    /// yet started). That keeps the hard gate live for elevated sessions
    /// rather than only after `begin_team_run`.
    pub fn may_execute_writes(&self, plan_mode_active: bool) -> Result<(), EffortGateError> {
        if plan_mode_active && self.mode.is_elevated() {
            return Err(EffortGateError::PlanBlocksExecute);
        }
        if !self.mode.is_elevated() {
            return Ok(());
        }
        if self.solo_waiver || self.pursuit == PursuitState::Waived {
            // Solo waiver: no fixed team; execute allowed (unless plan blocks).
            return Ok(());
        }
        if !self.synthesis_complete {
            return Err(EffortGateError::ExecuteBeforeSynthesis);
        }
        Ok(())
    }

    /// Session turn-start hook: under sticky Expert/Heavy (non-solo), start a
    /// fixed-team run for non-trivial work when idle (or re-enter after a
    /// finished prior run with synthesis already complete).
    ///
    /// Trivial tasks and solo waiver leave the ledger empty / Waived
    /// (writes stay available under elevated sticky mode).
    ///
    /// **First Heavy multi-agent spawn** in a session requires an explicit
    /// unlock (`--confirm` / "confirm heavy" / env `GROK_HEAVY_AUTO_CONFIRM=1`
    /// / prior unlock). Until then chrome shows NeedsHeavyConfirm and no
    /// team is started.
    ///
    /// **`--force-team`** (or force_team sticky this turn) runs the full team
    /// even when the classifier would waive as trivial.
    ///
    /// Returns whether a new team run was begun.
    pub fn on_session_turn_start(&mut self, task_text: &str) -> Result<bool, EffortGateError> {
        // Apply per-turn flags from the user message (and strip them for classify).
        let flags = parse_effort_turn_flags(task_text);
        if flags.force_team {
            self.force_team = true;
            self.solo_waiver = false;
            self.last_waiver = WaiverReason::None;
        }
        if flags.solo {
            self.solo_waiver = true;
            self.force_team = false;
            self.last_waiver = WaiverReason::Solo;
        }
        if flags.confirm_heavy {
            self.heavy_unlocked = true;
            if self.last_waiver == WaiverReason::NeedsHeavyConfirm {
                self.last_waiver = WaiverReason::None;
            }
        }
        let classify_text = flags.task.as_deref().unwrap_or("");

        if !self.mode.is_elevated() {
            return Ok(false);
        }
        if self.solo_waiver {
            self.pursuit = PursuitState::Waived;
            self.last_waiver = WaiverReason::Solo;
            return Ok(false);
        }
        // Flag-only / unlock-only turns do not open a team (need real task text).
        if classify_text.trim().is_empty() && !is_hard_stop_continue_request(task_text) {
            return Ok(false);
        }
        // Hard-stop continue grant (before trivial short-circuit so "continue"
        // is never treated as a trivial waived turn).
        if is_hard_stop_continue_request(task_text) && self.is_hard_stop() {
            let _ = self.hard_stop_continue();
            return Ok(false);
        }
        // First-Heavy confirm gate (before opening N slots).
        // `--force-team` does **not** bypass unlock (accidental-Heavy protection).
        if self.mode == EffortMode::Heavy && !self.heavy_unlocked {
            self.pursuit = PursuitState::Waived;
            self.last_waiver = WaiverReason::NeedsHeavyConfirm;
            // Writes stay allowed until operator confirms and a real team runs.
            self.synthesis_complete = true;
            return Ok(false);
        }
        if !self.force_team && is_trivial_task(classify_text) {
            // Trivial short-circuit: Waived so write tools stay available.
            self.pursuit = PursuitState::Waived;
            self.last_waiver = WaiverReason::Trivial;
            return Ok(false);
        }
        // Already pursuing with an open (non-terminal) run — keep it.
        if self.pursuit == PursuitState::Pursuing && !self.ledger.is_empty() {
            // Continue grant: leave ledger so fan-out can re-spawn replaced slots.
            if self.continue_wave_pending {
                return Ok(false);
            }
            // In-flight specialists still running/pending — do not reset.
            if !self.all_fixed_slots_terminal() {
                return Ok(false);
            }
            // Dead short team (all terminal, S < N, no pending continue):
            // reopen a fresh team so the next fan-out can unlock writes.
            // Hard-stop UX already offered continue/--solo; a new user turn
            // is treated as a new attempt under sticky elevated mode.
            if !self.synthesis_complete {
                self.begin_team_run()?;
                self.last_waiver = WaiverReason::None;
                return Ok(true);
            }
            return Ok(false);
        }
        // Fresh non-trivial work under elevated mode → open N slots.
        // Clears any prior synthesis_complete so execute re-gates.
        self.begin_team_run()?;
        // Successful Heavy team start counts as unlock for the session.
        if self.mode == EffortMode::Heavy {
            self.heavy_unlocked = true;
        }
        self.last_waiver = WaiverReason::None;
        self.force_team = false; // consume one-shot force
        Ok(true)
    }

    /// Bind a spawned subagent/task_id to the next free Pending ledger slot.
    /// Returns the slot index, or None when not pursuing / no free slot.
    pub fn assign_next_pending_task(&mut self, task_id: impl Into<String>) -> Option<usize> {
        if self.pursuit != PursuitState::Pursuing {
            return None;
        }
        let task_id = task_id.into();
        let row = self.ledger.iter_mut().find(|r| {
            !r.outside_n && r.task_id.is_none() && matches!(r.status, SpecialistStatus::Pending)
        })?;
        row.task_id = Some(task_id);
        row.status = SpecialistStatus::Running;
        Some(row.slot)
    }

    /// Map a finished subagent status string to a ledger status.
    pub fn specialist_status_from_subagent(status: &str) -> SpecialistStatus {
        match status {
            "completed" | "success" | "ok" => SpecialistStatus::Success,
            "cancelled" | "canceled" | "aborted" => SpecialistStatus::Cancelled,
            "timeout" | "timed_out" => SpecialistStatus::Timeout,
            "empty" | "empty_report" => SpecialistStatus::EmptyReport,
            _ => SpecialistStatus::Failed,
        }
    }

    /// Whether every fixed-team slot (non outside_n) is terminal.
    pub fn all_fixed_slots_terminal(&self) -> bool {
        self.ledger.iter().filter(|r| !r.outside_n).all(|r| {
            !matches!(
                r.status,
                SpecialistStatus::Pending | SpecialistStatus::Running
            )
        })
    }

    /// Record a specialist outcome by task_id (production subagent join path).
    pub fn on_session_specialist_finished(
        &mut self,
        task_id: &str,
        status: SpecialistStatus,
    ) -> Result<(), EffortGateError> {
        let slot = self
            .ledger
            .iter()
            .find(|r| r.task_id.as_deref() == Some(task_id))
            .map(|r| r.slot)
            .ok_or(EffortGateError::UnknownSlot(usize::MAX))?;
        self.record_outcome(slot, status, Some(task_id.to_string()))?;
        // Attempt full-team finalize when every slot is terminal.
        let _ = self.try_finalize_synthesis();
        Ok(())
    }

    /// Session user-cancel hook: freeze an in-flight team into PartialReport
    /// and mark partial synthesis so execute tools unlock for residual work.
    /// Returns progress label (`S of N`) when a team was aborted, else None.
    pub fn on_session_user_cancel(&mut self) -> Option<String> {
        if self.pursuit == PursuitState::Pursuing
            || self.ledger.iter().any(|r| {
                matches!(
                    r.status,
                    SpecialistStatus::Pending | SpecialistStatus::Running
                )
            })
        {
            self.abort_team();
            self.mark_partial_synthesis();
            return Some(self.progress_label());
        }
        None
    }

    /// Session specialist-completion hook by slot (faked task handles).
    pub fn on_session_specialist_outcome(
        &mut self,
        slot: usize,
        status: SpecialistStatus,
        task_id: Option<String>,
    ) -> Result<(), EffortGateError> {
        self.record_outcome(slot, status, task_id)?;
        let _ = self.try_finalize_synthesis();
        Ok(())
    }

    /// Session finalize hook: claim full team or return the gate error.
    pub fn on_session_claim_full_team(&self) -> Result<(), EffortGateError> {
        self.can_claim_full_team()
    }

    /// Attempt full-team claim + `mark_synthesis_complete`.
    ///
    /// Returns `Ok(true)` when synthesis is now complete (full team or
    /// already complete). Returns `Ok(false)` when still pursuing / short.
    /// Under-count with all slots terminal stays Pursuing (hard-stop path);
    /// full claim is never granted with S&lt;N.
    pub fn try_finalize_synthesis(&mut self) -> Result<bool, EffortGateError> {
        if self.synthesis_complete {
            return Ok(true);
        }
        if self.pursuit == PursuitState::PartialReport {
            self.mark_partial_synthesis();
            return Ok(true);
        }
        if self.pursuit == PursuitState::Waived {
            return Ok(false);
        }
        match self.can_claim_full_team() {
            Ok(()) => {
                self.mark_synthesis_complete()?;
                Ok(true)
            }
            Err(EffortGateError::UnderCount { .. })
            | Err(EffortGateError::MissingContrarian { .. }) => {
                // Still short: leave synthesis_complete false so execute stays
                // gated until full team, user abort/partial, solo, or /normal.
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Turn-end hook: if every fixed slot is terminal, attempt finalize.
    /// Returns whether synthesis is complete after the attempt.
    pub fn on_session_turn_end(&mut self) -> bool {
        if self.pursuit != PursuitState::Pursuing {
            return self.synthesis_complete;
        }
        if !self.all_fixed_slots_terminal() {
            return self.synthesis_complete;
        }
        match self.try_finalize_synthesis() {
            Ok(done) => done,
            Err(_) => self.synthesis_complete,
        }
    }

    /// Label a post-N implementer (outside fixed team).
    pub fn register_post_n_implementer(
        &mut self,
        role: impl Into<String>,
    ) -> Result<usize, EffortGateError> {
        if !self.synthesis_complete {
            return Err(EffortGateError::ExecuteBeforeSynthesis);
        }
        let slot = self.ledger.len();
        let mut row = SpecialistLedgerRow::new(slot, role);
        row.outside_n = true;
        row.status = SpecialistStatus::Running;
        self.ledger.push(row);
        Ok(slot)
    }

    /// Progress label `S of N` for abort/partial UX.
    pub fn progress_label(&self) -> String {
        let s = self.successful_count();
        let n = self.target_n().unwrap_or(0);
        format!("{s} of {n}")
    }

    /// Soft policy text injected into the leader prompt under Expert/Heavy.
    ///
    /// Fan-out itself is **shell-mandatory** (see `needs_mandatory_fanout` +
    /// SessionActor orchestration). This reminder tells the leader to
    /// synthesize from the team package rather than re-spawn.
    pub fn policy_reminder(&self, plan_mode_active: bool) -> Option<String> {
        if !self.mode.is_elevated() {
            return None;
        }
        let n = self.mode.team_size_default().unwrap_or(0);
        let contrarian = if self.mode.requires_contrarian() {
            "Require ≥1 successful contrarian specialist among the N."
        } else {
            "Prefer at least one contrarian angle when useful."
        };
        let plan = if plan_mode_active {
            " Plan mode is active: the fixed team and leader must stay non-writing until plan exit allows execute."
        } else {
            ""
        };
        let solo = if self.solo_waiver {
            " Solo waiver is set for this turn: do not run the full fixed team."
        } else {
            ""
        };
        // Body only — callers wrap with `<system-reminder>` (or the
        // session reminder tag) so injection paths never double-nest tags.
        Some(format!(
            "Effort mode: {mode}. For non-trivial work the shell runs a **mandatory** fixed \
             analytic team of N={n} specialists (join-all) before you synthesize. \
             Relative cost class (approx, not $): {cost}. \
             {contrarian} Do not claim full-team completion with fewer than N successful \
             specialist reports. Do not re-spawn the fixed team — use the team report package \
             when present. Execute/write only after synthesis; any implementer runs outside N.\
             {plan}{solo}\n\
             Live chrome shows S of N + elapsed. Abort: /normal (partial S of N — never full-team success).\n\
             Effort mode never enables always-approve/yolo.",
            mode = self.mode.as_str(),
            n = n,
            cost = self.mode.cost_class().label(),
            contrarian = contrarian,
            plan = plan,
            solo = solo,
        ))
    }

    /// True when sticky elevated mode has open fixed-team slots that still
    /// need shell-owned spawn (pending, unbound `task_id`).
    pub fn needs_mandatory_fanout(&self) -> bool {
        if !self.mode.is_elevated() || self.solo_waiver {
            return false;
        }
        if self.pursuit != PursuitState::Pursuing {
            return false;
        }
        self.ledger.iter().any(|r| {
            !r.outside_n && r.task_id.is_none() && matches!(r.status, SpecialistStatus::Pending)
        })
    }

    /// Build briefs only for unbound Pending fixed-team slots (replace waves
    /// and partial re-spawns). Full-team planning uses the same path so Expert
    /// brain selection stays fixed on the ledger from [`Self::begin_team_run`].
    pub fn pending_slot_briefs(&self, task_text: &str) -> Vec<SpecialistBrief> {
        if self.pursuit != PursuitState::Pursuing {
            return Vec::new();
        }
        let n = self.target_n().unwrap_or(self.ledger.len());
        let cfg = match crate::session::effort_brains::load_effort_brain_config() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "effort brains: load failed for pending briefs");
                return Vec::new();
            }
        };
        self.ledger
            .iter()
            .filter(|r| {
                !r.outside_n && r.task_id.is_none() && matches!(r.status, SpecialistStatus::Pending)
            })
            .filter_map(|r| {
                let spec = cfg.get(&r.role).or_else(|| {
                    tracing::warn!(role = %r.role, "effort brains: unknown ledger role");
                    None
                })?;
                Some(SpecialistBrief {
                    slot: r.slot,
                    role: r.role.clone(),
                    is_contrarian: r.is_contrarian || spec.contrarian_class,
                    description: crate::session::effort_brains::brain_description(
                        self.mode, r.slot, n, spec,
                    ),
                    prompt: crate::session::effort_brains::render_specialist_prompt(
                        self.mode, r.slot, n, task_text, spec, true,
                    ),
                    brain_id: spec.id.as_str().to_string(),
                    model_override: if cfg.allow_model_overrides {
                        spec.model.clone()
                    } else {
                        None
                    },
                })
            })
            .collect()
    }

    /// Whether a specialist report body is non-empty enough to count toward N
    /// (tech-spec §4.1: empty "ok" / whitespace-only must not count).
    pub fn report_body_counts_toward_n(body: &str) -> bool {
        let t = body.trim();
        if t.is_empty() {
            return false;
        }
        // Thin placeholders that are not task-relevant reports.
        let lower = t.to_ascii_lowercase();
        !matches!(
            lower.as_str(),
            "ok" | "okay" | "done" | "success" | "yes" | "no" | "n/a" | "none"
        )
    }

    /// Map a join result (success flag + body + cancelled) to ledger status.
    pub fn specialist_status_from_join(
        success: bool,
        cancelled: bool,
        body: &str,
    ) -> SpecialistStatus {
        if cancelled {
            return SpecialistStatus::Cancelled;
        }
        if !success {
            return SpecialistStatus::Failed;
        }
        if Self::report_body_counts_toward_n(body) {
            SpecialistStatus::Success
        } else {
            SpecialistStatus::EmptyReport
        }
    }

    /// Snapshot fields for TUI chrome (mode pill + S of N + Partial/Waived).
    pub fn chrome_state(&self) -> EffortChromeState {
        EffortChromeState {
            mode: self.mode,
            pursuit: self.pursuit,
            successful: self.successful_count(),
            target_n: self.target_n(),
            solo_waiver: self.solo_waiver,
            brain_hint: self.chrome_brain_hint(),
            waiver_reason: self.last_waiver,
            resume_notice: self.resume_elevated_notice,
            elapsed_secs: self.team_elapsed_secs(),
        }
    }

    /// Brain id for chrome: prefer in-flight Running, else next Pending, else last Success.
    fn chrome_brain_hint(&self) -> Option<String> {
        if !self.mode.is_elevated() || self.solo_waiver {
            return None;
        }
        if let Some(r) = self
            .ledger
            .iter()
            .find(|r| !r.outside_n && matches!(r.status, SpecialistStatus::Running))
        {
            return Some(r.role.clone());
        }
        if let Some(r) = self.ledger.iter().find(|r| {
            !r.outside_n && matches!(r.status, SpecialistStatus::Pending) && r.task_id.is_none()
        }) {
            return Some(r.role.clone());
        }
        self.ledger
            .iter()
            .rev()
            .find(|r| !r.outside_n && matches!(r.status, SpecialistStatus::Success))
            .map(|r| r.role.clone())
    }
}

// ── TUI chrome labels (pure) ───────────────────────────────────────────────

/// Effort fields the TUI needs for the status-bar chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffortChromeState {
    pub mode: EffortMode,
    pub pursuit: PursuitState,
    pub successful: usize,
    pub target_n: Option<usize>,
    pub solo_waiver: bool,
    /// Active / next / last brain id for elevated team runs.
    pub brain_hint: Option<String>,
    pub waiver_reason: WaiverReason,
    pub resume_notice: bool,
    /// Live wall-clock seconds while team is in flight (issue #8).
    pub elapsed_secs: Option<u64>,
}

impl EffortChromeState {
    /// Status-bar label from shell effort state, or `None` when Normal
    /// (elevated chrome must disappear).
    ///
    /// Examples: `Expert`, `Expert 2 of 4 · bayesian_update`, `Heavy Partial 3 of 16`,
    /// `Expert Solo`, `Heavy Trivial`, `Heavy · confirm`.
    pub fn status_label(self) -> Option<String> {
        format_effort_chrome_label(&self)
    }
}

/// Format the durable TUI effort chrome label from shipped tracker fields.
pub fn format_effort_chrome_label(state: &EffortChromeState) -> Option<String> {
    if !state.mode.is_elevated() {
        return None;
    }
    let name = match state.mode {
        EffortMode::Expert => "Expert",
        EffortMode::Heavy => "Heavy",
        EffortMode::Normal => return None,
    };
    let resume = if state.resume_notice {
        " · resumed"
    } else {
        ""
    };

    // Explicit waiver labels — never fake S of N progress.
    if state.waiver_reason == WaiverReason::NeedsHeavyConfirm {
        return Some(format!("{name} · confirm first team{resume}"));
    }
    if state.solo_waiver || state.waiver_reason == WaiverReason::Solo {
        return Some(format!("{name} Solo{resume}"));
    }
    if state.waiver_reason == WaiverReason::Trivial || state.pursuit == PursuitState::Waived {
        if state.waiver_reason == WaiverReason::Trivial {
            return Some(format!("{name} Trivial{resume}"));
        }
        return Some(format!("{name} Waived{resume}"));
    }
    let n = state.target_n.unwrap_or(0);
    let s = state.successful;
    let base = match state.pursuit {
        PursuitState::PartialReport => {
            if n > 0 {
                format!("{name} Partial {s} of {n}")
            } else {
                format!("{name} Partial")
            }
        }
        PursuitState::Pursuing | PursuitState::Aborting => {
            if n > 0 {
                format!("{name} {s} of {n}")
            } else {
                name.to_string()
            }
        }
        PursuitState::Idle | PursuitState::Waived => name.to_string(),
    };
    let base = format!("{base}{resume}");
    let base = match state.elapsed_secs {
        Some(secs)
            if matches!(
                state.pursuit,
                PursuitState::Pursuing | PursuitState::Aborting | PursuitState::PartialReport
            ) =>
        {
            format!("{base} · {}", format_elapsed_compact(secs))
        }
        _ => base,
    };
    match &state.brain_hint {
        Some(b)
            if !b.is_empty()
                && matches!(
                    state.pursuit,
                    PursuitState::Pursuing | PursuitState::Aborting | PursuitState::PartialReport
                ) =>
        {
            Some(format!("{base} · {b}"))
        }
        _ => Some(base),
    }
}

/// Wire payload fields for `SessionUpdate::EffortModeUpdated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffortChromeWire {
    pub mode: String,
    pub pursuit: String,
    pub label: Option<String>,
    pub successful: usize,
    pub target_n: Option<usize>,
    pub solo_waiver: bool,
    pub brain_hint: Option<String>,
}

impl EffortChromeState {
    pub fn to_wire(&self) -> EffortChromeWire {
        EffortChromeWire {
            mode: self.mode.as_str().to_string(),
            pursuit: match self.pursuit {
                PursuitState::Idle => "idle",
                PursuitState::Pursuing => "pursuing",
                PursuitState::Aborting => "aborting",
                PursuitState::PartialReport => "partial_report",
                PursuitState::Waived => "waived",
            }
            .to_string(),
            label: format_effort_chrome_label(self),
            successful: self.successful,
            target_n: self.target_n,
            solo_waiver: self.solo_waiver,
            brain_hint: self.brain_hint.clone(),
        }
    }
}

// ── Mandatory team briefs (pure) ───────────────────────────────────────────

/// One specialist brief for shell-owned Expert/Heavy fan-out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecialistBrief {
    pub slot: usize,
    pub role: String,
    pub is_contrarian: bool,
    pub description: String,
    pub prompt: String,
    /// Reasoning-brain id (same as `role` when brains are wired).
    pub brain_id: String,
    /// Optional model override from brain catalog (only if allow_model_overrides).
    pub model_override: Option<String>,
}

/// Build N specialist briefs for a mandatory team run under `mode`.
///
/// Loads the effort-brain catalog, selects ids (Expert random 4 / Heavy all 16),
/// and renders protocol prompts. Prefer [`EffortModeTracker::pending_slot_briefs`]
/// in production after [`EffortModeTracker::begin_team_run`] so Expert selection
/// is fixed on the ledger.
///
/// Returns empty when mode is Normal, or when brain config fails (logs error).
pub fn build_specialist_briefs(mode: EffortMode, task_text: &str) -> Vec<SpecialistBrief> {
    let Some(n) = mode.team_size_default() else {
        return Vec::new();
    };
    let cfg = match crate::session::effort_brains::load_effort_brain_config() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "effort brains: cannot build specialist briefs");
            return Vec::new();
        }
    };
    let brain_ids = match crate::session::effort_brains::select_brain_ids(&cfg, mode) {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(error = %e, "effort brains: selection failed");
            return Vec::new();
        }
    };
    if brain_ids.len() != n {
        tracing::error!(
            selected = brain_ids.len(),
            n,
            "effort brains: selection size mismatch"
        );
        return Vec::new();
    }
    brain_ids
        .into_iter()
        .enumerate()
        .filter_map(|(i, brain_id)| {
            let spec = cfg.get(brain_id.as_str())?;
            Some(SpecialistBrief {
                slot: i,
                role: brain_id.as_str().to_string(),
                is_contrarian: spec.contrarian_class,
                description: crate::session::effort_brains::brain_description(mode, i, n, spec),
                prompt: crate::session::effort_brains::render_specialist_prompt(
                    mode, i, n, task_text, spec, false,
                ),
                brain_id: brain_id.as_str().to_string(),
                model_override: if cfg.allow_model_overrides {
                    spec.model.clone()
                } else {
                    None
                },
            })
        })
        .collect()
}

/// Format joined specialist reports for injection into the leader turn.
///
/// Requires a **disagreement-oriented** synthesis (method deltas), not a bland average.
pub fn format_team_report_package(
    mode: EffortMode,
    progress: &str,
    reports: &[(SpecialistBrief, String)],
) -> String {
    let mode_name = mode.as_str();
    let mut out = format!(
        "Effort mode: {mode_name} — **mandatory team complete** ({progress}).\n\
         Specialists used **distinct reasoning brains** (method protocols). \
         Synthesize by comparing methods — do **not** paper over conflicts. \
         Do not re-run the fixed team.\n\
         \n\
         After reading all reports, structure your answer with these sections:\n\
         ## Consensus (multi-brain)\n\
         ## Conflicts (brain A vs brain B — do not paper over)\n\
         ## Unique contributions (single-brain claims)\n\
         ## Residuals / unknowns\n\
         ## Decision (cite brain ids)\n"
    );
    for (brief, body) in reports {
        let tag = if brief.is_contrarian {
            "contrarian-brain"
        } else {
            "brain"
        };
        let bid = if brief.brain_id.is_empty() {
            brief.role.as_str()
        } else {
            brief.brain_id.as_str()
        };
        out.push_str(&format!(
            "\n--- {tag} slot {} ({bid}) ---\n{}\n",
            brief.slot, body
        ));
    }
    out
}

// ── Errors ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffortGateError {
    IdleInNormal,
    NoTeamRun,
    UnderCount {
        s: usize,
        n: usize,
        pursuing: bool,
    },
    MissingContrarian {
        s: usize,
        n: usize,
    },
    PartialNotFullTeam {
        s: usize,
        n: usize,
    },
    WaivedNotFullTeam,
    ExecuteBeforeSynthesis,
    PlanBlocksExecute,
    UnknownSlot(usize),
    ReplaceCap {
        slot: usize,
        cap: u32,
    },
    ReplaceSuccessForbidden(usize),
    NotHardStopped,
    NoContinueWave,
    /// Effort-brain catalog/selection failed (fail loud; no angle fallback).
    BrainConfig(String),
}

impl std::fmt::Display for EffortGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IdleInNormal => write!(f, "effort multi-agent runtime idle in Normal"),
            Self::NoTeamRun => write!(f, "no team run recorded"),
            Self::UnderCount { s, n, .. } => {
                write!(f, "cannot claim full team: successful_count={s} of {n}")
            }
            Self::MissingContrarian { s, n } => {
                write!(f, "Heavy requires contrarian: successes={s} of {n}")
            }
            Self::PartialNotFullTeam { s, n } => {
                write!(f, "partial report: successful_count={s} of {n}")
            }
            Self::WaivedNotFullTeam => write!(f, "solo/waived turn is not a full team"),
            Self::ExecuteBeforeSynthesis => {
                write!(f, "execute/write blocked until after team synthesis")
            }
            Self::PlanBlocksExecute => {
                write!(
                    f,
                    "Plan mode + Effort: non-writing until plan allows execute"
                )
            }
            Self::UnknownSlot(s) => write!(f, "unknown specialist slot {s}"),
            Self::ReplaceCap { slot, cap } => {
                write!(f, "replace cap {cap} exhausted for slot {slot}")
            }
            Self::ReplaceSuccessForbidden(s) => {
                write!(f, "cannot replace successful slot {s}")
            }
            Self::NotHardStopped => write!(f, "not in hard-stop state"),
            Self::NoContinueWave => write!(f, "no pending continue wave"),
            Self::BrainConfig(detail) => write!(f, "effort brain config error: {detail}"),
        }
    }
}

impl std::error::Error for EffortGateError {}

// ── Arg parse helpers (slash resolve) ──────────────────────────────────────

/// Strip leading effort turn flags from args; return structured flags + task.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffortTurnFlags {
    pub solo: bool,
    pub force_team: bool,
    pub confirm_heavy: bool,
    pub task: Option<String>,
}

/// Parse `/expert` `/heavy` args and free-form turn text for effort flags.
///
/// Recognized leading tokens (order-independent among flags):
/// `--solo`, `--force-team`, `--confirm` / `--confirm-heavy`.
pub fn parse_effort_turn_flags(args: &str) -> EffortTurnFlags {
    let mut flags = EffortTurnFlags::default();
    let mut rest = Vec::new();
    for tok in args.split_whitespace() {
        match tok {
            "--solo" => flags.solo = true,
            "--force-team" | "--force_team" => flags.force_team = true,
            "--confirm" | "--confirm-heavy" | "--confirm_heavy" => flags.confirm_heavy = true,
            other => rest.push(other),
        }
    }
    let joined = rest.join(" ");
    let lower = joined.trim().to_ascii_lowercase();
    // Natural-language confirm-only messages unlock without starting a team.
    if matches!(
        lower.as_str(),
        "confirm heavy" | "confirm" | "yes heavy" | "unlock heavy"
    ) {
        flags.confirm_heavy = true;
        flags.task = None;
    } else {
        flags.task = if joined.trim().is_empty() {
            None
        } else {
            Some(joined)
        };
    }
    // Conflicting flags: solo wins over force_team.
    if flags.solo {
        flags.force_team = false;
    }
    flags
}

/// Re-encode flags + task for turn inject so `on_session_turn_start` sees them.
pub fn encode_effort_turn_text(
    task: &str,
    solo: bool,
    force_team: bool,
    confirm_heavy: bool,
) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if solo {
        parts.push("--solo");
    }
    if force_team {
        parts.push("--force-team");
    }
    if confirm_heavy {
        parts.push("--confirm");
    }
    let t = task.trim();
    if !t.is_empty() {
        parts.push(t);
    }
    parts.join(" ")
}

/// Strip leading `--solo` tokens from args; return (solo, remaining task).
/// Prefer [`parse_effort_turn_flags`] for new call sites.
pub fn parse_solo_and_task(args: &str) -> (bool, Option<String>) {
    let f = parse_effort_turn_flags(args);
    (f.solo, f.task)
}

/// Conservative trivial short-circuit: true only for tiny non-judgment work.
pub fn is_trivial_task(task: &str) -> bool {
    let t = task.trim().to_ascii_lowercase();
    if t.is_empty() {
        return false;
    }
    // Hard research / multi-file keywords → never trivial.
    const HARD: &[&str] = &[
        "architect",
        "architecture",
        "audit",
        "security",
        "refactor",
        "multi-file",
        "multifile",
        "research",
        "design",
        "investigate",
        "implement",
        "migrate",
        "migration",
        "performance",
        "scalability",
        "threat model",
        "code review",
    ];
    if HARD.iter().any(|k| t.contains(k)) {
        return false;
    }
    // Very short typo/rename / acknowledgment style.
    t.len() <= 48
        && (t.starts_with("fix typo")
            || t.starts_with("rename ")
            || t.starts_with("typo")
            || t == "ok"
            || t == "thanks"
            || t == "thanks!"
            || t == "thx"
            || t == "lgtm"
            || t == "looks good"
            || t == "looks good."
            || t == "sg"
            || t == "ship it")
}

/// True when env opts out of first-Heavy confirm (tests / power users).
pub fn heavy_auto_confirm_from_env() -> bool {
    matches!(
        std::env::var("GROK_HEAVY_AUTO_CONFIRM")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

/// User message that grants one hard-stop replace wave (tech-spec §4.4).
pub fn is_hard_stop_continue_request(task: &str) -> bool {
    let t = task.trim().to_ascii_lowercase();
    matches!(
        t.as_str(),
        "continue"
            | "continue."
            | "continue!"
            | "yes continue"
            | "keep going"
            | "retry"
            | "retry failed"
            | "one more wave"
            | "replace wave"
    )
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        PathBuf::from("/tmp/effort-mode-test-session")
    }

    #[test]
    fn team_sizes_and_contrarian() {
        assert_eq!(EffortMode::Normal.team_size_default(), None);
        assert_eq!(EffortMode::Expert.team_size_default(), Some(4));
        assert_eq!(EffortMode::Heavy.team_size_default(), Some(16));
        assert!(!EffortMode::Expert.requires_contrarian());
        assert!(EffortMode::Heavy.requires_contrarian());
    }

    #[test]
    fn parse_solo_strips_flag() {
        assert_eq!(
            parse_solo_and_task("--solo fix the flaky test"),
            (true, Some("fix the flaky test".into()))
        );
        assert_eq!(parse_solo_and_task(""), (false, None));
        assert_eq!(
            parse_solo_and_task("just a task"),
            (false, Some("just a task".into()))
        );
    }

    #[test]
    fn sticky_mode_and_normal_clear() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        assert_eq!(t.mode(), EffortMode::Expert);
        t.set_mode(EffortMode::Heavy, true);
        assert!(t.solo_waiver());
        t.clear_to_normal();
        assert_eq!(t.mode(), EffortMode::Normal);
        assert!(!t.solo_waiver());
        assert_eq!(t.pursuit(), PursuitState::Idle);
    }

    #[test]
    fn snapshot_round_trip_keeps_mode() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        let snap = t.snapshot();
        let restored = EffortModeTracker::from_snapshot(tmp(), snap);
        assert_eq!(restored.mode(), EffortMode::Heavy);
    }

    #[test]
    fn snapshot_restores_synthesis_after_partial_and_full() {
        // Partial abort unlock must survive resume.
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        t.record_outcome(0, SpecialistStatus::Success, Some("a".into()))
            .unwrap();
        t.abort_team();
        t.mark_partial_synthesis();
        assert!(t.synthesis_complete());
        assert!(t.may_execute_writes(false).is_ok());
        let snap = t.snapshot();
        let restored = EffortModeTracker::from_snapshot(tmp(), snap);
        assert_eq!(restored.pursuit(), PursuitState::PartialReport);
        assert!(restored.synthesis_complete());
        assert!(restored.may_execute_writes(false).is_ok());

        // Full-team Idle + synthesis_complete must survive resume.
        let mut full = EffortModeTracker::new(tmp());
        full.set_mode(EffortMode::Expert, false);
        full.begin_team_run().unwrap();
        for i in 0..4 {
            full.record_outcome(i, SpecialistStatus::Success, Some(format!("t{i}")))
                .unwrap();
        }
        full.mark_synthesis_complete().unwrap();
        let snap = full.snapshot();
        let restored = EffortModeTracker::from_snapshot(tmp(), snap);
        assert_eq!(restored.pursuit(), PursuitState::Idle);
        assert!(restored.synthesis_complete());
        assert!(restored.may_execute_writes(false).is_ok());
    }

    #[test]
    fn short_terminal_team_reopens_on_next_turn() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        for i in 0..4 {
            t.record_outcome(i, SpecialistStatus::Failed, None).unwrap();
        }
        assert!(t.all_fixed_slots_terminal());
        assert!(!t.synthesis_complete());
        assert!(matches!(
            t.may_execute_writes(false),
            Err(EffortGateError::ExecuteBeforeSynthesis)
        ));
        // Next non-trivial turn must not stick: reopen fresh team.
        assert!(
            t.on_session_turn_start("architect multi-file auth migration")
                .unwrap()
        );
        assert_eq!(t.pursuit(), PursuitState::Pursuing);
        assert!(t.needs_mandatory_fanout());
        assert_eq!(t.successful_count(), 0);
        assert!(
            t.ledger()
                .iter()
                .all(|r| r.status == SpecialistStatus::Pending)
        );
    }

    #[test]
    fn replace_wave_prepares_failed_slots() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        t.record_outcome(0, SpecialistStatus::Success, Some("ok".into()))
            .unwrap();
        t.record_outcome(1, SpecialistStatus::Failed, None).unwrap();
        t.record_outcome(2, SpecialistStatus::EmptyReport, None)
            .unwrap();
        t.record_outcome(3, SpecialistStatus::Timeout, None)
            .unwrap();
        assert!(t.try_prepare_replace_wave());
        assert_eq!(t.replaceable_slots().len(), 0); // already Pending
        assert!(t.needs_mandatory_fanout());
        let pending: Vec<_> = t
            .ledger()
            .iter()
            .filter(|r| r.status == SpecialistStatus::Pending)
            .map(|r| r.slot)
            .collect();
        assert_eq!(pending, vec![1, 2, 3]);
        // Success slot untouched.
        assert_eq!(t.ledger()[0].status, SpecialistStatus::Success);
        assert_eq!(t.successful_count(), 1);
    }

    #[test]
    fn empty_join_body_maps_to_empty_report() {
        assert_eq!(
            EffortModeTracker::specialist_status_from_join(true, false, "ok"),
            SpecialistStatus::EmptyReport
        );
        assert_eq!(
            EffortModeTracker::specialist_status_from_join(true, false, "   "),
            SpecialistStatus::EmptyReport
        );
        assert_eq!(
            EffortModeTracker::specialist_status_from_join(
                true,
                false,
                "Findings: auth middleware lacks timeout."
            ),
            SpecialistStatus::Success
        );
        assert_eq!(
            EffortModeTracker::specialist_status_from_join(false, true, "cancelled"),
            SpecialistStatus::Cancelled
        );
        assert!(!EffortModeTracker::report_body_counts_toward_n("ok"));
        assert!(EffortModeTracker::report_body_counts_toward_n(
            "risk: shared .grok tree"
        ));
    }

    #[test]
    fn hard_stop_continue_request_grants_wave() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        for i in 0..4 {
            t.record_outcome(i, SpecialistStatus::Failed, None).unwrap();
        }
        // Exhaust replace budget without free slots left (per-slot cap).
        assert!(t.try_prepare_replace_wave()); // wave 1
        for i in 0..4 {
            t.record_outcome(i, SpecialistStatus::Failed, Some(format!("r{i}")))
                .unwrap();
        }
        // Wave 2: per-slot cap exhausted → forced hard-stop.
        assert!(!t.try_prepare_replace_wave());
        assert!(t.is_hard_stop());
        assert!(!t.on_session_turn_start("continue").unwrap());
        // continue_wave_pending set; is_hard_stop false until wave launches.
        assert!(!t.is_hard_stop());
        assert!(t.try_prepare_replace_wave()); // consumes continue
    }

    #[test]
    fn expert_full_team_requires_4_successes() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        for i in 0..3 {
            t.record_outcome(i, SpecialistStatus::Success, Some(format!("t{i}")))
                .unwrap();
        }
        assert!(matches!(
            t.can_claim_full_team(),
            Err(EffortGateError::UnderCount {
                s: 3,
                n: 4,
                pursuing: true
            })
        ));
        t.record_outcome(3, SpecialistStatus::Success, Some("t3".into()))
            .unwrap();
        assert!(t.can_claim_full_team().is_ok());
        t.mark_synthesis_complete().unwrap();
        assert!(t.synthesis_complete());
    }

    #[test]
    fn timeout_and_empty_do_not_count() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        t.record_outcome(0, SpecialistStatus::Timeout, None)
            .unwrap();
        t.record_outcome(1, SpecialistStatus::EmptyReport, None)
            .unwrap();
        t.record_outcome(2, SpecialistStatus::Failed, None).unwrap();
        t.record_outcome(3, SpecialistStatus::Success, Some("ok".into()))
            .unwrap();
        assert_eq!(t.successful_count(), 1);
        assert!(t.can_claim_full_team().is_err());
    }

    #[test]
    fn heavy_requires_contrarian_among_16() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        t.begin_team_run().unwrap();
        assert_eq!(t.ledger().len(), 16);
        let contrarian_n = t.ledger().iter().filter(|r| r.is_contrarian).count();
        assert!(
            contrarian_n >= 1,
            "Heavy roster needs ≥1 contrarian-class brain"
        );
        // Success on all non-contrarian slots only.
        for row in t.ledger.clone() {
            if !row.is_contrarian {
                t.record_outcome(row.slot, SpecialistStatus::Success, Some("x".into()))
                    .unwrap();
            } else {
                t.record_outcome(row.slot, SpecialistStatus::Failed, None)
                    .unwrap();
            }
        }
        assert_eq!(t.successful_count(), 16 - contrarian_n);
        assert!(matches!(
            t.can_claim_full_team(),
            Err(EffortGateError::UnderCount { .. })
                | Err(EffortGateError::MissingContrarian { .. })
        ));
        // Fix all contrarian slots.
        for row in t.ledger.clone() {
            if row.is_contrarian {
                t.record_outcome(row.slot, SpecialistStatus::Success, Some("c".into()))
                    .unwrap();
            }
        }
        assert_eq!(t.successful_count(), 16);
        assert!(t.can_claim_full_team().is_ok());
    }

    #[test]
    fn heavy_16_success_without_contrarian_flag_fails() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        t.begin_team_run().unwrap();
        // Force all rows non-contrarian success (simulate missing contrarian role).
        for row in &mut t.ledger {
            row.is_contrarian = false;
            row.status = SpecialistStatus::Success;
            row.counts_toward_n = true;
        }
        assert_eq!(t.successful_count(), 16);
        assert!(matches!(
            t.can_claim_full_team(),
            Err(EffortGateError::MissingContrarian { .. })
        ));
    }

    #[test]
    fn abort_yields_partial_s_of_n() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        t.record_outcome(0, SpecialistStatus::Success, Some("a".into()))
            .unwrap();
        t.record_outcome(1, SpecialistStatus::Running, Some("b".into()))
            .unwrap();
        t.abort_team();
        assert_eq!(t.pursuit(), PursuitState::PartialReport);
        assert_eq!(t.successful_count(), 1);
        assert_eq!(t.progress_label(), "1 of 4");
        assert!(matches!(
            t.can_claim_full_team(),
            Err(EffortGateError::PartialNotFullTeam { s: 1, n: 4 })
        ));
        // Sticky mode remains Expert.
        assert_eq!(t.mode(), EffortMode::Expert);
    }

    #[test]
    fn mid_flight_normal_cancels_and_clears() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        t.begin_team_run().unwrap();
        t.record_outcome(0, SpecialistStatus::Running, Some("r".into()))
            .unwrap();
        t.clear_to_normal();
        assert_eq!(t.mode(), EffortMode::Normal);
        assert_eq!(t.pursuit(), PursuitState::Idle);
        assert!(t.ledger().is_empty());
    }

    #[test]
    fn execute_blocked_before_synthesis() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        // Elevated sticky with empty ledger still blocks writes (hard gate live).
        assert!(matches!(
            t.may_execute_writes(false),
            Err(EffortGateError::ExecuteBeforeSynthesis)
        ));
        t.begin_team_run().unwrap();
        for i in 0..4 {
            t.record_outcome(i, SpecialistStatus::Success, Some("x".into()))
                .unwrap();
        }
        assert!(matches!(
            t.may_execute_writes(false),
            Err(EffortGateError::ExecuteBeforeSynthesis)
        ));
        t.mark_synthesis_complete().unwrap();
        assert!(t.may_execute_writes(false).is_ok());
        let slot = t.register_post_n_implementer("implementer").unwrap();
        assert!(t.ledger()[slot].outside_n);
    }

    #[test]
    fn session_hooks_begin_abort_claim() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        assert!(
            t.on_session_turn_start("architect multi-file auth migration")
                .unwrap()
        );
        assert_eq!(t.pursuit(), PursuitState::Pursuing);
        assert_eq!(t.ledger().len(), 4);
        t.on_session_specialist_outcome(0, SpecialistStatus::Success, Some("t0".into()))
            .unwrap();
        assert!(matches!(
            t.on_session_claim_full_team(),
            Err(EffortGateError::UnderCount { s: 1, n: 4, .. })
        ));
        let label = t.on_session_user_cancel().unwrap();
        assert_eq!(label, "1 of 4");
        assert_eq!(t.pursuit(), PursuitState::PartialReport);
    }

    #[test]
    fn plan_plus_effort_blocks_execute() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        t.begin_team_run().unwrap();
        for i in 0..16 {
            t.record_outcome(i, SpecialistStatus::Success, Some("x".into()))
                .unwrap();
        }
        t.mark_synthesis_complete().unwrap();
        assert!(matches!(
            t.may_execute_writes(true),
            Err(EffortGateError::PlanBlocksExecute)
        ));
    }

    #[test]
    fn hard_stop_continue_one_wave_per_grant_then_again() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        t.mark_replace_wave();
        t.mark_replace_wave();
        assert!(t.is_hard_stop());
        // First continue grant.
        t.hard_stop_continue().unwrap();
        assert!(!t.is_hard_stop()); // pending wave not yet launched
        assert!(t.hard_stop_continue().is_err()); // cannot double-grant
        t.begin_continue_wave().unwrap();
        // Wave consumed; still short → hard-stop again; user may continue again.
        assert!(t.is_hard_stop());
        t.hard_stop_continue().unwrap();
        t.begin_continue_wave().unwrap();
        assert!(t.is_hard_stop());
    }

    #[test]
    fn production_finalize_unlocks_execute() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.on_session_turn_start("architect multi-file auth migration")
            .unwrap();
        assert!(matches!(
            t.may_execute_writes(false),
            Err(EffortGateError::ExecuteBeforeSynthesis)
        ));
        for i in 0..4 {
            let tid = format!("sub-{i}");
            assert_eq!(t.assign_next_pending_task(&tid), Some(i));
            t.on_session_specialist_finished(&tid, SpecialistStatus::Success)
                .unwrap();
        }
        assert!(t.synthesis_complete());
        assert!(t.may_execute_writes(false).is_ok());
    }

    #[test]
    fn normal_idle_no_team_hooks() {
        let t = EffortModeTracker::new(tmp());
        assert_eq!(t.mode(), EffortMode::Normal);
        assert_eq!(t.pursuit(), PursuitState::Idle);
        assert!(t.policy_reminder(false).is_none());
        assert!(t.may_execute_writes(false).is_ok());
        assert!(matches!(
            EffortModeTracker::new(tmp()).begin_team_run(),
            Err(EffortGateError::IdleInNormal)
        ));
    }

    #[test]
    fn effort_never_implies_yolo() {
        // Structural: tracker has no yolo/always-approve field; policy text forbids it.
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        let p = t.policy_reminder(false).unwrap();
        assert!(p.contains("never enables always-approve"));
    }

    #[test]
    fn feature_flag_default_on() {
        // Do not mutate env in parallel-sensitive ways beyond isolation — check default when unset.
        let prev = std::env::var(EFFORT_MODE_BUILTINS_ENV).ok();
        unsafe { std::env::remove_var(EFFORT_MODE_BUILTINS_ENV) };
        assert!(effort_mode_builtins_enabled());
        unsafe { std::env::set_var(EFFORT_MODE_BUILTINS_ENV, "0") };
        assert!(!effort_mode_builtins_enabled());
        unsafe { std::env::set_var(EFFORT_MODE_BUILTINS_ENV, "1") };
        assert!(effort_mode_builtins_enabled());
        match prev {
            Some(v) => unsafe { std::env::set_var(EFFORT_MODE_BUILTINS_ENV, v) },
            None => unsafe { std::env::remove_var(EFFORT_MODE_BUILTINS_ENV) },
        }
    }

    #[test]
    fn triviality_helper() {
        assert!(is_trivial_task("fix typo in readme"));
        assert!(!is_trivial_task("architect multi-file migration of auth"));
    }

    #[test]
    fn specialist_briefs_expert_n4_and_heavy_n16() {
        unsafe { std::env::set_var(crate::session::effort_brains::EFFORT_BRAIN_SEED_ENV, "7") };
        let expert = build_specialist_briefs(EffortMode::Expert, "audit auth");
        unsafe { std::env::remove_var(crate::session::effort_brains::EFFORT_BRAIN_SEED_ENV) };
        assert_eq!(expert.len(), 4);
        assert!(expert[0].prompt.contains("audit auth"));
        assert!(expert[0].prompt.contains("Brain protocol"));
        assert!(expert[0].prompt.contains("non-writing"));
        // roles are brain ids, not specialist-i
        assert!(!expert[0].role.starts_with("specialist-"));
        let mut ids: Vec<_> = expert.iter().map(|b| b.brain_id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 4);

        let heavy = build_specialist_briefs(EffortMode::Heavy, "deep audit");
        assert_eq!(heavy.len(), 16);
        assert_eq!(heavy[15].role, "red_team");
        assert!(heavy[15].is_contrarian);
        assert!(heavy[15].prompt.contains("red_team"));
        assert!(heavy.iter().filter(|b| b.is_contrarian).count() >= 1);

        assert!(build_specialist_briefs(EffortMode::Normal, "x").is_empty());
    }

    #[test]
    fn team_report_package_includes_slots() {
        let briefs = build_specialist_briefs(EffortMode::Expert, "task");
        let reports: Vec<_> = briefs
            .into_iter()
            .enumerate()
            .map(|(i, b)| (b, format!("report-{i}")))
            .collect();
        let pkg = format_team_report_package(EffortMode::Expert, "4 of 4", &reports);
        assert!(pkg.contains("mandatory team complete"));
        assert!(pkg.contains("report-0"));
        assert!(pkg.contains("Conflicts"));
        assert!(pkg.contains("Decision (cite brain ids)"));
        assert!(pkg.contains("slot 3"));
    }

    #[test]
    fn begin_team_run_uses_brain_ids_on_ledger() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        t.begin_team_run().unwrap();
        assert_eq!(t.ledger().len(), 16);
        assert_eq!(t.ledger()[0].role, "first_principles");
        assert_eq!(t.ledger()[15].role, "red_team");
        assert!(t.ledger()[15].is_contrarian);
        let pending = t.pending_slot_briefs("task");
        assert_eq!(pending.len(), 16);
        assert!(pending[0].prompt.contains("first_principles"));
    }

    #[test]
    fn needs_mandatory_fanout_while_pending_unbound() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        assert!(!t.needs_mandatory_fanout());
        t.begin_team_run().unwrap();
        assert!(t.needs_mandatory_fanout());
        assert_eq!(t.assign_next_pending_task("s0"), Some(0));
        assert!(t.needs_mandatory_fanout());
        for i in 1..4 {
            assert_eq!(t.assign_next_pending_task(format!("s{i}")), Some(i));
        }
        assert!(!t.needs_mandatory_fanout());
    }

    #[test]
    fn policy_reminder_states_mandatory_shell_team() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        let p = t.policy_reminder(false).unwrap();
        assert!(p.contains("mandatory"));
        assert!(p.contains("N=16"));
        assert!(p.contains("Do not re-spawn"));
    }

    #[test]
    fn chrome_label_expert_heavy_progress_partial_waived_normal_clears() {
        // Normal → no elevated chrome.
        assert_eq!(
            format_effort_chrome_label(&EffortChromeState {
                mode: EffortMode::Normal,
                pursuit: PursuitState::Idle,
                successful: 0,
                target_n: None,
                solo_waiver: false,
                brain_hint: None,
                waiver_reason: WaiverReason::None,
                resume_notice: false,
                elapsed_secs: None,
            }),
            None
        );

        // Sticky Expert idle → mode name only.
        assert_eq!(
            format_effort_chrome_label(&EffortChromeState {
                mode: EffortMode::Expert,
                pursuit: PursuitState::Idle,
                successful: 0,
                target_n: Some(4),
                solo_waiver: false,
                brain_hint: None,
                waiver_reason: WaiverReason::None,
                resume_notice: false,
                elapsed_secs: None,
            }),
            Some("Expert".into())
        );

        // Pursuing with S of N.
        assert_eq!(
            format_effort_chrome_label(&EffortChromeState {
                mode: EffortMode::Expert,
                pursuit: PursuitState::Pursuing,
                successful: 2,
                target_n: Some(4),
                solo_waiver: false,
                brain_hint: None,
                waiver_reason: WaiverReason::None,
                resume_notice: false,
                elapsed_secs: None,
            }),
            Some("Expert 2 of 4".into())
        );
        assert_eq!(
            format_effort_chrome_label(&EffortChromeState {
                mode: EffortMode::Heavy,
                pursuit: PursuitState::Pursuing,
                successful: 12,
                target_n: Some(16),
                solo_waiver: false,
                brain_hint: None,
                waiver_reason: WaiverReason::None,
                resume_notice: false,
                elapsed_secs: None,
            }),
            Some("Heavy 12 of 16".into())
        );

        // Partial / Solo / Trivial / confirm.
        assert_eq!(
            format_effort_chrome_label(&EffortChromeState {
                mode: EffortMode::Heavy,
                pursuit: PursuitState::PartialReport,
                successful: 3,
                target_n: Some(16),
                solo_waiver: false,
                brain_hint: None,
                waiver_reason: WaiverReason::None,
                resume_notice: false,
                elapsed_secs: None,
            }),
            Some("Heavy Partial 3 of 16".into())
        );
        assert_eq!(
            format_effort_chrome_label(&EffortChromeState {
                mode: EffortMode::Expert,
                pursuit: PursuitState::Waived,
                successful: 0,
                target_n: Some(4),
                solo_waiver: false,
                brain_hint: None,
                waiver_reason: WaiverReason::Trivial,
                resume_notice: false,
                elapsed_secs: None,
            }),
            Some("Expert Trivial".into())
        );
        assert_eq!(
            format_effort_chrome_label(&EffortChromeState {
                mode: EffortMode::Expert,
                pursuit: PursuitState::Idle,
                successful: 0,
                target_n: Some(4),
                solo_waiver: true,
                brain_hint: None,
                waiver_reason: WaiverReason::Solo,
                resume_notice: false,
                elapsed_secs: None,
            }),
            Some("Expert Solo".into())
        );
        assert_eq!(
            format_effort_chrome_label(&EffortChromeState {
                mode: EffortMode::Heavy,
                pursuit: PursuitState::Waived,
                successful: 0,
                target_n: Some(16),
                solo_waiver: false,
                brain_hint: None,
                waiver_reason: WaiverReason::NeedsHeavyConfirm,
                resume_notice: true,
                elapsed_secs: None,
            }),
            Some("Heavy · confirm first team · resumed".into())
        );

        // Tracker path: begin team → chrome shows 0 of 4; success → 1 of 4.
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        assert_eq!(t.chrome_state().status_label().as_deref(), Some("Expert"));
        t.begin_team_run().unwrap();
        let label0 = t.chrome_state().status_label().unwrap();
        assert!(label0.starts_with("Expert 0 of 4"), "got {label0}");
        t.record_outcome(0, SpecialistStatus::Success, Some("a".into()))
            .unwrap();
        let label1 = t.chrome_state().status_label().unwrap();
        assert!(label1.starts_with("Expert 1 of 4"), "got {label1}");
        t.clear_to_normal();
        assert_eq!(t.chrome_state().status_label(), None);
    }

    #[test]
    fn triviality_hard_architecture_never_trivial() {
        assert!(is_trivial_task("fix typo in readme"));
        assert!(is_trivial_task("thanks"));
        assert!(!is_trivial_task("architect multi-file migration of auth"));
        assert!(!is_trivial_task("security audit of the sandbox"));
        assert!(!is_trivial_task(
            "performance investigation of the hot path"
        ));
    }

    #[test]
    fn force_team_overrides_trivial_classifier() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        // Without force: trivial short-circuits.
        assert!(!t.on_session_turn_start("fix typo").unwrap());
        assert_eq!(t.last_waiver(), WaiverReason::Trivial);
        assert_eq!(t.pursuit(), PursuitState::Waived);
        // Force team on a trivial-looking message.
        assert!(t.on_session_turn_start("--force-team fix typo").unwrap());
        assert_eq!(t.pursuit(), PursuitState::Pursuing);
        assert_eq!(t.last_waiver(), WaiverReason::None);
        assert_eq!(t.target_n(), Some(4));
    }

    #[test]
    fn first_heavy_team_requires_confirm_unless_unlocked() {
        // Ensure env does not auto-confirm in this process for the test body.
        let prev = std::env::var("GROK_HEAVY_AUTO_CONFIRM").ok();
        unsafe { std::env::remove_var("GROK_HEAVY_AUTO_CONFIRM") };

        let mut t = EffortModeTracker::new(tmp());
        // new() may have read env before remove — force locked state.
        t.set_mode(EffortMode::Heavy, false);
        // If env was set earlier in process, unlock may already be true; force lock.
        if t.heavy_unlocked() {
            // Simulate locked session by constructing via snapshot.
            let mut snap = t.snapshot();
            snap.heavy_unlocked = false;
            t = EffortModeTracker::from_snapshot(tmp(), snap);
            // from_snapshot sets resume notice; clear for this test.
            let _ = t.take_resume_elevated_notice();
        }
        assert!(!t.heavy_unlocked());
        assert!(
            !t.on_session_turn_start("architect multi-file auth migration")
                .unwrap()
        );
        assert_eq!(t.last_waiver(), WaiverReason::NeedsHeavyConfirm);
        assert!(
            t.chrome_state().status_label().unwrap().contains("confirm"),
            "{:?}",
            t.chrome_state().status_label()
        );
        // Confirm unlocks and starts team.
        assert!(
            t.on_session_turn_start("--confirm architect multi-file auth migration")
                .unwrap()
        );
        assert!(t.heavy_unlocked());
        assert_eq!(t.pursuit(), PursuitState::Pursuing);

        match prev {
            Some(v) => unsafe { std::env::set_var("GROK_HEAVY_AUTO_CONFIRM", v) },
            None => unsafe { std::env::remove_var("GROK_HEAVY_AUTO_CONFIRM") },
        }
    }

    #[test]
    fn resume_elevated_notice_on_snapshot_restore() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Heavy, false);
        t.unlock_heavy();
        let snap = t.snapshot();
        let restored = EffortModeTracker::from_snapshot(tmp(), snap);
        assert!(restored.resume_elevated_notice());
        assert!(
            restored
                .chrome_state()
                .status_label()
                .unwrap()
                .contains("resumed")
        );
    }

    #[test]
    fn parse_effort_flags_force_and_confirm() {
        let f = parse_effort_turn_flags("--force-team --confirm audit auth");
        assert!(f.force_team);
        assert!(f.confirm_heavy);
        assert!(!f.solo);
        assert_eq!(f.task.as_deref(), Some("audit auth"));
        let s = parse_effort_turn_flags("--solo --force-team x");
        assert!(s.solo);
        assert!(!s.force_team); // solo wins
    }

    #[test]
    fn preflight_shows_n_and_cost_class_not_normal() {
        assert!(format_effort_preflight(EffortMode::Normal).is_none());
        let e = format_effort_preflight(EffortMode::Expert).unwrap();
        assert!(e.contains("N=4"), "{e}");
        assert!(e.contains("multi-agent"), "{e}");
        assert!(e.contains("≈4×"), "{e}");
        assert!(e.contains("Abort"), "{e}");
        let h = format_effort_preflight(EffortMode::Heavy).unwrap();
        assert!(h.contains("N=16"), "{h}");
        assert!(h.contains("≈16×"), "{h}");
        assert!(h.contains("--confirm"), "{h}");
    }

    #[test]
    fn elapsed_compact_and_run_footer() {
        assert_eq!(format_elapsed_compact(12), "12s");
        assert_eq!(format_elapsed_compact(65), "1m05s");
        assert_eq!(format_elapsed_compact(120), "2m");
        let f = format_effort_run_footer(EffortMode::Expert, 4, 4, Some(125), false);
        assert!(f.contains("Expert"), "{f}");
        assert!(f.contains("4 of 4"), "{f}");
        assert!(f.contains("2m05s"), "{f}");
        assert!(f.contains("approx"), "{f}");
        let p = format_effort_run_footer(EffortMode::Heavy, 3, 16, Some(40), true);
        assert!(p.contains("partial 3 of 16"), "{p}");
    }

    #[test]
    fn begin_team_sets_timer_and_preflight_take_once() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        t.begin_team_run().unwrap();
        assert!(t.team_elapsed_secs().is_some());
        let label = t.chrome_state().status_label().unwrap();
        assert!(label.contains("of 4"), "{label}");
        // elapsed present as · Ns or · 0s
        assert!(label.contains('·'), "{label}");
        let msg = t.take_preflight_message().unwrap();
        assert!(msg.contains("N=4"));
        assert!(t.take_preflight_message().is_none());
        t.mark_partial_synthesis();
        assert!(t.team_elapsed_secs().is_none());
        let foot = t.take_run_footer_message().unwrap();
        assert!(foot.contains("partial") || foot.contains("of"), "{foot}");
        assert!(t.take_run_footer_message().is_none());
    }

    #[test]
    fn soft_budget_env_warns_without_panic() {
        let prev = std::env::var(EFFORT_SOFT_BUDGET_N_ENV).ok();
        unsafe { std::env::set_var(EFFORT_SOFT_BUDGET_N_ENV, "2") };
        let e = format_effort_preflight(EffortMode::Expert).unwrap();
        assert!(e.contains("Soft budget warn"), "{e}");
        unsafe { std::env::set_var(EFFORT_SOFT_BUDGET_N_ENV, "not-a-number") };
        assert_eq!(soft_budget_n_from_env(), None);
        match prev {
            Some(v) => unsafe { std::env::set_var(EFFORT_SOFT_BUDGET_N_ENV, v) },
            None => unsafe { std::env::remove_var(EFFORT_SOFT_BUDGET_N_ENV) },
        }
    }
}
