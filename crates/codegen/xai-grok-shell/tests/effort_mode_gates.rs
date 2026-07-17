//! Integration tests for EffortMode hard gates + sticky mode + slash resolve
//! (public API on the shipped library surface).
//!
//! These live outside the lib's `#[cfg(test)]` graph so they compile against
//! the non-test library surface (avoids dependency `cfg(test)` helper gaps).

use std::path::PathBuf;
use xai_grok_shell::session::effort_mode::{
    EffortChromeState, EffortGateError, EffortMode, EffortModeTracker, PursuitState,
    SpecialistStatus, build_specialist_briefs, format_effort_chrome_label,
    format_team_report_package, is_trivial_task, parse_solo_and_task,
};
use xai_grok_shell::session::{EffortSlashResolve, resolve_effort_slash};

fn tmp() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../target/effort-mode-integration-test"
    ))
}

// ── Criterion 1: slash resolve (shipped path) ──────────────────────────────

#[test]
fn slash_expert_heavy_normal_resolve_as_builtins() {
    assert_eq!(
        resolve_effort_slash("/expert", true, &[]),
        EffortSlashResolve::Builtin {
            mode: EffortMode::Expert,
            task: None,
            solo: false,
        }
    );
    assert_eq!(
        resolve_effort_slash("/heavy deep audit", true, &[]),
        EffortSlashResolve::Builtin {
            mode: EffortMode::Heavy,
            task: Some("deep audit".into()),
            solo: false,
        }
    );
    assert_eq!(
        resolve_effort_slash("/normal", true, &[]),
        EffortSlashResolve::Builtin {
            mode: EffortMode::Normal,
            task: None,
            solo: false,
        }
    );
}

#[test]
fn slash_solo_parsed_and_stripped_at_resolve() {
    assert_eq!(
        resolve_effort_slash("/expert --solo fix the flaky test", true, &[]),
        EffortSlashResolve::Builtin {
            mode: EffortMode::Expert,
            task: Some("fix the flaky test".into()),
            solo: true,
        }
    );
}

#[test]
fn slash_builtin_wins_over_same_named_skill() {
    assert_eq!(
        resolve_effort_slash("/expert --solo task", true, &["expert"]),
        EffortSlashResolve::Builtin {
            mode: EffortMode::Expert,
            task: Some("task".into()),
            solo: true,
        }
    );
}

#[test]
fn slash_flag_off_falls_through_to_skill_or_passthrough() {
    // Gate off + same-named skill → skill reclaim.
    assert_eq!(
        resolve_effort_slash("/expert fix it", false, &["expert"]),
        EffortSlashResolve::Skill {
            name: "expert".into()
        }
    );
    // Gate off + no skill → ordinary pass-through.
    assert_eq!(
        resolve_effort_slash("/expert fix it", false, &[]),
        EffortSlashResolve::PassThrough
    );
}

// ── Criterion 2–3: join / ledger / abort (session hooks = SessionActor path) ─

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
}

#[test]
fn sticky_mode_snapshot_round_trip() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Heavy, false);
    let snap = t.snapshot();
    let restored = EffortModeTracker::from_snapshot(tmp(), snap);
    assert_eq!(restored.mode(), EffortMode::Heavy);
}

#[test]
fn snapshot_resume_keeps_partial_and_full_synthesis_unlock() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    t.begin_team_run().unwrap();
    t.on_session_specialist_outcome(0, SpecialistStatus::Success, Some("a".into()))
        .unwrap();
    let _ = t.on_session_user_cancel();
    assert!(t.synthesis_complete());
    assert!(t.may_execute_writes(false).is_ok());
    let restored = EffortModeTracker::from_snapshot(tmp(), t.snapshot());
    assert!(restored.synthesis_complete());
    assert!(restored.may_execute_writes(false).is_ok());
    assert_eq!(restored.pursuit(), PursuitState::PartialReport);

    let mut full = EffortModeTracker::new(tmp());
    full.set_mode(EffortMode::Expert, false);
    full.begin_team_run().unwrap();
    for i in 0..4 {
        full.on_session_specialist_outcome(i, SpecialistStatus::Success, Some(format!("t{i}")))
            .unwrap();
    }
    assert!(full.synthesis_complete());
    let restored = EffortModeTracker::from_snapshot(tmp(), full.snapshot());
    assert!(restored.synthesis_complete());
    assert!(restored.may_execute_writes(false).is_ok());
}

#[test]
fn short_team_next_turn_reopens_mandatory_fanout() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    t.on_session_turn_start("architect multi-file auth migration")
        .unwrap();
    for i in 0..4 {
        t.on_session_specialist_outcome(i, SpecialistStatus::Failed, None)
            .unwrap();
    }
    assert!(!t.synthesis_complete());
    assert!(matches!(
        t.may_execute_writes(false),
        Err(EffortGateError::ExecuteBeforeSynthesis)
    ));
    assert!(t
        .on_session_turn_start("architect multi-file retry of auth")
        .unwrap());
    assert!(t.needs_mandatory_fanout());
    assert_eq!(t.ledger().len(), 4);
}

#[test]
fn replace_wave_and_empty_body_do_not_count() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    t.begin_team_run().unwrap();
    t.on_session_specialist_outcome(0, SpecialistStatus::Success, Some("ok-body".into()))
        .unwrap();
    t.on_session_specialist_outcome(1, SpecialistStatus::Failed, None)
        .unwrap();
    t.on_session_specialist_outcome(2, SpecialistStatus::Failed, None)
        .unwrap();
    t.on_session_specialist_outcome(3, SpecialistStatus::Failed, None)
        .unwrap();
    assert!(t.try_prepare_replace_wave());
    assert!(t.needs_mandatory_fanout());

    // Empty join body → EmptyReport, not Success.
    assert_eq!(
        EffortModeTracker::specialist_status_from_join(true, false, "ok"),
        SpecialistStatus::EmptyReport
    );
    assert_eq!(
        EffortModeTracker::specialist_status_from_join(
            true,
            false,
            "Detailed findings about the auth path."
        ),
        SpecialistStatus::Success
    );
}

#[test]
fn expert_full_team_requires_4_successes() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    // Session turn-start hook (what SessionActor::effort_on_turn_start calls).
    assert!(t
        .on_session_turn_start("architect multi-file auth migration")
        .unwrap());
    for i in 0..3 {
        t.on_session_specialist_outcome(i, SpecialistStatus::Success, Some(format!("t{i}")))
            .unwrap();
    }
    assert!(matches!(
        t.on_session_claim_full_team(),
        Err(EffortGateError::UnderCount {
            s: 3,
            n: 4,
            pursuing: true
        })
    ));
    t.on_session_specialist_outcome(3, SpecialistStatus::Success, Some("t3".into()))
        .unwrap();
    assert!(t.on_session_claim_full_team().is_ok());
    t.mark_synthesis_complete().unwrap();
    assert!(t.synthesis_complete());
}

#[test]
fn heavy_requires_contrarian() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Heavy, false);
    t.on_session_turn_start("research multi-file audit of auth")
        .unwrap();
    for row in t.ledger().to_vec() {
        if !row.is_contrarian {
            t.on_session_specialist_outcome(row.slot, SpecialistStatus::Success, Some("x".into()))
                .unwrap();
        } else {
            t.on_session_specialist_outcome(row.slot, SpecialistStatus::Failed, None)
                .unwrap();
        }
    }
    assert!(t.on_session_claim_full_team().is_err());
    let cslot = t.ledger().iter().find(|r| r.is_contrarian).unwrap().slot;
    t.on_session_specialist_outcome(cslot, SpecialistStatus::Success, Some("c".into()))
        .unwrap();
    assert!(t.on_session_claim_full_team().is_ok());
}

#[test]
fn timeout_and_empty_report_do_not_count_as_success() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    t.begin_team_run().unwrap();
    t.on_session_specialist_outcome(0, SpecialistStatus::Timeout, None)
        .unwrap();
    t.on_session_specialist_outcome(1, SpecialistStatus::EmptyReport, None)
        .unwrap();
    t.on_session_specialist_outcome(2, SpecialistStatus::Failed, None)
        .unwrap();
    t.on_session_specialist_outcome(3, SpecialistStatus::Success, Some("ok".into()))
        .unwrap();
    assert_eq!(t.successful_count(), 1);
    assert!(matches!(
        t.on_session_claim_full_team(),
        Err(EffortGateError::UnderCount { s: 1, n: 4, .. })
    ));
}

#[test]
fn hard_stop_continue_one_wave_per_grant_then_hard_stop_again() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    t.begin_team_run().unwrap();
    t.mark_replace_wave();
    t.mark_replace_wave();
    assert!(t.is_hard_stop());
    t.hard_stop_continue().unwrap();
    assert!(t.hard_stop_continue().is_err()); // no double-grant
    t.begin_continue_wave().unwrap();
    // Still short → hard-stop again; user may continue once more (§4.4).
    assert!(t.is_hard_stop());
    t.hard_stop_continue().unwrap();
    t.begin_continue_wave().unwrap();
    assert!(t.is_hard_stop());
}

#[test]
fn production_join_path_finalizes_and_unlocks_execute() {
    // Mirrors SessionActor: bind spawn → finished → try_finalize → execute ok.
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    t.on_session_turn_start("architect multi-file auth migration")
        .unwrap();
    assert!(matches!(
        t.may_execute_writes(false),
        Err(EffortGateError::ExecuteBeforeSynthesis)
    ));
    for i in 0..4 {
        let tid = format!("subagent-{i}");
        assert_eq!(t.assign_next_pending_task(&tid), Some(i));
        t.on_session_specialist_finished(&tid, SpecialistStatus::Success)
            .unwrap();
    }
    assert!(t.synthesis_complete());
    assert!(t.may_execute_writes(false).is_ok());
    // Under-count cannot claim full team mid-run.
    let mut short = EffortModeTracker::new(tmp());
    short.set_mode(EffortMode::Expert, false);
    short.begin_team_run().unwrap();
    short
        .on_session_specialist_outcome(0, SpecialistStatus::Success, Some("a".into()))
        .unwrap();
    assert!(matches!(
        short.on_session_claim_full_team(),
        Err(EffortGateError::UnderCount { s: 1, n: 4, .. })
    ));
    assert!(!short.synthesis_complete());
}

#[test]
fn session_cancel_aborts_to_partial_s_of_n() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    t.on_session_turn_start("implement multi-file feature")
        .unwrap();
    t.on_session_specialist_outcome(0, SpecialistStatus::Success, Some("a".into()))
        .unwrap();
    // SessionActor::effort_on_user_cancel → on_session_user_cancel.
    let label = t.on_session_user_cancel().expect("partial label");
    assert_eq!(label, "1 of 4");
    assert!(matches!(
        t.on_session_claim_full_team(),
        Err(EffortGateError::PartialNotFullTeam { s: 1, n: 4 })
    ));
    // Sticky mode remains Expert after abort.
    assert_eq!(t.mode(), EffortMode::Expert);
    // Partial synthesis unlocks execute for residual work.
    assert!(t.synthesis_complete());
    assert!(t.may_execute_writes(false).is_ok());
}

// ── Criterion 4: execute ordering + plan matrix ────────────────────────────

#[test]
fn execute_blocked_until_production_finalize() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    // Sticky elevated, no team yet — hard gate still blocks writes.
    assert!(matches!(
        t.may_execute_writes(false),
        Err(EffortGateError::ExecuteBeforeSynthesis)
    ));
    t.on_session_turn_start("architect multi-file migration")
        .unwrap();
    // Mid-team (3/4) still blocked.
    for i in 0..3 {
        t.on_session_specialist_outcome(i, SpecialistStatus::Success, Some(format!("x{i}")))
            .unwrap();
    }
    assert!(matches!(
        t.may_execute_writes(false),
        Err(EffortGateError::ExecuteBeforeSynthesis)
    ));
    // 4th success triggers try_finalize_synthesis on the production path.
    t.on_session_specialist_outcome(3, SpecialistStatus::Success, Some("x3".into()))
        .unwrap();
    assert!(t.synthesis_complete());
    assert!(t.may_execute_writes(false).is_ok());
    let slot = t.register_post_n_implementer("implementer").unwrap();
    assert!(t.ledger()[slot].outside_n);
}

#[test]
fn plan_plus_effort_blocks_execute() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Heavy, false);
    t.begin_team_run().unwrap();
    for i in 0..16 {
        t.on_session_specialist_outcome(i, SpecialistStatus::Success, Some("x".into()))
            .unwrap();
    }
    t.mark_synthesis_complete().unwrap();
    assert!(matches!(
        t.may_execute_writes(true),
        Err(EffortGateError::PlanBlocksExecute)
    ));
}

// ── Criterion 5: Normal idle + no yolo ─────────────────────────────────────

#[test]
fn normal_idle_and_policy_never_implies_yolo() {
    let idle = EffortModeTracker::new(tmp());
    assert_eq!(idle.mode(), EffortMode::Normal);
    assert!(idle.policy_reminder(false).is_none());
    assert!(idle.may_execute_writes(false).is_ok());
    assert!(matches!(
        EffortModeTracker::new(tmp()).begin_team_run(),
        Err(EffortGateError::IdleInNormal)
    ));

    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Heavy, false);
    let p = t.policy_reminder(false).unwrap();
    assert!(p.contains("never enables always-approve"));
    assert!(p.contains("mandatory"));
    assert!(is_trivial_task("fix typo in readme"));
    assert!(!is_trivial_task("architect multi-file migration of auth"));
}

// ── Criterion 6: mandatory team briefs (Expert N=4 / Heavy N=16) ───────────

#[test]
fn mandatory_specialist_briefs_expert_and_heavy() {
    let expert = build_specialist_briefs(EffortMode::Expert, "architect multi-file audit");
    assert_eq!(expert.len(), 4);
    assert!(expert.iter().all(|b| !b.is_contrarian));
    assert!(expert[0].prompt.contains("mandatory"));
    assert!(expert[0].prompt.contains("architect multi-file audit"));

    let heavy = build_specialist_briefs(EffortMode::Heavy, "deep research");
    assert_eq!(heavy.len(), 16);
    assert!(heavy[15].is_contrarian);
    assert!(heavy[15].role.starts_with("contrarian-"));
    assert!(heavy.iter().take(15).all(|b| !b.is_contrarian));

    assert!(build_specialist_briefs(EffortMode::Normal, "x").is_empty());
}

#[test]
fn needs_mandatory_fanout_tracks_unbound_pending_slots() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    assert!(!t.needs_mandatory_fanout());
    t.on_session_turn_start("architect multi-file migration")
        .unwrap();
    assert!(t.needs_mandatory_fanout());
    // Pre-bind all slots as the shell orchestrator does before spawn.
    for i in 0..4 {
        t.record_outcome(
            i,
            SpecialistStatus::Running,
            Some(format!("task-{i}")),
        )
        .unwrap();
    }
    assert!(!t.needs_mandatory_fanout());
    // Full success finalizes and unlocks execute.
    for i in 0..4 {
        t.on_session_specialist_outcome(i, SpecialistStatus::Success, Some(format!("task-{i}")))
            .unwrap();
    }
    assert!(t.synthesis_complete());
    assert!(t.may_execute_writes(false).is_ok());
}

#[test]
fn heavy_mandatory_team_requires_contrarian_success() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Heavy, false);
    t.begin_team_run().unwrap();
    assert!(t.needs_mandatory_fanout());
    assert_eq!(t.ledger().len(), 16);
    assert!(t.ledger()[15].is_contrarian);

    // 16 successes without marking contrarian on last would still work because
    // begin_team_run already set is_contrarian on slot 15.
    for i in 0..16 {
        t.on_session_specialist_outcome(i, SpecialistStatus::Success, Some(format!("h{i}")))
            .unwrap();
    }
    assert!(t.synthesis_complete());
    assert!(t.may_execute_writes(false).is_ok());

    let briefs = build_specialist_briefs(EffortMode::Heavy, "task");
    let reports: Vec<_> = briefs
        .into_iter()
        .enumerate()
        .map(|(i, b)| (b, format!("body-{i}")))
        .collect();
    let pkg = format_team_report_package(EffortMode::Heavy, "16 of 16", &reports);
    assert!(pkg.contains("mandatory team complete"));
    assert!(pkg.contains("contrarian"));
    assert!(pkg.contains("body-15"));
}

// ── Criterion 7: full chrome labels (mode + S of N + Partial/Waived) ───────

#[test]
fn effort_chrome_labels_mode_progress_partial_waived_and_normal_clears() {
    // (a) Expert / Heavy sticky chrome names the mode.
    assert_eq!(
        format_effort_chrome_label(EffortChromeState {
            mode: EffortMode::Expert,
            pursuit: PursuitState::Idle,
            successful: 0,
            target_n: Some(4),
            solo_waiver: false,
        })
        .as_deref(),
        Some("Expert")
    );
    assert_eq!(
        format_effort_chrome_label(EffortChromeState {
            mode: EffortMode::Heavy,
            pursuit: PursuitState::Idle,
            successful: 0,
            target_n: Some(16),
            solo_waiver: false,
        })
        .as_deref(),
        Some("Heavy")
    );

    // (b) Normal clears elevated chrome.
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

    // (c) Pursuing ledger with known S and N.
    assert_eq!(
        format_effort_chrome_label(EffortChromeState {
            mode: EffortMode::Expert,
            pursuit: PursuitState::Pursuing,
            successful: 2,
            target_n: Some(4),
            solo_waiver: false,
        })
        .as_deref(),
        Some("Expert 2 of 4")
    );
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Heavy, false);
    t.begin_team_run().unwrap();
    let label = t.chrome_state().status_label().expect("heavy pursuing chrome");
    assert!(label.contains("Heavy"), "{label}");
    assert!(label.contains("0 of 16"), "{label}");
    for i in 0..5 {
        t.record_outcome(i, SpecialistStatus::Success, Some(format!("t{i}")))
            .unwrap();
    }
    let label = t.chrome_state().status_label().expect("progress");
    assert_eq!(label, "Heavy 5 of 16");

    // (d) Partial / Waived labels.
    assert_eq!(
        format_effort_chrome_label(EffortChromeState {
            mode: EffortMode::Expert,
            pursuit: PursuitState::PartialReport,
            successful: 1,
            target_n: Some(4),
            solo_waiver: false,
        })
        .as_deref(),
        Some("Expert Partial 1 of 4")
    );
    assert_eq!(
        format_effort_chrome_label(EffortChromeState {
            mode: EffortMode::Heavy,
            pursuit: PursuitState::Waived,
            successful: 0,
            target_n: Some(16),
            solo_waiver: false,
        })
        .as_deref(),
        Some("Heavy Waived")
    );

    // Wire payload carries the same shipped label (no pager reimplementation).
    let wire = t.chrome_state().to_wire();
    assert_eq!(wire.mode, "heavy");
    assert_eq!(wire.pursuit, "pursuing");
    assert_eq!(wire.label.as_deref(), Some("Heavy 5 of 16"));
    assert_eq!(wire.successful, 5);
    assert_eq!(wire.target_n, Some(16));
}

#[test]
fn effort_chrome_wire_clears_on_normal() {
    let mut t = EffortModeTracker::new(tmp());
    t.set_mode(EffortMode::Expert, false);
    assert!(t.chrome_state().to_wire().label.is_some());
    t.clear_to_normal();
    let wire = t.chrome_state().to_wire();
    assert_eq!(wire.mode, "normal");
    assert!(wire.label.is_none());
}
