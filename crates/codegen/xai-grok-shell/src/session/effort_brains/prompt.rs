//! Render specialist prompts from brain protocols.

use super::types::BrainSpec;
use crate::session::effort_mode::EffortMode;

/// Build the analytic specialist prompt for one brain slot.
pub fn render_specialist_prompt(
    mode: EffortMode,
    slot: usize,
    n: usize,
    task_text: &str,
    spec: &BrainSpec,
    replacement: bool,
) -> String {
    let mode_name = mode.as_str();
    let task = task_text.trim();
    let task = if task.is_empty() {
        "(no task text — analyze the current session context and codebase)"
    } else {
        task
    };
    let forbidden = if spec.forbidden.is_empty() {
        "(none listed)".to_string()
    } else {
        spec.forbidden
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let artifacts = {
        let mut parts = vec![
            "brain_id".into(),
            "method_applied".into(),
            "claims".into(),
            "evidence".into(),
            "uncertainties".into(),
            "disagreements_invited".into(),
            "verdict".into(),
        ];
        for a in &spec.artifact_sections {
            if !parts.iter().any(|p| p == a) {
                parts.push(a.clone());
            }
        }
        parts
            .into_iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let launch = if replacement {
        "The shell launched you as a replacement / recovery spawn; do not spawn further subagents."
    } else {
        "The shell launched you; do not spawn further subagents."
    };
    let contrarian = if spec.contrarian_class { "yes" } else { "no" };

    format!(
        "You are specialist slot {slot} of {n} on a **mandatory** {mode_name} effort-mode \
         analytic team (join-all). {launch}\n\
         \n\
         **Brain id:** {id}\n\
         **Family:** {family}\n\
         **Delta role:** {delta}\n\
         **Contrarian class:** {contrarian}\n\
         \n\
         **Forbidden moves:**\n{forbidden}\n\
         \n\
         **Required artifact sections:**\n{artifacts}\n\
         \n\
         # Brain protocol\n\
         {body}\n\
         \n\
         # User task\n\
         {task}\n\
         \n\
         Stay analytic and non-writing (read/search only). Produce the required artifact. \
         End with a short summary the lead agent can synthesize. Do not cosplay a character — \
         apply the reasoning protocol only.",
        id = spec.id.as_str(),
        family = spec.family,
        delta = spec.delta_role,
        body = spec.body_markdown.trim(),
    )
}

/// Short description for spawn UI / ledger chrome.
pub fn brain_description(mode: EffortMode, slot: usize, n: usize, spec: &BrainSpec) -> String {
    let mode_name = mode.as_str();
    if spec.contrarian_class {
        format!(
            "{mode_name} contrarian brain {} ({}/{})",
            spec.id.as_str(),
            slot + 1,
            n
        )
    } else {
        format!(
            "{mode_name} brain {} ({}/{})",
            spec.id.as_str(),
            slot + 1,
            n
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::effort_brains::load_effort_brain_config_from_layers;

    #[test]
    fn prompt_contains_protocol_and_task() {
        let cfg = load_effort_brain_config_from_layers(None, None).unwrap();
        let spec = cfg.get("red_team").unwrap();
        let p = render_specialist_prompt(EffortMode::Heavy, 15, 16, "audit auth", spec, false);
        assert!(p.contains("red_team"));
        assert!(p.contains("audit auth"));
        assert!(p.contains("Brain protocol"));
        assert!(p.contains("non-writing"));
        assert!(p.contains("Contrarian class:** yes"));
        assert!(!p.contains("You are a famous"));
    }
}
