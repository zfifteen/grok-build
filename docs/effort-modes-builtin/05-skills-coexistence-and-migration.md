# 05 — Skills Coexistence and Migration

**Status:** Phase 1 draft  
**Date:** 2026-07-15

---

## 1. Current state (skills-first v1)

| Artifact | Location |
|----------|----------|
| Skill packages | `~/.grok/skills/{expert,heavy,normal}/SKILL.md` |
| Design / install | `~/IdeaProjects/research/grok-build-effort-modes/` |
| Install script | `research/.../scripts/install-effort-mode-skills.sh` |
| PGS / agency references | `prime-gap-structure/AGENTS.md`, agency STATE |

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

- Ship skill stubs that only print: mode is a product builtin; run `/expert` on current Grok Build.  
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

## 8. Rollback

If builtins regress:

1. Feature-flag effort builtins off (recommended kill switch).  
2. Skills remain installed → previous policy behavior returns for slash.  
3. Keep tracker behind flag so Normal path is identical to today.
