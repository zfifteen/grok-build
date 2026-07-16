//! Effort mode (Expert / Heavy / Normal) session state machine and hard gates.
//!
//! Parallel to [`super::plan_mode::PlanModeTracker`]: pure FSM, no I/O.
//! SessionActor owns one tracker; slash builtins set sticky mode; hard
//! join/ledger/replace/execute rules live here for unit tests with faked
//! specialist outcomes (no live model required).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

// ── Tracker ────────────────────────────────────────────────────────────────

/// Pure effort-mode session tracker (plan-mode twin).
pub struct EffortModeTracker {
    mode: EffortMode,
    pursuit: PursuitState,
    /// Per-turn solo waiver (`--solo` or explicit user solo request).
    solo_waiver: bool,
    /// True after successful synthesis of the current team run.
    synthesis_complete: bool,
    replace_waves_used: u32,
    /// Extra wave granted by hard-stop `continue` (at most one pending wave).
    continue_wave_pending: bool,
    ledger: Vec<SpecialistLedgerRow>,
    caps: EffortCaps,
    session_dir: PathBuf,
}

impl EffortModeTracker {
    pub fn new(session_dir: PathBuf) -> Self {
        Self {
            mode: EffortMode::Normal,
            pursuit: PursuitState::Idle,
            solo_waiver: false,
            synthesis_complete: false,
            replace_waves_used: 0,
            continue_wave_pending: false,
            ledger: Vec::new(),
            caps: EffortCaps::default(),
            session_dir,
        }
    }

    pub fn from_snapshot(session_dir: PathBuf, snapshot: EffortModeSnapshot) -> Self {
        let mut t = Self::new(session_dir);
        t.mode = snapshot.mode;
        // Transient pursuit does not survive restart — sticky mode does.
        t.pursuit = match snapshot.pursuit {
            PursuitState::Pursuing | PursuitState::Aborting => PursuitState::Idle,
            other => other,
        };
        t.solo_waiver = snapshot.solo_waiver;
        t.synthesis_complete = false;
        t.replace_waves_used = 0;
        t.ledger = snapshot.ledger;
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
        if self.pursuit == PursuitState::Pursuing || self.pursuit == PursuitState::Aborting {
            // Mode change mid-flight does not auto-start a new team.
        }
    }

    /// `/normal`: clear mode and cancel team.
    pub fn clear_to_normal(&mut self) {
        self.cancel_team();
        self.mode = EffortMode::Normal;
        self.solo_waiver = false;
        self.pursuit = PursuitState::Idle;
        self.synthesis_complete = false;
        self.replace_waves_used = 0;
        self.continue_wave_pending = false;
        self.ledger.clear();
    }

    pub fn set_solo_waiver(&mut self, solo: bool) {
        self.solo_waiver = solo;
    }

    /// Begin a fixed-team run for elevated modes (non-trivial, not solo).
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
        self.pursuit = PursuitState::Pursuing;
        self.synthesis_complete = false;
        self.replace_waves_used = 0;
        self.continue_wave_pending = false;
        self.ledger.clear();
        for i in 0..n {
            let role = if self.mode.requires_contrarian() && i == n - 1 {
                format!("contrarian-{i}")
            } else {
                format!("specialist-{i}")
            };
            let mut row = SpecialistLedgerRow::new(i, role);
            if self.mode.requires_contrarian() && i == n - 1 {
                row.is_contrarian = true;
            }
            self.ledger.push(row);
        }
        Ok(())
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
        let n = self
            .target_n()
            .ok_or(EffortGateError::IdleInNormal)?;
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
        Ok(())
    }

    /// Partial synthesis after abort (S of N allowed).
    pub fn mark_partial_synthesis(&mut self) {
        self.synthesis_complete = true;
        if self.pursuit != PursuitState::Waived {
            self.pursuit = PursuitState::PartialReport;
        }
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
    /// (writes stay allowed under elevated sticky mode).
    /// Returns whether a new team run was begun.
    pub fn on_session_turn_start(&mut self, task_text: &str) -> Result<bool, EffortGateError> {
        if !self.mode.is_elevated() {
            return Ok(false);
        }
        if self.solo_waiver {
            self.pursuit = PursuitState::Waived;
            return Ok(false);
        }
        if is_trivial_task(task_text) {
            // Trivial short-circuit: Waived so write tools stay available.
            self.pursuit = PursuitState::Waived;
            return Ok(false);
        }
        // Already pursuing — keep the open run.
        if self.pursuit == PursuitState::Pursuing && !self.ledger.is_empty() {
            return Ok(false);
        }
        // Fresh non-trivial work under elevated mode → open N slots.
        // Clears any prior synthesis_complete so execute re-gates.
        self.begin_team_run()?;
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
            !r.outside_n
                && r.task_id.is_none()
                && matches!(r.status, SpecialistStatus::Pending)
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
        self.ledger
            .iter()
            .filter(|r| !r.outside_n)
            .all(|r| {
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
            .ok_or_else(|| EffortGateError::UnknownSlot(usize::MAX))?;
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
             {contrarian} Do not claim full-team completion with fewer than N successful \
             specialist reports. Do not re-spawn the fixed team — use the team report package \
             when present. Execute/write only after synthesis; any implementer runs outside N.\
             {plan}{solo}\n\
             Effort mode never enables always-approve/yolo.",
            mode = self.mode.as_str(),
            n = n,
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
            !r.outside_n
                && r.task_id.is_none()
                && matches!(r.status, SpecialistStatus::Pending)
        })
    }

    /// Snapshot fields for TUI chrome (mode pill + S of N + Partial/Waived).
    pub fn chrome_state(&self) -> EffortChromeState {
        EffortChromeState {
            mode: self.mode,
            pursuit: self.pursuit,
            successful: self.successful_count(),
            target_n: self.target_n(),
            solo_waiver: self.solo_waiver,
        }
    }
}

// ── TUI chrome labels (pure) ───────────────────────────────────────────────

/// Effort fields the TUI needs for the status-bar chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffortChromeState {
    pub mode: EffortMode,
    pub pursuit: PursuitState,
    pub successful: usize,
    pub target_n: Option<usize>,
    pub solo_waiver: bool,
}

impl EffortChromeState {
    /// Status-bar label from shell effort state, or `None` when Normal
    /// (elevated chrome must disappear).
    ///
    /// Examples: `Expert`, `Expert 2 of 4`, `Heavy Partial 3 of 16`,
    /// `Expert Waived`.
    pub fn status_label(self) -> Option<String> {
        format_effort_chrome_label(self)
    }
}

/// Format the durable TUI effort chrome label from shipped tracker fields.
///
/// Drive this helper from unit tests and from the wire payload builder so
/// progress math is not reimplemented in the pager.
pub fn format_effort_chrome_label(state: EffortChromeState) -> Option<String> {
    if !state.mode.is_elevated() {
        return None;
    }
    let name = match state.mode {
        EffortMode::Expert => "Expert",
        EffortMode::Heavy => "Heavy",
        EffortMode::Normal => return None,
    };
    if state.solo_waiver || state.pursuit == PursuitState::Waived {
        return Some(format!("{name} Waived"));
    }
    let n = state.target_n.unwrap_or(0);
    let s = state.successful;
    match state.pursuit {
        PursuitState::PartialReport => {
            if n > 0 {
                Some(format!("{name} Partial {s} of {n}"))
            } else {
                Some(format!("{name} Partial"))
            }
        }
        PursuitState::Pursuing | PursuitState::Aborting => {
            if n > 0 {
                Some(format!("{name} {s} of {n}"))
            } else {
                Some(name.to_string())
            }
        }
        PursuitState::Idle | PursuitState::Waived => Some(name.to_string()),
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
}

impl EffortChromeState {
    pub fn to_wire(self) -> EffortChromeWire {
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
}

/// Build N specialist briefs for a mandatory team run under `mode`.
///
/// Expert → 4; Heavy → 16 with the last slot marked contrarian.
/// Returns empty when mode is Normal.
pub fn build_specialist_briefs(mode: EffortMode, task_text: &str) -> Vec<SpecialistBrief> {
    let Some(n) = mode.team_size_default() else {
        return Vec::new();
    };
    let task = task_text.trim();
    let task = if task.is_empty() {
        "(no task text — analyze the current session context and codebase)"
    } else {
        task
    };
    let mode_name = mode.as_str();
    (0..n)
        .map(|i| {
            let is_contrarian = mode.requires_contrarian() && i == n - 1;
            let role = if is_contrarian {
                format!("contrarian-{i}")
            } else {
                format!("specialist-{i}")
            };
            let description = if is_contrarian {
                format!("{mode_name} contrarian specialist {i}/{n}")
            } else {
                format!("{mode_name} specialist {i}/{n}")
            };
            let angle = specialist_angle(i, n, is_contrarian);
            let prompt = format!(
                "You are specialist slot {i} of {n} on a **mandatory** {mode_name} effort-mode \
                 analytic team (join-all). The shell launched you; do not spawn further subagents.\n\
                 \n\
                 **Role:** {role}\n\
                 **Analytic angle:** {angle}\n\
                 \n\
                 **User task:**\n{task}\n\
                 \n\
                 Produce a structured specialist report: findings, evidence (paths/symbols), \
                 risks, and an independent verdict. Stay analytic and non-writing \
                 (read/search only). End with a short summary the lead agent can synthesize."
            );
            SpecialistBrief {
                slot: i,
                role,
                is_contrarian,
                description,
                prompt,
            }
        })
        .collect()
}

fn specialist_angle(slot: usize, n: usize, is_contrarian: bool) -> &'static str {
    if is_contrarian {
        return "Contrarian: challenge assumptions, find holes, argue the strongest case against the leading approach";
    }
    // Cycle distinct angles so N=4 and N=16 both get diversity without N templates.
    match slot % 4 {
        0 => "Correctness / verification: invariants, edge cases, tests, failure modes",
        1 => "Architecture / design: structure, coupling, alternatives, migration risk",
        2 => "Implementation / ops: build, integration, performance, operability",
        _ => {
            if n > 4 && slot >= n / 2 {
                "Synthesis prep: cross-cut gaps, residual risks, readiness criteria"
            } else {
                "Product / UX / operator impact: clarity, observability, user-visible behavior"
            }
        }
    }
}

/// Format joined specialist reports for injection into the leader turn.
pub fn format_team_report_package(
    mode: EffortMode,
    progress: &str,
    reports: &[(SpecialistBrief, String)],
) -> String {
    let mode_name = mode.as_str();
    let mut out = format!(
        "Effort mode: {mode_name} — **mandatory team complete** ({progress}). \
         Synthesize these specialist reports into your answer. Do not re-run the fixed team.\n"
    );
    for (brief, body) in reports {
        let tag = if brief.is_contrarian {
            "contrarian"
        } else {
            "specialist"
        };
        out.push_str(&format!(
            "\n--- {tag} slot {} ({}) ---\n{}\n",
            brief.slot, brief.role, body
        ));
    }
    out
}

// ── Errors ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffortGateError {
    IdleInNormal,
    NoTeamRun,
    UnderCount { s: usize, n: usize, pursuing: bool },
    MissingContrarian { s: usize, n: usize },
    PartialNotFullTeam { s: usize, n: usize },
    WaivedNotFullTeam,
    ExecuteBeforeSynthesis,
    PlanBlocksExecute,
    UnknownSlot(usize),
    ReplaceCap { slot: usize, cap: u32 },
    ReplaceSuccessForbidden(usize),
    NotHardStopped,
    NoContinueWave,
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
                write!(f, "Plan mode + Effort: non-writing until plan allows execute")
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
        }
    }
}

impl std::error::Error for EffortGateError {}

// ── Arg parse helpers (slash resolve) ──────────────────────────────────────

/// Strip leading `--solo` tokens from args; return (solo, remaining task).
pub fn parse_solo_and_task(args: &str) -> (bool, Option<String>) {
    let mut solo = false;
    let mut rest: Vec<&str> = Vec::new();
    for tok in args.split_whitespace() {
        if tok == "--solo" {
            solo = true;
        } else {
            rest.push(tok);
        }
    }
    let task = if rest.is_empty() {
        None
    } else {
        Some(rest.join(" "))
    };
    (solo, task)
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
        "audit",
        "refactor",
        "multi-file",
        "research",
        "design",
        "investigate",
        "implement",
        "migrate",
    ];
    if HARD.iter().any(|k| t.contains(k)) {
        return false;
    }
    // Very short typo/rename style.
    t.len() <= 40
        && (t.starts_with("fix typo")
            || t.starts_with("rename ")
            || t.starts_with("typo")
            || t == "ok"
            || t == "thanks")
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
        t.record_outcome(0, SpecialistStatus::Timeout, None).unwrap();
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
        assert_eq!(t.successful_count(), 15);
        assert!(matches!(
            t.can_claim_full_team(),
            Err(EffortGateError::UnderCount { .. }) | Err(EffortGateError::MissingContrarian { .. })
        ));
        // Fix contrarian slot.
        let cslot = t.ledger.iter().find(|r| r.is_contrarian).unwrap().slot;
        t.record_outcome(cslot, SpecialistStatus::Success, Some("c".into()))
            .unwrap();
        // Still 15 non-contrarian + 1 contrarian = 16, but we only have 15 if one failed earlier...
        // We failed contrarian then succeeded — count is 16.
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
        assert!(t
            .on_session_turn_start("architect multi-file auth migration")
            .unwrap());
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
        let expert = build_specialist_briefs(EffortMode::Expert, "audit auth");
        assert_eq!(expert.len(), 4);
        assert!(expert.iter().all(|b| !b.is_contrarian));
        assert!(expert[0].prompt.contains("audit auth"));
        assert!(expert[0].description.contains("expert specialist"));

        let heavy = build_specialist_briefs(EffortMode::Heavy, "deep audit");
        assert_eq!(heavy.len(), 16);
        assert!(heavy[15].is_contrarian);
        assert_eq!(heavy[15].role, "contrarian-15");
        assert!(heavy[15].prompt.contains("Contrarian"));
        assert!(heavy.iter().take(15).all(|b| !b.is_contrarian));

        assert!(build_specialist_briefs(EffortMode::Normal, "x").is_empty());
    }

    #[test]
    fn needs_mandatory_fanout_while_pending_unbound() {
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        assert!(!t.needs_mandatory_fanout());
        t.begin_team_run().unwrap();
        assert!(t.needs_mandatory_fanout());
        // Bind one slot — still needs fanout for remaining.
        assert_eq!(t.assign_next_pending_task("s0"), Some(0));
        assert!(t.needs_mandatory_fanout());
        for i in 1..4 {
            assert_eq!(t.assign_next_pending_task(format!("s{i}")), Some(i));
        }
        assert!(!t.needs_mandatory_fanout());
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
        assert!(pkg.contains("slot 3"));
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
            format_effort_chrome_label(EffortChromeState {
                mode: EffortMode::Normal,
                pursuit: PursuitState::Idle,
                successful: 0,
                target_n: None,
                solo_waiver: false,
            }),
            None
        );

        // Sticky Expert idle → mode name only.
        assert_eq!(
            format_effort_chrome_label(EffortChromeState {
                mode: EffortMode::Expert,
                pursuit: PursuitState::Idle,
                successful: 0,
                target_n: Some(4),
                solo_waiver: false,
            }),
            Some("Expert".into())
        );

        // Pursuing with S of N.
        assert_eq!(
            format_effort_chrome_label(EffortChromeState {
                mode: EffortMode::Expert,
                pursuit: PursuitState::Pursuing,
                successful: 2,
                target_n: Some(4),
                solo_waiver: false,
            }),
            Some("Expert 2 of 4".into())
        );
        assert_eq!(
            format_effort_chrome_label(EffortChromeState {
                mode: EffortMode::Heavy,
                pursuit: PursuitState::Pursuing,
                successful: 12,
                target_n: Some(16),
                solo_waiver: false,
            }),
            Some("Heavy 12 of 16".into())
        );

        // Partial / Waived.
        assert_eq!(
            format_effort_chrome_label(EffortChromeState {
                mode: EffortMode::Heavy,
                pursuit: PursuitState::PartialReport,
                successful: 3,
                target_n: Some(16),
                solo_waiver: false,
            }),
            Some("Heavy Partial 3 of 16".into())
        );
        assert_eq!(
            format_effort_chrome_label(EffortChromeState {
                mode: EffortMode::Expert,
                pursuit: PursuitState::Waived,
                successful: 0,
                target_n: Some(4),
                solo_waiver: false,
            }),
            Some("Expert Waived".into())
        );
        assert_eq!(
            format_effort_chrome_label(EffortChromeState {
                mode: EffortMode::Expert,
                pursuit: PursuitState::Idle,
                successful: 0,
                target_n: Some(4),
                solo_waiver: true,
            }),
            Some("Expert Waived".into())
        );

        // Tracker path: begin team → chrome shows 0 of 4; success → 1 of 4.
        let mut t = EffortModeTracker::new(tmp());
        t.set_mode(EffortMode::Expert, false);
        assert_eq!(t.chrome_state().status_label().as_deref(), Some("Expert"));
        t.begin_team_run().unwrap();
        assert_eq!(
            t.chrome_state().status_label().as_deref(),
            Some("Expert 0 of 4")
        );
        t.record_outcome(0, SpecialistStatus::Success, Some("a".into()))
            .unwrap();
        assert_eq!(
            t.chrome_state().status_label().as_deref(),
            Some("Expert 1 of 4")
        );
        t.clear_to_normal();
        assert_eq!(t.chrome_state().status_label(), None);
    }
}
