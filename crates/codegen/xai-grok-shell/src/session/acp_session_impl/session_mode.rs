//! Session/plan-mode concern for `SessionActor` (`handle_session_mode`,
//! plan-mode reminders and persistence, active-template detection).
use super::*;
pub(super) fn prompt_mode_from_session_mode_id(session_mode_id: &acp::SessionModeId) -> PromptMode {
    use xai_grok_tools::types::SessionMode;
    match SessionMode::from_id(session_mode_id.0.as_ref()) {
        SessionMode::Plan => PromptMode::Plan,
        SessionMode::Ask => PromptMode::Ask,
        SessionMode::Default => PromptMode::Agent,
    }
}
/// Pass-through twin: no toolset in this build carries a plan-gated tool.
pub(super) fn filter_cursor_tools_by_plan_mode(
    defs: Vec<ToolDefinition>,
    _plan_active: bool,
) -> Vec<ToolDefinition> {
    defs
}
impl SessionActor {
    pub(super) fn apply_prompt_modes_to_snapshot(&self, snapshot: &mut TurnDeltaSnapshot) {
        snapshot.start_prompt_mode = Some(self.turn_start_prompt_mode.lock().to_string());
        snapshot.end_prompt_mode = Some(self.turn_prompt_mode.lock().to_string());
    }
    /// `false` twin: this template integration is not compiled into this
    /// build, so no session runs it. Keeps ungated call sites compiling in
    /// both configurations.
    pub(super) fn is_cursor_harness(&self) -> bool {
        false
    }
    pub(super) async fn handle_session_mode(&self, session_mode_id: acp::SessionModeId) {
        use xai_grok_tools::types::SessionMode;
        let prompt_mode = prompt_mode_from_session_mode_id(&session_mode_id);
        *self.current_prompt_mode.lock() = prompt_mode;
        let mode = SessionMode::from_id(session_mode_id.0.as_ref());
        if mode.is_plan() {
            let entered = self.plan_mode.lock().enter_pending();
            if entered {
                self.persist_plan_mode_state();
                self.enqueue_current_mode_update(acp::SessionModeId::new(
                    SessionMode::Plan.as_id(),
                ));
            }
            tracing::info!(
                session_id = % self.session_info.id.0, entered,
                "Plan mode toggled ON (Pending)"
            );
            let turn_in_flight = self.state.lock().await.running_task.is_some();
            if entered && turn_in_flight {
                self.activate_plan_mode_mid_turn().await;
            }
            xai_grok_telemetry::session_ctx::log_event(
                xai_grok_telemetry::events::PlanModeToggled {
                    enabled: true,
                    trigger: xai_grok_telemetry::events::PlanModeTrigger::User,
                    turn_in_flight,
                    was_previously_active: !entered,
                },
            );
            if entered {
                tracing::info_span!(
                    "session.permission_mode_changed",
                    from_mode =
                        super::telemetry::permission_mode_label(self.permissions.is_yolo_mode()),
                    to_mode = "plan",
                    trigger = "user",
                    enabled = true,
                )
                .in_scope(|| {});
            }
            return;
        }
        let was_plan = {
            let tracker = self.plan_mode.lock();
            tracker.state() != crate::session::plan_mode::PlanModeState::Inactive
        };
        if was_plan {
            let turn_in_flight = self.state.lock().await.running_task.is_some();
            self.plan_mode.lock().user_exit(turn_in_flight);
            self.persist_plan_mode_state();
            self.enqueue_current_mode_update(session_mode_id.clone());
            tracing::info!(
                session_id = % self.session_info.id.0, new_mode = % session_mode_id.0,
                turn_in_flight, "Plan mode toggled OFF"
            );
            xai_grok_telemetry::session_ctx::log_event(
                xai_grok_telemetry::events::PlanModeToggled {
                    enabled: false,
                    trigger: xai_grok_telemetry::events::PlanModeTrigger::User,
                    turn_in_flight,
                    was_previously_active: true,
                },
            );
            tracing::info_span!(
                "session.permission_mode_changed", from_mode = "plan", to_mode = %
                session_mode_id.0, trigger = "user", enabled = false,
            )
            .in_scope(|| {});
        }
        let agent_def = match session_mode_id.0.as_ref() {
            "browser_use" => Some(AgentDefinition::browser_use()),
            name => {
                let cwd = self.tool_context.cwd.as_path();
                xai_grok_agent::discovery::by_name_in_cwd(name, cwd)
            }
        };
        if let Some(ref def) = agent_def {
            tracing::info!(
                session_id = % self.session_info.id.0, agent_name = % def.name,
                agent_scope = % def.scope, prompt_mode = ? def.prompt_mode,
                has_completion_req = def.completion_requirement.is_some(), tool_configs =
                def.tool_config.tools.len(), "Resolved AgentDefinition for session mode"
            );
            self.agent
                .borrow()
                .update_policies_from_definition(def)
                .await;
            *self.active_agent_type.lock() = Some(def.name.clone());
        }
        if let Some(ref def) = agent_def {
            let new_prompt = self.agent.borrow().render_prompt_for_definition(def).await;
            let mut conversation = self.chat_state_handle.get_conversation().await;
            for item in conversation.iter_mut() {
                if let ConversationItem::System(sys) = item {
                    sys.content = std::sync::Arc::<str>::from(new_prompt);
                    break;
                }
            }
            self.chat_state_handle.replace_conversation(conversation);
        }
    }
    /// Bring the plan-mode tracker into agreement with the prompt's mode.
    ///
    /// Mirrors `handle_session_mode` but driven from `_meta.mode` on the
    /// prompt — the only signal the client sends. Both transitions are
    /// idempotent, so `set_mode`-driven flows are unaffected.
    pub(super) fn reconcile_plan_mode_with_prompt(&self, prompt_mode: PromptMode) {
        use crate::session::plan_mode::PlanModeState;
        *self.current_prompt_mode.lock() = prompt_mode;
        match prompt_mode {
            PromptMode::Plan => {
                let entered = self.plan_mode.lock().enter_pending();
                if entered {
                    self.persist_plan_mode_state();
                }
            }
            PromptMode::Agent | PromptMode::Ask => {
                let was_plan = {
                    let tracker = self.plan_mode.lock();
                    tracker.state() != PlanModeState::Inactive
                };
                if was_plan {
                    self.plan_mode.lock().user_exit(false);
                    self.persist_plan_mode_state();
                }
            }
        }
    }
    /// Inject plan mode system-reminders into the conversation.
    ///
    /// Called once per turn from `handle_prompt()`, before the user's actual
    /// message is pushed. Handles three mutually-ordered cases:
    ///
    /// 1. **Pending → Active**: First prompt after user toggled plan mode on.
    ///    Injects the full (or reentry) reminder and transitions to Active.
    /// 2. **Already Active**: Subsequent prompts while plan mode is on.
    ///    Injects an alternating full/sparse per-turn reminder.
    /// 3. **Exit reminder**: One-shot reminder after plan mode was exited.
    ///    Injected once, then the flag is cleared.
    ///
    /// All reminders are pushed as `<system-reminder>`-wrapped user messages
    /// so the model sees them in the same turn as the user's prompt.
    /// Tool names are resolved at render time via `TemplateRenderer`.
    pub(super) async fn inject_plan_mode_reminders(&self) {
        use crate::session::plan_mode::{
            PlanModeState, plan_mode_exit_reminder_template, plan_mode_reminder_full_template,
            plan_mode_reminder_sparse_template,
        };
        let use_cursor_reminders = self.is_cursor_harness();
        let push_reminder = |this: &Self, content: &str| {
            this.push_system_reminder_with_tag(content, this.reminder_wrapper_tag());
        };
        let mut injected_this_turn = false;
        let activation = {
            let tracker = self.plan_mode.lock();
            (tracker.state() == PlanModeState::Pending)
                .then(|| (tracker.is_reentry(), tracker.plan_file_path().to_path_buf()))
        };
        if let Some((is_reentry, plan_path)) = activation {
            self.plan_mode.lock().activate();
            self.persist_plan_mode_state();
            let plan_has_content =
                crate::session::plan_mode::plan_file_has_content(&plan_path).await;
            let template = self.plan_activation_template(is_reentry);
            if let Some(rendered) = self
                .render_plan_template(template, &plan_path, plan_has_content)
                .await
            {
                push_reminder(self, &rendered);
                injected_this_turn = true;
                self.plan_mode.lock().record_reminder_injected();
                self.persist_plan_mode_state();
                tracing::info!(
                    session_id = % self.session_info.id.0, is_reentry,
                    uses_template_reminders = use_cursor_reminders,
                    "Plan mode activated: injected system-reminder"
                );
            }
        }
        if !injected_this_turn {
            let per_turn = {
                let tracker = self.plan_mode.lock();
                tracker.is_active().then(|| {
                    (
                        tracker.should_use_full_reminder(),
                        tracker.plan_file_path().to_path_buf(),
                    )
                })
            };
            if let Some((use_full, plan_path)) = per_turn {
                let plan_has_content =
                    crate::session::plan_mode::plan_file_has_content(&plan_path).await;
                let template = if use_full {
                    plan_mode_reminder_full_template()
                } else {
                    plan_mode_reminder_sparse_template()
                };
                if let Some(rendered) = self
                    .render_plan_template(template, &plan_path, plan_has_content)
                    .await
                {
                    push_reminder(self, &rendered);
                    self.plan_mode.lock().record_reminder_injected();
                    self.persist_plan_mode_state();
                }
            }
        }
        if self.plan_mode.lock().has_pending_exit_reminder() {
            let plan_path = self.plan_mode.lock().plan_file_path().to_path_buf();
            let template = plan_mode_exit_reminder_template();
            if let Some(rendered) = self.render_plan_template(template, &plan_path, false).await {
                push_reminder(self, &rendered);
            }
            self.plan_mode.lock().clear_pending_exit_reminder();
            self.persist_plan_mode_state();
        }
    }
    /// Activate plan mode for a turn that is already running.
    ///
    /// Mid-turn counterpart of `inject_plan_mode_reminders` case 1: the user
    /// toggled plan mode ON (Shift+Tab) while the model was thinking, so the
    /// tracker sits in `Pending` and the running turn would otherwise proceed
    /// without any plan-mode instruction. Activate immediately (so
    /// `is_active()` tool gating applies to subsequent calls) and buffer the
    /// activation reminder on the tracker; `flush_pending_skill_reminders`
    /// delivers it at the running turn's next safe point (loop top / after
    /// each tool batch) — or, if the turn ends first, the cancel/idle flush
    /// lands it for the next turn. Buffering (vs a direct conversation push)
    /// keeps the in-flight batch's tool_result blocks adjacent, and lets a
    /// toggle-off withdraw an undelivered reminder (`user_exit`).
    ///
    /// No-op unless the tracker is `Pending`: `enter_pending`'s
    /// `ExitPending → Active` re-entry needs no reminder (the model already
    /// has plan-mode context and no exit reminder was injected yet).
    ///
    /// A failed template render still activates (without a buffer), keeping
    /// gating in lockstep with the turn-start path.
    pub(super) async fn activate_plan_mode_mid_turn(&self) {
        use crate::session::plan_mode::PlanModeState;
        let activation = {
            let tracker = self.plan_mode.lock();
            (tracker.state() == PlanModeState::Pending)
                .then(|| (tracker.is_reentry(), tracker.plan_file_path().to_path_buf()))
        };
        let Some((is_reentry, plan_path)) = activation else {
            return;
        };
        let plan_has_content = crate::session::plan_mode::plan_file_has_content(&plan_path).await;
        let template = self.plan_activation_template(is_reentry);
        let rendered = self
            .render_plan_template(template, &plan_path, plan_has_content)
            .await;
        let tag = self.reminder_wrapper_tag();
        let buffered = rendered.is_some();
        let activated = match rendered {
            Some(rendered) => self
                .plan_mode
                .lock()
                .activate_mid_turn(format!("<{tag}>\n{rendered}\n</{tag}>")),
            None => {
                tracing::warn!(
                    session_id = % self.session_info.id.0,
                    "Mid-turn plan activation: reminder render failed; \
                     activating without a buffered reminder"
                );
                self.plan_mode.lock().activate()
            }
        };
        if !activated {
            return;
        }
        self.persist_plan_mode_state();
        tracing::info!(
            session_id = % self.session_info.id.0, is_reentry, buffered,
            "Plan mode activated mid-turn"
        );
    }
    /// The activation reminder template for the active template (no
    /// first-entry/reentry distinction), or grok's reentry/full variant.
    /// Shared by turn-start injection (`inject_plan_mode_reminders` case 1)
    /// and the mid-turn toggle (`activate_plan_mode_mid_turn`).
    fn plan_activation_template(&self, is_reentry: bool) -> &'static str {
        use crate::session::plan_mode::{
            plan_mode_reentry_reminder_template, plan_mode_reminder_full_template,
        };
        if is_reentry {
            plan_mode_reentry_reminder_template()
        } else {
            plan_mode_reminder_full_template()
        }
    }
    /// Render a plan mode template via the tool bridge's `TemplateRenderer`.
    ///
    /// Passes `plan_path` and `plan_has_content` as extra context alongside the
    /// registry's `tools.by_kind.*` mappings.
    pub(super) async fn render_plan_template(
        &self,
        template: &str,
        plan_path: &std::path::Path,
        plan_has_content: bool,
    ) -> Option<String> {
        let extra = serde_json::json!(
            { "plan_path" : plan_path.display().to_string(), "plan_has_content" :
            plan_has_content, }
        );
        self.agent
            .borrow()
            .tool_bridge()
            .render_prompt(template, &extra)
            .await
    }
    /// Persist the current plan mode state to disk.
    ///
    /// Called after every state transition so plan mode survives
    /// session reload/resume/reconnect.
    pub(super) fn persist_plan_mode_state(&self) {
        let snapshot = self.plan_mode.lock().snapshot();
        let _ = self
            .notifications
            .persistence_tx
            .send(PersistenceMsg::PlanModeState(snapshot));
    }

    /// Emit sticky effort chrome to the pager (mode + S of N + Partial/Waived).
    ///
    /// Sync-safe: persists + enqueues on the session event FIFO (same pattern
    /// as plan `CurrentModeUpdate` enqueue) so live UI cannot drift from the
    /// ledger without relying on model prose.
    pub(super) fn emit_effort_chrome_update(&self) {
        use crate::extensions::notification::{
            SessionNotification as XaiSessionNotification, SessionUpdate as XaiSessionUpdate,
        };
        use crate::session::replay_events::SessionEvent;

        let wire = self.effort_mode.lock().chrome_state().to_wire();
        let update = XaiSessionUpdate::EffortModeUpdated {
            mode: wire.mode,
            pursuit: wire.pursuit,
            label: wire.label,
            successful: wire.successful,
            target_n: wire.target_n,
            solo_waiver: wire.solo_waiver,
        };
        // Durable for resume/replay viewers.
        self.persist_xai_update_only(update.clone());
        let notification = XaiSessionNotification {
            session_id: self.session_info.id.clone(),
            update,
            meta: None,
        };
        let _ = self
            .event_tx
            .send(SessionEvent::Notification(notification.into()));
    }

    /// Inject soft effort-mode policy when sticky mode is Expert/Heavy.
    /// Mirrors plan-mode per-turn reminder injection.
    pub(super) async fn inject_effort_mode_reminders(&self) {
        let plan_active = self.plan_mode.lock().is_active();
        let reminder = self.effort_mode.lock().policy_reminder(plan_active);
        if let Some(body) = reminder {
            self.push_system_reminder_with_tag(&body, self.reminder_wrapper_tag());
            tracing::debug!(
                session_id = %self.session_info.id.0,
                "Effort mode: injected policy system-reminder"
            );
        }
    }

    /// Turn-start hard runtime: open a fixed-team ledger for non-trivial
    /// work under sticky Expert/Heavy (non-solo).
    pub(super) fn effort_on_turn_start(&self, task_text: &str) {
        let mut tracker = self.effort_mode.lock();
        match tracker.on_session_turn_start(task_text) {
            Ok(true) => {
                tracing::info!(
                    session_id = %self.session_info.id.0,
                    mode = tracker.mode().as_str(),
                    n = tracker.target_n(),
                    "effort mode: began fixed team run"
                );
                drop(tracker);
                self.persist_effort_mode_state();
                self.emit_effort_chrome_update();
            }
            Ok(false) => {
                let needs_heavy_confirm =
                    tracker.last_waiver()
                        == crate::session::effort_mode::WaiverReason::NeedsHeavyConfirm;
                let resume = tracker.take_resume_elevated_notice();
                let mode = tracker.mode();
                drop(tracker);
                if resume && mode.is_elevated() {
                    self.push_system_reminder_with_tag(
                        &format!(
                            "Effort mode restored: sticky **{}** is still active from the previous session. \
                             Use /normal to clear, /expert, or /heavy --solo for this turn.",
                            mode.as_str()
                        ),
                        self.reminder_wrapper_tag(),
                    );
                }
                if needs_heavy_confirm {
                    self.push_system_reminder_with_tag(
                        "Heavy mode is sticky but the **first multi-agent team** this session needs \
                         an explicit confirm (cost guard). Reply with `--confirm` or \
                         `/heavy --confirm <task>`, or use `--solo` / `/normal`. \
                         Env `GROK_HEAVY_AUTO_CONFIRM=1` skips this gate.",
                        self.reminder_wrapper_tag(),
                    );
                }
                self.emit_effort_chrome_update();
            }
            Err(e) => {
                tracing::debug!(
                    session_id = %self.session_info.id.0,
                    error = %e,
                    "effort mode: turn-start team begin skipped"
                );
            }
        }
    }

    /// Shell-owned mandatory Expert/Heavy fan-out: spawn N specialists,
    /// join-all, record ledger outcomes, inject report package for the leader.
    ///
    /// Also runs automatic replace waves (up to `max_replace_waves`) when the
    /// first join is short, and a hard-stop `continue` wave when the user
    /// granted one. Does **not** rely on the model calling `spawn_subagent`.
    pub(super) async fn maybe_run_mandatory_effort_team(&self, task_text: &str) {
        // Prepare replace / continue wave if prior join left replaceable slots.
        {
            let mut tracker = self.effort_mode.lock();
            if tracker.try_prepare_replace_wave() {
                tracing::info!(
                    session_id = %self.session_info.id.0,
                    "effort mode: prepared replace / continue wave"
                );
                drop(tracker);
                self.persist_effort_mode_state();
                self.emit_effort_chrome_update();
            }
        }

        let (needs, mode) = {
            let tracker = self.effort_mode.lock();
            (tracker.needs_mandatory_fanout(), tracker.mode())
        };
        if !needs {
            return;
        }

        let Some(event_tx) = self.tool_context.subagent_event_tx.clone() else {
            tracing::warn!(
                session_id = %self.session_info.id.0,
                "effort mode: mandatory team requested but subagent channel missing"
            );
            self.push_system_reminder_with_tag(
                "Effort mode: mandatory specialist team could not start (subagents unavailable). \
                 Writes stay blocked until synthesis, abort, --solo, or /normal.",
                self.reminder_wrapper_tag(),
            );
            return;
        };

        let parent_prompt_id = self
            .current_prompt_id
            .lock()
            .ok()
            .and_then(|g| g.clone());
        let cwd = Some(self.session_info.cwd.clone());
        let parent_session_id = self.session_info.id.0.to_string();

        // Full team or only Pending recovery slots (replace wave).
        // Always build from the ledger so Expert brain selection is not re-rolled.
        let planned = {
            let tracker = self.effort_mode.lock();
            let pending = tracker.pending_slot_briefs(task_text);
            if pending.is_empty() {
                return;
            }
            crate::session::effort_team::plan_pending_slots(pending)
        };
        if planned.is_empty() {
            return;
        }

        let n = mode.team_size_default().unwrap_or(0);
        let wave_size = planned.len();

        // Pre-bind every planned slot → task_id (Running) before spawn so join maps 1:1.
        {
            let mut tracker = self.effort_mode.lock();
            for p in &planned {
                if let Err(e) = tracker.record_outcome(
                    p.brief.slot,
                    crate::session::effort_mode::SpecialistStatus::Running,
                    Some(p.task_id.clone()),
                ) {
                    tracing::warn!(
                        session_id = %self.session_info.id.0,
                        slot = p.brief.slot,
                        error = %e,
                        "effort mode: failed to pre-bind specialist slot"
                    );
                }
            }
            drop(tracker);
            self.persist_effort_mode_state();
            // Show Expert/Heavy S of N before specialists finish.
            self.emit_effort_chrome_update();
        }

        tracing::info!(
            session_id = %self.session_info.id.0,
            mode = mode.as_str(),
            n,
            wave_size,
            "effort mode: launching mandatory specialist team"
        );

        // Join-all loop: first wave + automatic replace waves within budget.
        let mut all_joins: Vec<crate::session::effort_team::SpecialistJoinResult> = Vec::new();
        let mut current_planned = planned;

        loop {
            let joins = crate::session::effort_team::run_planned_team(
                &event_tx,
                &parent_session_id,
                parent_prompt_id.clone(),
                cwd.clone(),
                current_planned,
            )
            .await;

            {
                let mut tracker = self.effort_mode.lock();
                for join in &joins {
                    let status =
                        crate::session::effort_mode::EffortModeTracker::specialist_status_from_join(
                            join.success,
                            join.cancelled,
                            &join.body,
                        );
                    if let Err(e) = tracker.on_session_specialist_outcome(
                        join.brief.slot,
                        status,
                        Some(join.task_id.clone()),
                    ) {
                        tracing::debug!(
                            session_id = %self.session_info.id.0,
                            task_id = %join.task_id,
                            slot = join.brief.slot,
                            error = %e,
                            "effort mode: join record failed"
                        );
                    }
                }
                all_joins.extend(joins);

                // Auto replace-wave while budget remains and still short.
                if tracker.synthesis_complete() {
                    break;
                }
                if !tracker.try_prepare_replace_wave() {
                    break;
                }
                let next_briefs = tracker.pending_slot_briefs(task_text);
                if next_briefs.is_empty() {
                    break;
                }
                current_planned =
                    crate::session::effort_team::plan_pending_slots(next_briefs);
                // Pre-bind replace slots.
                for p in &current_planned {
                    let _ = tracker.record_outcome(
                        p.brief.slot,
                        crate::session::effort_mode::SpecialistStatus::Running,
                        Some(p.task_id.clone()),
                    );
                }
                drop(tracker);
                self.persist_effort_mode_state();
                self.emit_effort_chrome_update();
                tracing::info!(
                    session_id = %self.session_info.id.0,
                    wave_size = current_planned.len(),
                    "effort mode: launching automatic replace wave"
                );
                continue;
            }
        }

        {
            let tracker = self.effort_mode.lock();
            let progress = tracker.progress_label();
            let synth = tracker.synthesis_complete();
            let hard_stop = tracker.is_hard_stop();
            drop(tracker);
            self.persist_effort_mode_state();

            // Package reports for the leader (success and failure bodies).
            // Prefer latest body per slot (replace wave overwrites earlier).
            let mut latest: std::collections::BTreeMap<
                usize,
                (crate::session::effort_mode::SpecialistBrief, String),
            > = std::collections::BTreeMap::new();
            for j in &all_joins {
                latest.insert(j.brief.slot, (j.brief.clone(), j.body.clone()));
            }
            let report_pairs: Vec<_> = latest.into_values().collect();
            let package = crate::session::effort_mode::format_team_report_package(
                mode,
                &progress,
                &report_pairs,
            );
            let mut body = package;
            if synth {
                body.push_str(
                    "\nFull-team synthesis gate is satisfied. Synthesize and answer the user. \
                     Execute/write tools are unlocked (unless plan mode blocks).",
                );
            } else if hard_stop {
                body.push_str(&format!(
                    "\n**Hard-stop:** team short of N ({progress}). Writes stay blocked. \
                     User choices: continue (one more replace wave on the next turn), \
                     --solo, or /normal. A new non-trivial user turn without continue \
                     restarts a fresh team."
                ));
            } else {
                body.push_str(&format!(
                    "\nTeam incomplete ({progress}). Writes stay blocked until full claim, \
                     abort/partial, --solo, or /normal. The next non-trivial user turn \
                     restarts a fresh team if still short."
                ));
            }
            self.push_system_reminder_with_tag(&body, self.reminder_wrapper_tag());

            tracing::info!(
                session_id = %self.session_info.id.0,
                mode = mode.as_str(),
                progress = %progress,
                synthesis_complete = synth,
                hard_stop,
                "effort mode: mandatory team join finished"
            );
            self.emit_effort_chrome_update();
        }
    }

    /// User cancel: abort in-flight effort team → PartialReport (S of N).
    pub(super) fn effort_on_user_cancel(&self) {
        let mut tracker = self.effort_mode.lock();
        if let Some(label) = tracker.on_session_user_cancel() {
            tracing::info!(
                session_id = %self.session_info.id.0,
                progress = %label,
                "effort mode: team aborted to partial report"
            );
            drop(tracker);
            self.persist_effort_mode_state();
            self.emit_effort_chrome_update();
        }
    }

    /// Bind a freshly spawned subagent to the next free specialist slot.
    pub(crate) fn effort_bind_spawned_subagent(&self, subagent_id: &str) {
        let mut tracker = self.effort_mode.lock();
        if let Some(slot) = tracker.assign_next_pending_task(subagent_id) {
            tracing::debug!(
                session_id = %self.session_info.id.0,
                subagent_id,
                slot,
                "effort mode: bound subagent to specialist slot"
            );
            drop(tracker);
            self.persist_effort_mode_state();
            self.emit_effort_chrome_update();
        }
    }

    /// Specialist finished (real SubagentFinished path or faked handle).
    pub(crate) fn effort_on_subagent_finished(&self, subagent_id: &str, status: &str) {
        let specialist =
            crate::session::effort_mode::EffortModeTracker::specialist_status_from_subagent(status);
        let mut tracker = self.effort_mode.lock();
        match tracker.on_session_specialist_finished(subagent_id, specialist) {
            Ok(()) => {
                let done = tracker.synthesis_complete();
                let progress = tracker.progress_label();
                drop(tracker);
                self.persist_effort_mode_state();
                self.emit_effort_chrome_update();
                if done {
                    tracing::info!(
                        session_id = %self.session_info.id.0,
                        progress = %progress,
                        "effort mode: full-team synthesis complete; execute unlocked"
                    );
                }
            }
            Err(crate::session::effort_mode::EffortGateError::UnknownSlot(_)) => {
                // Subagent not part of the effort ledger (normal work).
            }
            Err(e) => {
                tracing::debug!(
                    session_id = %self.session_info.id.0,
                    error = %e,
                    "effort mode: specialist finish ignored"
                );
            }
        }
    }

    /// Specialist completion by slot (faked task handles / tests).
    pub(crate) fn effort_record_specialist_outcome(
        &self,
        slot: usize,
        status: crate::session::effort_mode::SpecialistStatus,
        task_id: Option<String>,
    ) -> Result<(), crate::session::effort_mode::EffortGateError> {
        let result = self
            .effort_mode
            .lock()
            .on_session_specialist_outcome(slot, status, task_id);
        if result.is_ok() {
            self.persist_effort_mode_state();
            self.emit_effort_chrome_update();
        }
        result
    }

    /// Claim full-team finalize (denied when under-count / missing contrarian).
    pub(crate) fn effort_claim_full_team(
        &self,
    ) -> Result<(), crate::session::effort_mode::EffortGateError> {
        self.effort_mode.lock().on_session_claim_full_team()
    }

    /// Turn-end: finalize synthesis when all fixed slots are terminal.
    pub(super) fn effort_on_turn_end(&self) {
        let mut tracker = self.effort_mode.lock();
        let done = tracker.on_session_turn_end();
        if done || tracker.pursuit() == crate::session::effort_mode::PursuitState::Pursuing {
            drop(tracker);
            self.persist_effort_mode_state();
            self.emit_effort_chrome_update();
            if done {
                tracing::info!(
                    session_id = %self.session_info.id.0,
                    "effort mode: turn-end synthesis finalize succeeded"
                );
            }
        }
    }

    /// Apply sticky effort mode (Expert/Heavy/Normal) and persist snapshot.
    pub(super) fn apply_effort_mode(
        &self,
        mode: crate::session::effort_mode::EffortMode,
        solo: bool,
    ) {
        let previous = self.effort_mode.lock().mode();
        {
            let mut tracker = self.effort_mode.lock();
            if mode == crate::session::effort_mode::EffortMode::Normal {
                // Mid-flight /normal: abort remaining specialists then clear.
                let _ = tracker.on_session_user_cancel();
                tracker.clear_to_normal();
            } else {
                tracker.set_mode(mode, solo);
            }
        }
        self.persist_effort_mode_state();
        self.emit_effort_chrome_update();
        // If Heavy was just set, surface first-team confirm guidance once.
        if mode == crate::session::effort_mode::EffortMode::Heavy && !solo {
            let unlocked = self.effort_mode.lock().heavy_unlocked();
            if !unlocked {
                self.push_system_reminder_with_tag(
                    "Heavy is sticky. The **first multi-agent team** this session needs `--confirm` \
                     (or `GROK_HEAVY_AUTO_CONFIRM=1`). Use `--solo` for single-leader turns, \
                     `--force-team` to override trivial waiver.",
                    self.reminder_wrapper_tag(),
                );
            }
        }
        tracing::info!(
            session_id = %self.session_info.id.0,
            mode = mode.as_str(),
            previous = previous.as_str(),
            solo,
            "effort mode set",
        );
        tracing::info_span!(
            "session.effort_mode_toggled",
            mode = mode.as_str(),
            previous = previous.as_str(),
            solo,
        )
        .in_scope(|| {});
    }

    /// Persist sticky effort mode to disk (best-effort via persistence channel).
    pub(super) fn persist_effort_mode_state(&self) {
        let snapshot = self.effort_mode.lock().snapshot();
        let _ = self
            .notifications
            .persistence_tx
            .send(PersistenceMsg::EffortModeState(snapshot));
    }
}
