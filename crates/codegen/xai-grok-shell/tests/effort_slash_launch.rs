//! Launch check: real resolve path for effort slash builtins (verification
//! plan step 4). Captures criterion-1 behavior without reimplementing resolve.

use xai_grok_shell::session::effort_mode::EffortMode;
use xai_grok_shell::session::{EffortSlashResolve, resolve_effort_slash};

#[test]
fn launch_expert_empty_and_with_task() {
    assert_eq!(
        resolve_effort_slash("/expert", true, &[]),
        EffortSlashResolve::Builtin {
            mode: EffortMode::Expert,
            task: None,
            solo: false,
        }
    );
    assert_eq!(
        resolve_effort_slash("/expert fix CI", true, &[]),
        EffortSlashResolve::Builtin {
            mode: EffortMode::Expert,
            task: Some("fix CI".into()),
            solo: false,
        }
    );
}

#[test]
fn launch_normal_clears_via_resolve() {
    assert_eq!(
        resolve_effort_slash("/normal ignored-args", true, &[]),
        EffortSlashResolve::Builtin {
            mode: EffortMode::Normal,
            task: None,
            solo: false,
        }
    );
}

#[test]
fn launch_flag_off_skill_reclaim() {
    assert_eq!(
        resolve_effort_slash("/heavy", false, &["heavy"]),
        EffortSlashResolve::Skill {
            name: "heavy".into()
        }
    );
    assert_eq!(
        resolve_effort_slash("/heavy", false, &[]),
        EffortSlashResolve::PassThrough
    );
}

#[test]
fn launch_skill_collision_builtin_wins_when_enabled() {
    for name in ["expert", "heavy", "normal"] {
        let prompt = format!("/{name}");
        match resolve_effort_slash(&prompt, true, &[name]) {
            EffortSlashResolve::Builtin { mode, .. } => {
                assert_eq!(mode.as_str(), name);
            }
            other => panic!("expected Builtin for /{name}, got {other:?}"),
        }
    }
}
