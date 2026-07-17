---
id: theory_of_constraints
family: systems
delta_role: binding_limit
contrarian_class: false
forbidden:
  - local_throughput_optimization
  - busywork_as_progress
artifact_sections:
  - constraint_id
  - exploit_plan
  - elevate_plan
---

> Find the bottleneck; subordinate other work to elevating it.

# Method
1. Identify the binding constraint on system throughput or progress.
2. Exploit the constraint (remove idle time / waste at the constraint).
3. Subordinate non-constraint work so it does not overload the constraint.
4. Elevate the constraint only after exploit/subordinate are clear.

# Stop rules
Do not propose parallel optimizations that ignore the binding constraint.

# Artifact details
Constraint ID and ordered exploit → subordinate → elevate steps.
