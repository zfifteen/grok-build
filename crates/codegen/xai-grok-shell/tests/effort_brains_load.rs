//! Focused integration tests for effort reasoning brains.

use xai_grok_shell::session::effort_brains::{
    load_effort_brain_config_from_layers, select_brain_ids, EffortBrainError, RosterSelection,
    BRAIN_COUNT_HEAVY, EXPERT_K_DEFAULT, EFFORT_BRAIN_SEED_ENV,
};
use xai_grok_shell::session::effort_mode::{
    build_specialist_briefs, format_team_report_package, EffortMode, EffortModeTracker,
};

#[test]
fn builtins_load_and_validate() {
    let cfg = load_effort_brain_config_from_layers(None, None).expect("builtins");
    assert_eq!(cfg.brain_count(), BRAIN_COUNT_HEAVY);
    match &cfg.heavy {
        RosterSelection::Fixed { slots } => {
            assert_eq!(slots.len(), BRAIN_COUNT_HEAVY);
            assert_eq!(slots.last().unwrap().as_str(), "red_team");
        }
        _ => panic!("heavy fixed"),
    }
    match &cfg.expert {
        RosterSelection::Random { k, pool } => {
            assert_eq!(*k, EXPERT_K_DEFAULT);
            assert_eq!(pool.len(), BRAIN_COUNT_HEAVY);
        }
        _ => panic!("expert random"),
    }
    assert!(cfg.get("red_team").unwrap().contrarian_class);
}

#[test]
fn bad_heavy_roster_rejected() {
    let dir = std::env::temp_dir().join(format!(
        "effort-brains-it-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("rosters")).unwrap();
    std::fs::write(
        dir.join("rosters/heavy.toml"),
        r#"
version = 1
mode = "heavy"
selection = "fixed"
slots = ["first_principles", "red_team"]
"#,
    )
    .unwrap();
    let err = load_effort_brain_config_from_layers(None, Some(&dir)).unwrap_err();
    assert!(
        matches!(
            err,
            EffortBrainError::HeavySlotCount {
                found: 2,
                expected: 16
            }
        ),
        "got {err}"
    );
}

#[test]
fn select_and_build_briefs_use_brains() {
    let cfg = load_effort_brain_config_from_layers(None, None).unwrap();
    let heavy_ids = select_brain_ids(&cfg, EffortMode::Heavy).unwrap();
    assert_eq!(heavy_ids.len(), 16);
    assert_eq!(heavy_ids[15].as_str(), "red_team");

    unsafe { std::env::set_var(EFFORT_BRAIN_SEED_ENV, "99") };
    let a = select_brain_ids(&cfg, EffortMode::Expert).unwrap();
    let b = select_brain_ids(&cfg, EffortMode::Expert).unwrap();
    unsafe { std::env::remove_var(EFFORT_BRAIN_SEED_ENV) };
    assert_eq!(a, b);
    assert_eq!(a.len(), 4);

    let briefs = build_specialist_briefs(EffortMode::Heavy, "deep audit");
    assert_eq!(briefs.len(), 16);
    assert!(briefs[0].prompt.contains("Brain protocol"));
    assert!(briefs[0].prompt.contains("deep audit"));
    assert_eq!(briefs[15].brain_id, "red_team");

    let pkg = format_team_report_package(
        EffortMode::Heavy,
        "16 of 16",
        &briefs
            .into_iter()
            .map(|b| (b, "body".into()))
            .collect::<Vec<_>>(),
    );
    assert!(pkg.contains("Conflicts"));
    assert!(pkg.contains("red_team"));
}

#[test]
fn tracker_begin_locks_brain_roles() {
    let dir = std::env::temp_dir().join(format!(
        "effort-tracker-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let mut t = EffortModeTracker::new(dir);
    t.set_mode(EffortMode::Heavy, false);
    t.begin_team_run().unwrap();
    assert_eq!(t.ledger()[0].role, "first_principles");
    let pending = t.pending_slot_briefs("task");
    assert_eq!(pending.len(), 16);
    assert_eq!(pending[0].brain_id, "first_principles");
}


#[test]
fn chrome_includes_brain_hint() {
    use xai_grok_shell::session::effort_mode::format_effort_chrome_label;
    use xai_grok_shell::session::effort_mode::{EffortChromeState, PursuitState};
    let dir = std::env::temp_dir().join(format!(
        "effort-chrome-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let mut tr = EffortModeTracker::new(dir);
    tr.set_mode(EffortMode::Heavy, false);
    tr.begin_team_run().unwrap();
    let label = tr.chrome_state().status_label().unwrap();
    assert!(label.contains("Heavy 0 of 16"), "{label}");
    assert!(label.contains("first_principles"), "{label}");

    let bare = format_effort_chrome_label(&EffortChromeState {
        mode: EffortMode::Expert,
        pursuit: PursuitState::Pursuing,
        successful: 1,
        target_n: Some(4),
        solo_waiver: false,
        brain_hint: Some("bayesian_update".into()),
    })
    .unwrap();
    assert_eq!(bare, "Expert 1 of 4 · bayesian_update");
}

#[test]
fn seed_export_roundtrip() {
    use xai_grok_shell::session::effort_brains::export_builtin_effort_brains_to;
    let dir = std::env::temp_dir().join(format!(
        "effort-seed-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    export_builtin_effort_brains_to(&dir).unwrap();
    assert!(dir.join("README.md").is_file());
    assert_eq!(std::fs::read_dir(dir.join("brains")).unwrap().count(), 16);
}
