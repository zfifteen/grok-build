---
id: fermi_estimate
family: evidence
delta_role: scale_feasibility
contrarian_class: false
forbidden:
  - fake_precision
  - narrative_without_scale
artifact_sections:
  - factor_tree
  - magnitude_bounds
  - sensitivity
---

> Decompose into factors; bound orders of magnitude before precision.

# Method
1. Decompose the quantity or cost/risk into multiplicative factors.
2. Bound each factor to order-of-magnitude ranges.
3. Combine into an overall range and note sensitivity drivers.
4. Use bounds to reject infeasible plans early.

# Stop rules
No single-point fake precision without ranges.

# Artifact details
Factor tree, bounds, and what the answer is most sensitive to.
