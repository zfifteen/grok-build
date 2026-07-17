# 05 — Skills Coexistence and Migration

**Status:** Phase 1 draft  
**Date:** 2026-07-15

---

## 1. Current state (skills-first v1)

| Artifact | Location (portable) | Operator workstation (Velocity Works) |
|----------|---------------------|----------------------------------------|
| Skill packages | User skills dir: `~/.grok/skills/{expert,heavy,normal}/` (or project `.grok/skills/`) | Same layout on principal machine |
| Design / install archive | External research tree **not vendored** in this repo | e.g. `IdeaProjects/research/grok-build-effort-modes/` |
| Install script | `scripts/install-effort-mode-skills.sh` in that research tree | Same |
| Downstream consumers | Project `AGENTS.md` / ops docs that reference `/expert` `/heavy` | e.g. PGS / agency continuity |

Behavior is **policy-enforced** by the model following skill text. Sticky mode is best-effort. No shell mode bit.

---

## 2. Target state (builtins)

| Concern | Owner |
|---------|--------|
| Name resolution `/expert` etc. | Shell `BUILTIN_COMMANDS` |
| Sticky mode | `EffortModeTracker` |
| Hard N gates | Effort runtime (Phase 4) |
| User docs | Product user guide + changelog |

Skills must **not** run a second full fan-out when builtins are active.

---

## 3. Coexistence window

During Phases 2–4:

1. Builtins register and win resolution when both exist.  
2. Leave skill files installed so older builds / other hosts still work.  
3. Update skill frontmatter descriptions to:  
   “Prefer product builtin when available; this skill is fallback policy only.”  
4. Optionally set `disable-model-invocation: true` remains; slash still works on old builds.

---

## 4. Migration steps

### M1 — Before first builtin release

- Document dual path in research README + this package.  
- Agency continuity: builtins program open; skills still production path.

### M2 — Builtin scaffold ships (Phase 2)

- Verify skill slash does not fire for `/expert` on new binary.  
- Smoke: uninstall skills → builtins still work; install skills → builtins still win.

### M3 — Hard runtime ships (Phase 4)

- Skill text becomes optional fallback for forks without builtins.  
- PGS `AGENTS.md`: “`/expert` `/heavy` product builtins when present; skills otherwise.”

### M4 — Deprecation (optional, later)

- Ship skill stubs that only print: mode is a product builtin; run `/expert` on current Power Grok.  
- Or remove global install from install script with major-version note.

---

## 5. Name collision rules

| Client version | `/expert` resolves to |
|----------------|------------------------|
| Pre-builtin CLI | Skill (if installed) |
| Post-builtin CLI | **Builtin always** |
| Post-builtin + no skill | Builtin |
| Foreign harness without builtins | Skill if installed |

---

## 6. What we preserve from skills

Port into product policy / runtime:

- Fixed N = 4 / 16  
- Contrarian ≥1 on Heavy  
- Join-all, replace caps, abort partial  
- Execute outside N  
- Solo waiver + trivial short-circuit  
- Plan orthogonal; no always-approve coupling  
- disable auto-invoke for skill era (builtins are explicit slash)

---

## 7. What skills cannot do (why migrate)

From research Spec 11 — still true of pure skills:

- Shell-owned sticky mode bit / chrome without model turn  
- Hard empty-fanout gates in orchestrator  
- Winning true builtin registry  

Builtins close those gaps.

---

## 8. Rollback (feature flag — normative)

Recommended kill switch: **`effort_mode_builtins`** (name illustrative; wire to product config/env).

### When flag is **on** (ship default after Phase 2+)

- `/expert` `/heavy` `/normal` are registered builtins.  
- Shell `resolve()` matches builtins **before** skill parse (`resolve_builtin_shadows_same_named_skill` pattern).  
- Skills of the same name do **not** run.

### When flag is **off** (rollback)

Turning the flag off is **not** “idle the tracker but leave the three names in `BUILTIN_COMMANDS`.” That would **brick** the skills fallback forever, because builtins always win name resolution.

Flag-off **must**:

1. **Omit** `expert` / `heavy` / `normal` from the effective builtin table used by `resolve` and autocomplete (`available_commands` / `allows` must not advertise them). Prefer compile-time or runtime filtering so the names are absent from the builtin match path entirely.  
2. **Idle** any `EffortModeTracker` / effort runtime so the Normal path is identical to pre-feature builds (no policy inject, no hard gates).  
3. Leave skill packages installed (if present) so slash can fall through to **skills** again on hosts that still use skills-first.

Only then does “skills return for slash” hold. Document this in code comments next to the three `BuiltinCommand` entries when implemented.
