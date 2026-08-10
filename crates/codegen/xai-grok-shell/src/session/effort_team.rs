//! Shell-owned mandatory Expert/Heavy team orchestration helpers.
//!
//! Pure spawn/join utilities used by `SessionActor`. Live subagent dispatch
//! goes through the same `SubagentEvent` channel as goal planner/classifier
//! harness spawns — **not** the model tool path.

use xai_grok_tools::implementations::grok_build::task::backend::{ChannelBackend, SubagentBackend};
use xai_grok_tools::implementations::grok_build::task::types::{
    SubagentEvent, SubagentOwner, SubagentRequest, SubagentResult, SubagentRuntimeOverrides,
};
use xai_tool_types::SubagentCapabilityMode;

use super::effort_mode::{EffortMode, SpecialistBrief, build_specialist_briefs};

/// Default analytic subagent type for effort-mode specialists.
pub const EFFORT_SPECIALIST_SUBAGENT_TYPE: &str = "explore";

/// One completed specialist report (or failure text) after join.
#[derive(Debug, Clone)]
pub struct SpecialistJoinResult {
    pub brief: SpecialistBrief,
    pub task_id: String,
    pub success: bool,
    pub cancelled: bool,
    pub body: String,
}

/// Spawn one specialist on the coordinator channel and await its result.
///
/// `task_id` must already be bound to `brief.slot` on the effort ledger so
/// join outcomes map 1:1 without racing `SubagentSpawned` bind order.
pub async fn spawn_specialist(
    event_tx: &tokio::sync::mpsc::UnboundedSender<SubagentEvent>,
    parent_session_id: &str,
    parent_prompt_id: Option<String>,
    cwd: Option<String>,
    task_id: String,
    brief: SpecialistBrief,
) -> SpecialistJoinResult {
    let request = SubagentRequest {
        id: task_id.clone(),
        prompt: brief.prompt.clone(),
        description: brief.description.clone(),
        subagent_type: EFFORT_SPECIALIST_SUBAGENT_TYPE.to_string(),
        parent_session_id: parent_session_id.to_string(),
        parent_prompt_id,
        resume_from: None,
        cwd,
        runtime_overrides: SubagentRuntimeOverrides {
            // Analytic-only fixed team: read/search, no writes.
            capability_mode: Some(SubagentCapabilityMode::ReadOnly),
            model: brief.model_override.clone(),
            model_override_provenance:
                xai_grok_tools::implementations::grok_build::task::types::ModelOverrideProvenance::Harness,
            ..Default::default()
        },
        run_in_background: false,
        surface_completion: true,
        // Block until the specialist finishes (no short foreground budget).
        await_to_completion: true,
        fork_context: false,
        owner: SubagentOwner::Task,
        cancel_token: tokio_util::sync::CancellationToken::new(),
    };
    let backend = ChannelBackend::new(event_tx.clone());
    match backend.spawn_with_foreground_wait(request, None).await {
        Ok(result) => join_from_result(brief, task_id, result),
        Err(err) => SpecialistJoinResult {
            brief,
            task_id,
            success: false,
            cancelled: false,
            body: format!("effort team: subagent spawn failed: {err}"),
        },
    }
}

fn join_from_result(
    brief: SpecialistBrief,
    task_id: String,
    result: SubagentResult,
) -> SpecialistJoinResult {
    let body = if result.success {
        result.output.to_string()
    } else {
        result
            .error
            .unwrap_or_else(|| "specialist failed without error text".to_string())
    };
    // Preserve the raw subagent success flag; empty-body demotion to
    // EmptyReport happens in `specialist_status_from_join` (tech-spec §4.1).
    SpecialistJoinResult {
        brief,
        task_id,
        success: result.success,
        cancelled: result.cancelled,
        body,
    }
}

/// Planned specialist spawn: fixed `task_id` for ledger slot + brief.
#[derive(Debug, Clone)]
pub struct PlannedSpecialist {
    pub task_id: String,
    pub brief: SpecialistBrief,
}

/// Build N planned specialists with fresh UUIDs (call before ledger bind + spawn).
pub fn plan_mandatory_team(mode: EffortMode, task_text: &str) -> Vec<PlannedSpecialist> {
    build_specialist_briefs(mode, task_text)
        .into_iter()
        .map(|brief| PlannedSpecialist {
            task_id: uuid::Uuid::now_v7().to_string(),
            brief,
        })
        .collect()
}

/// Plan spawns for already-Pending unbound slots (replace / recovery waves).
pub fn plan_pending_slots(briefs: Vec<SpecialistBrief>) -> Vec<PlannedSpecialist> {
    briefs
        .into_iter()
        .map(|brief| PlannedSpecialist {
            task_id: uuid::Uuid::now_v7().to_string(),
            brief,
        })
        .collect()
}

/// Spawn planned specialists in parallel and join-all.
pub async fn run_planned_team(
    event_tx: &tokio::sync::mpsc::UnboundedSender<SubagentEvent>,
    parent_session_id: &str,
    parent_prompt_id: Option<String>,
    cwd: Option<String>,
    planned: Vec<PlannedSpecialist>,
) -> Vec<SpecialistJoinResult> {
    if planned.is_empty() {
        return Vec::new();
    }
    let futs: Vec<_> = planned
        .into_iter()
        .map(|p| {
            let tx = event_tx.clone();
            let parent = parent_session_id.to_string();
            let prompt_id = parent_prompt_id.clone();
            let cwd = cwd.clone();
            async move {
                spawn_specialist(
                    &tx,
                    &parent,
                    prompt_id,
                    cwd,
                    p.task_id,
                    p.brief,
                )
                .await
            }
        })
        .collect();
    futures::future::join_all(futs).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::effort_mode::EffortMode;

    #[test]
    fn build_via_run_helpers_sizes() {
        assert_eq!(
            build_specialist_briefs(EffortMode::Expert, "t").len(),
            4
        );
        assert_eq!(build_specialist_briefs(EffortMode::Heavy, "t").len(), 16);
    }

    #[test]
    fn join_from_result_maps_success_and_failure() {
        let brief = build_specialist_briefs(EffortMode::Expert, "t")
            .into_iter()
            .next()
            .unwrap();
        let ok = join_from_result(
            brief.clone(),
            "id-1".into(),
            SubagentResult {
                success: true,
                output: std::sync::Arc::from("hello"),
                ..Default::default()
            },
        );
        assert!(ok.success);
        assert_eq!(ok.body, "hello");

        let err = join_from_result(
            brief.clone(),
            "id-2".into(),
            SubagentResult {
                success: false,
                error: Some("boom".into()),
                cancelled: true,
                ..Default::default()
            },
        );
        assert!(!err.success);
        assert!(err.cancelled);
        assert_eq!(err.body, "boom");

        // Empty / thin bodies demote to EmptyReport via status mapper.
        use crate::session::effort_mode::EffortModeTracker;
        use crate::session::effort_mode::SpecialistStatus;
        let empty = join_from_result(
            brief.clone(),
            "id-3".into(),
            SubagentResult {
                success: true,
                output: std::sync::Arc::from("   "),
                ..Default::default()
            },
        );
        assert!(empty.success); // raw subagent flag
        assert_eq!(
            EffortModeTracker::specialist_status_from_join(
                empty.success,
                empty.cancelled,
                &empty.body
            ),
            SpecialistStatus::EmptyReport
        );

        let thin_ok = join_from_result(
            brief,
            "id-4".into(),
            SubagentResult {
                success: true,
                output: std::sync::Arc::from("ok"),
                ..Default::default()
            },
        );
        assert_eq!(
            EffortModeTracker::specialist_status_from_join(
                thin_ok.success,
                thin_ok.cancelled,
                &thin_ok.body
            ),
            SpecialistStatus::EmptyReport
        );
    }
}
