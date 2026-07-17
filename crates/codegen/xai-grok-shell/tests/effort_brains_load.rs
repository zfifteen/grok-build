//! Focused integration tests for effort reasoning brains (PR-A).
//! Avoids compiling the full xai-grok-shell lib-test graph.

use xai_grok_shell::session::effort_brains::{
    load_effort_brain_config_from_layers, EffortBrainError, RosterSelection, BRAIN_COUNT_HEAVY,
    EXPERT_K_DEFAULT,
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
    assert!(cfg.get("inversion").unwrap().contrarian_class);
    assert!(cfg.get("pre_mortem").unwrap().contrarian_class);
    assert!(cfg
        .get("first_principles")
        .unwrap()
        .body_markdown
        .contains("Method"));
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
