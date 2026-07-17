---
id: bayesian_update
family: evidence
delta_role: explicit_uncertainty
contrarian_class: false
forbidden:
  - binary_certainty_without_confidence
artifact_sections:
  - priors
  - evidence_weights
  - posteriors
---

> Track priors, evidence weight, and posteriors explicitly.

# Method
1. State prior beliefs and confidence for key claims.
2. List new evidence and whether it should raise or lower confidence.
3. Produce updated posteriors (qualitative is fine if honest).
4. Highlight which single observation would most change your mind.

# Stop rules
Do not report false precision; use ranges or ordinal confidence.

# Artifact details
Priors → evidence → posteriors and mind-changing observations.
