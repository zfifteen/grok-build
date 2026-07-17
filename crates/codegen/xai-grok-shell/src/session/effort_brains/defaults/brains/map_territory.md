---
id: map_territory
family: foundations
delta_role: expose_representation_error
contrarian_class: false
forbidden:
  - docs_as_ground_truth
  - type_system_as_proof_of_behavior
artifact_sections:
  - representation_gaps
  - runtime_checks
  - stale_assumptions
---

> Separate models, docs, and types from runtime reality; hunt map errors.

# Method
1. Identify every map in play (docs, types, diagrams, mental models).
2. Ask where each map can diverge from the territory (code/runtime/users).
3. Prefer evidence from the system as it runs over descriptions of intent.
4. Flag stale or aspirational documentation as risk.

# Stop rules
Stop when major maps are classified as verified, unverified, or known-wrong.

# Artifact details
List representation gaps with how to verify each against territory.
