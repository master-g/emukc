---
title: "chore: Sunset openspec — Migrate to ce-compound-engineering"
status: completed
type: chore
date: 2026-06-22
---

# chore: Sunset openspec — Migrate to ce-compound-engineering

## Summary

Retire the `openspec/` spec-driven governance system and migrate its three
distinct roles onto the `ce-compound-engineering` toolkit that this repo
already uses. This is a **governance + documentation migration** — it touches
no gameplay/runtime source and changes no behavioral contract, only *where and
how* those contracts are recorded and how cross-crate changes are gated.

openspec is three systems in one; ce-compound-engineering is a toolkit. The
migration is therefore an **role split**, not a 1:1 rename:

| openspec role | ce-compound-engineering target | State today |
| --- | --- | --- |
| Forward proposals (`changes/` proposal+design+tasks) | `ce-plan` → `docs/plans/` | Already in use (47 archived + 4 active) |
| Retrospective knowledge (archived change lessons) | `ce-compound` → `docs/solutions/[category]/` | Just started (3 docs) |
| Knowledge freshness | `ce-compound-refresh` | Available |
| Code review gate | `ce-code-review` / `/review` | Already in use |
| **Living behavioral contracts (`specs/`, 37 WHEN/THEN capabilities)** | **No direct ce equivalent** → migrate to `docs/solutions/` knowledge track | **This plan's main work** |
| Process gate (CLAUDE.md mandates propose→archive) | Rewrite CLAUDE.md / PROJECT_MEMORY.md | This plan |

**Scope-honesty note.** ce-compound-engineering has no analogue to openspec's
continuously-maintained `specs/` capability contracts. Per the recorded
decision (see Decisions §D1), the still-valuable specs are demoted from
*living contracts* to *captured knowledge* under `docs/solutions/`. After this
migration there is **no machine-enforced behavioral contract layer**;
behavioral drift is caught by tests + `ce-code-review`, not by a spec gate.
This is an accepted consequence, not a gap to fill later.

## Problem Frame

The project has outgrown openspec:

1. **Tooling duplication.** openspec ships its own propose/apply/archive/explore
   flow (`opsx-*` commands + openspec skills under `.claude/`, `.codex/`,
   `.github/`, `.opencode/`) that overlaps with the ce-* skills already
   installed and actually used. Two governance vocabularies coexist.
2. **specs/ vs solutions/ tension.** `openspec/specs/` (37 capabilities,
   WHEN/THEN) and `docs/solutions/` (ce-compound, 3 docs) describe the same
   system from two angles with no bridge. New lessons land in `docs/solutions/`;
   the specs that motivated them go stale silently.
3. **Process gate friction.** CLAUDE.md (lines 177, 198, 216, 221) routes
   cross-crate / spec changes through `/openspec-propose` → implement →
   `/openspec-archive-change`. In practice the team already drafts
   `docs/plans/` for forward work and captures lessons via ce-compound; the
   openspec gate is a second, redundant path.
4. **In-flight debt.** Four un-archived openspec changes
   (`fix-battle-attack-system`, `fix-sortie-state-and-routing`,
   `harden-battle-refactor-followup`, `fix-batch-craft-quest-progress`) sit
   with all tasks unchecked (see PROJECT_MEMORY §Next Session). They need a
   home that is not openspec.
5. **Hard dependency in a sibling plan.** `2026-06-15-002-feat-battle-map-
   client-sync-loop-plan.md` work unit U4 ("drift report → openspec change
   scaffold") writes `proposal.md`/`tasks.md` into `openspec/changes/`. That
   plan cannot ship unchanged once openspec is removed.

## Decisions (recorded this session)

- **D1 — Spec contract destination: migrate to `docs/solutions/` knowledge
  track.** Each of the 37 `openspec/specs/<cap>/spec.md` is triaged: still-
  valuable ones become `docs/solutions/architecture-patterns/` or
  `conventions/` docs (ce-compound knowledge-track template); obsolete ones are
  dropped with openspec. **Accepted trade-off:** living contracts become
  historical knowledge; no machine enforcement of behavioral invariants
  remains.
- **D2 — In-flight changes: translate into `docs/plans/` plans.** Each of the
  four un-archived openspec changes is converted into one ce-plan-format plan
  under `docs/plans/`, merging its `proposal.md` + `design.md` + `tasks.md`
  into the single-plan layout this repo already uses. The openspec originals
  are removed in U5.
- **D3 — Session scope: this plan document only.** No source, no openspec
  asset, and no CLAUDE.md rewrite is performed in the planning session. Execution
  follows the Implementation Units below in later sessions, each as its own
  commit.

## Requirements

- **R1.** Zero behavioral change. `cargo build`, `cargo clippy --workspace`,
  `cargo test`, and `cargo test --test gameplay_tests` stay green at every
  commit boundary. The battle golden transcript is not re-frozen.
- **R2.** No knowledge loss without an explicit decision. Every `openspec/`
  artifact is either migrated to a ce-compound-engineering location or
  explicitly dropped with a one-line rationale recorded in the migration log
  (U6).
- **R3.** Single governance vocabulary after migration. `grep -rin openspec`
  across the repo (excluding `docs/plans/archive/`, git history, and this plan)
  returns nothing.
- **R4.** CLAUDE.md process gate rewritten so cross-crate / behavioral changes
  route through `ce-plan` → implement → `ce-compound` (capture) +
  `ce-code-review`, with no openspec reference.
- **R5.** The sibling client-sync plan (`2026-06-15-002`) is updated so U4 no
  longer writes into `openspec/changes/`; its scaffold target is repointed to
  `docs/plans/` (ce-plan naming).
- **R6.** Each Implementation Unit is its own Conventional Commit; no AI
  attribution; soft tabs; the Balance Defaults Policy is untouched (no
  `feat(balance)` triggers).
- **R7.** Migration is reversible up to U5 (deletion). U1–U4 only add files
  and edit docs; U5 is the irreversible `git rm`.

## Key Technical Decisions

### KTD1. openspec → ce role mapping (authoritative)

```
openspec/specs/<cap>/spec.md            → docs/solutions/{architecture-patterns|conventions|best-practices}/<cap>-*.md   (D1 triage)
openspec/changes/<active>/proposal.md   → folded into docs/plans/<date>-<verb>-<subject>-plan.md §Summary/Problem/Requirements   (D2)
openspec/changes/<active>/design.md     → folded into the same plan §Key Technical Decisions / High-Level Design
openspec/changes/<active>/tasks.md      → folded into the same plan §Implementation Units (checkboxes preserved)
openspec/changes/<active>/specs/ deltas → folded into the plan as §Behavioral notes; the spec itself follows D1
openspec/changes/archive/**             → dropped (history preserved in git; PROJECT_MEMORY points to it)
openspec/config.yaml                    → dropped (rules retired; equivalent guidance moves into CLAUDE.md §Process gates)
opsx-* commands / openspec skills       → dropped (ce-* skills replace them)
```

### KTD2. Spec triage rule (for U1)

A spec is **kept-and-migrated** if it documents an invariant that is *not*
already enforced by a passing test or by the type system, **and** the
capability is still live in the codebase. Otherwise it is **dropped** with a
logged one-liner. Destination category:

- Core capability contract (sortie, material, quest, fleet, map, battle,
  rng) → `architecture-patterns/`.
- Cross-cutting convention (balance policy, audit config, test/example
  layout, naming) → `conventions/`.
- Infra/tooling knowledge (bootstrap, cache, decoder, progress logging,
  populate) → `best-practices/` (or dropped if already covered by an existing
  `docs/solutions/` doc — prefer consolidation).

Triage outcome table (working draft; finalized during U1 execution):

| Spec | Draft fate | Destination |
| --- | --- | --- |
| sortie, material, quest, fleet, map-unlock, map-data-authority, user-lifecycle, useitem-response, equipment-improvement-bonus, night-battle-sinking-protection | keep | `architecture-patterns/` |
| battle-damage-foundation, battle-kouku-stage3, battle-sim-params, battle-crate-docs, rng-facade | keep | `architecture-patterns/` (battle cluster) |
| balance-defaults-policy, audit-config, test-example-layout, rules-default-strategy | keep | `conventions/` |
| bootstrap-guide, cli-progress, progress-logging-helper, populate-error-classification, material-concurrency | keep | `best-practices/` |
| cache-list-dedup, cache-make-list-versioning, cache-manifest-integration, decoder-*(6), pathrules-* (2), manifest-damage-variants, resource-manifest, web-asset-bootstrap | consolidate or drop | `best-practices/` if not already in `docs/solutions/`; else drop (log reason) |

### KTD3. In-flight translation mapping (for U2)

Each openspec change becomes one plan. The translation preserves every
unchecked task as a checkbox so no work is silently dropped.

| openspec change | New plan slug (draft) |
| --- | --- |
| `fix-battle-attack-system` | `docs/plans/2026-06-22-002-fix-battle-attack-system-plan.md` |
| `fix-sortie-state-and-routing` | `docs/plans/2026-06-22-003-fix-sortie-state-and-routing-plan.md` |
| `harden-battle-refactor-followup` | `docs/plans/2026-06-22-004-refactor-battle-rng-and-practice-store-plan.md` |
| `fix-batch-craft-quest-progress` | `docs/plans/2026-06-22-005-fix-batch-craft-quest-progress-plan.md` |

Slug numbering keeps this migration plan at `001` and stacks the translated
plans at `002`–`005` in the same date bucket.

### KTD4. Repointing client-sync U4 (for U3)

`2026-06-15-002` U4 currently emits `openspec/changes/<slug>/{proposal.md,
tasks.md, .openspec.yaml}`. After migration the same drift scaffold becomes a
ce-plan stub at `docs/plans/<date>-sync-battle-protocol-<version>-plan.md`
populated with Summary/Requirements/Implementation-Units skeletons. KTD6 in
that plan ("routes through openspec, the repo's existing governance") is
rewritten to route through ce-plan. The `--scaffold` flag and verification
shape are unchanged; only the output directory and artifact names change.

### KTD5. CLAUDE.md gate rewrite (for U4)

Line 221's mandatory openspec flow becomes: *"New gameplay behavior, spec
changes, or cross-crate contracts go through `ce-plan` first (draft a
`docs/plans/` plan) → implement against its Implementation Units → capture
lessons with `ce-compound` into `docs/solutions/` → verify with
`ce-code-review`."* Line 198's do-not-modify `openspec/specs/**` entry is
removed (the directory no longer exists). Lines 177/216 balance-policy
references to "openspec proposal" become "ce-plan plan + regression test".
PROJECT_MEMORY.md's openspec references and the `config.yaml:70` typo note are
removed (the typo is deleted with the file).

### KTD6. Commit hygiene

One Conventional Commit per unit. Suggested prefixes: `docs(migration)` for
U1–U4 document work, `chore(openspec)` for the U5 removal, `docs(plans)` for
U3. No balance numerics change, so the Balance Defaults Policy never triggers.

## High-Level Technical Design

Six Implementation Units, executed in order. U1–U4 are additive (reversible);
U5 is the irreversible removal; U6 is verification. Each unit ends green.

```
U1 spec triage + migration ──┐
                             ├─ U2 in-flight translation (4 plans)
                             ├─ U3 repoint client-sync U4
                             ├─ U4 rewrite CLAUDE.md + PROJECT_MEMORY.md
U5 remove openspec/ + tooling ─┘  (only after U1–U4 land)
U6 verification sweep
```

## Implementation Units

### U1. Spec contract triage and migration

- **Goal:** Convert the 37 `openspec/specs/<cap>/spec.md` files into
  ce-compound knowledge-track docs (or drop with a logged reason), per KTD2.
- **Files:**
  - Read: every `openspec/specs/*/spec.md`.
  - Write: `docs/solutions/{architecture-patterns,conventions,best-practices}/<cap>-*.md`
    using the ce-compound knowledge-track template
    (`~/.pi/agent/skills/ce-compound/assets/resolution-template.md`).
  - Append: `docs/migration/openspec-sunset-log.md` (new) — one line per spec:
    `<cap> → <destination> | dropped: <reason>`.
- **Approach:** For each spec, run the KTD2 triage rule. When migrating,
  rewrite WHEN/THEN scenarios as "Guidance" prose with key examples; carry the
  capability name into `tags`. When the spec's content is already covered by
  an existing `docs/solutions/` doc, consolidate instead of duplicating (and
  note consolidation in the log). Do not invent new contracts — migrate what
  exists, nothing more.
- **Verification:** `find docs/solutions -type f | wc -l` increased by the
  kept-spec count; the sunset log has 37 lines; every kept doc parses with
  valid ce-compound YAML frontmatter.
- **Commit:** `docs(migration): migrate openspec specs to docs/solutions knowledge track`

### U2. Translate the four in-flight openspec changes into plans

- **Goal:** Per D2/KTD3, produce four `docs/plans/` plans that fully absorb
  each openspec change's proposal/design/tasks so the originals can be deleted
  in U5 without loss.
- **Files:**
  - Read: `openspec/changes/{fix-battle-attack-system,fix-sortie-state-and-routing,
    harden-battle-refactor-followup,fix-batch-craft-quest-progress}/{proposal,design,tasks}.md`
    and each `specs/` delta.
  - Write: the four plan files listed in KTD3.
- **Approach:** Map proposal → §Summary/Problem Frame/Requirements/Non-goals;
  design → §Key Technical Decisions/High-Level Design (cite exact crate paths
  the openspec design already names); tasks → §Implementation Units preserving
  every `[ ]` checkbox and its crate targets; specs delta → §Behavioral notes
  pointing at the U1 destination doc. Numbering `002`–`005` per KTD3.
- **Verification:** Each new plan has non-empty Summary, Requirements, and
  Implementation Units; checkbox count equals the source `tasks.md` count
  (none silently dropped). Diff the two to confirm parity.
- **Commit:** `docs(plans): translate in-flight openspec changes into ce-plan plans`

### U3. Repoint the client-sync plan's openspec dependency

- **Goal:** Per R5/KTD4, make `2026-06-15-002` buildable without openspec.
- **Files:**
  - Edit: `docs/plans/2026-06-15-002-feat-battle-map-client-sync-loop-plan.md`
    (U4 section, KTD6, the `--scaffold` output-path bullets, the references
    list, and the mermaid `U4` node label).
- **Approach:** Replace `openspec/changes/<slug>/` output targets with
  `docs/plans/<date>-sync-battle-protocol-<version>-plan.md` (ce-plan stub).
  Rewrite KTD6 to cite ce-plan as the governance path. Keep the `--scaffold`
  flag semantics and the verification shape; only the destination and artifact
  names change.
- **Verification:** `grep -in openspec docs/plans/2026-06-15-002-*.md` returns
  nothing; the plan still reads coherently end-to-end.
- **Commit:** `docs(plans): repoint client-sync U4 scaffold from openspec to ce-plan`

### U4. Rewrite the process gates in CLAUDE.md and PROJECT_MEMORY.md

- **Goal:** Per R4/KTD5, remove the mandatory openspec flow and point at the
  ce-compound-engineering flow instead.
- **Files:**
  - Edit: `CLAUDE.md` (lines ~177, 198, 216, 221) and `PROJECT_MEMORY.md`
    (balance-policy note, `config.yaml:70` typo note, §Next Session list of
    four openspec changes → repoint to the U2 plan slugs).
  - Optional: add a one-paragraph "Governance" note to CLAUDE.md explaining
    the ce-plan → ce-compound → ce-code-review loop, so the retired openspec
    flow is not silently un-replaced.
- **Approach:** Surgical edits only (per repo's "no opportunistic refactor"
    rule). Replace, do not delete, the gate wording so future cross-crate
    changes still have a mandated path.
- **Verification:** `grep -in openspec CLAUDE.md PROJECT_MEMORY.md` returns
  nothing; the balance-policy and process-gate sections still make sense.
- **Commit:** `docs(claude): replace openspec process gate with ce-compound-engineering flow`

### U5. Remove openspec assets and tooling scaffolding

- **Goal:** Per R3/R7, delete the now-orphaned openspec tree and its agent
  scaffolding. **Irreversible** — run only after U1–U4 land and review.
- **Files (delete):**
  - `openspec/` (entire tree: `config.yaml`, `specs/`, `changes/` incl.
    `archive/`).
  - `.claude/commands/opsx/{apply,archive,explore,propose}.md`
  - `.claude/skills/openspec-{apply-change,archive-change,explore,propose}/`
  - `.codex/skills/openspec-{apply-change,archive-change,explore,propose}/`
  - `.github/prompts/opsx-{apply,archive,explore,propose}.prompt.md`
  - `.github/skills/openspec-{apply-change,archive-change,explore,propose}/`
  - `.opencode/command/opsx-{apply,archive,explore,propose}.md`
- **Approach:** `git rm -r` each path above. Confirm Makefile and the
  non-skill part of `.github/` have no openspec references (verified clean in
  this planning session). Keep `docs/plans/archive/` untouched (historical
  plans that mention openspec stay as-is — they are frozen history).
- **Verification:** `git status` shows only deletions; `find . -path ./.git
  -prune -o -iname '*openspec*' -print -o -iname '*opsx*' -print` returns
  nothing outside `docs/plans/archive/`.
- **Commit:** `chore(openspec): remove openspec tree and agent scaffolding (migrated to ce-compound-engineering)`

### U6. Verification sweep

- **Goal:** Per R1/R3, prove the migration is complete and the tree is healthy.
- **Commands:**
  - `cargo fmt --all --check`
  - `cargo clippy --workspace -- -W warnings`
  - `cargo test` (plus `--test gameplay_tests` and crate-level subsets per CLAUDE.md)
  - `grep -rin openspec . --exclude-dir=.git --exclude-dir=docs/plans/archive | grep -v 'docs/plans/2026-06-22-001'` → expect empty
- **Approach:** Run the gates locally (no CI server per CLAUDE.md). Any
  remaining openspec mention outside the allow-list is either a missed edit
  (fix) or a legitimate historical reference (annotate in this plan's
  References).
- **Commit:** none (verification-only). If fixes are needed, fold them into
  the offending unit's commit via amend before review.

## Acceptance / Done

The migration is complete when **all** hold:

- A1. U1–U6 each landed as its own commit; working tree clean.
- A2. `cargo fmt --check`, `cargo clippy --workspace -- -W warnings`,
  `cargo test`, `cargo test --test gameplay_tests` all green.
- A3. `grep -rin openspec` (per U6 scope) returns empty.
- A4. `docs/migration/openspec-sunset-log.md` lists all 37 specs with fate.
- A5. Four translated plans exist under `docs/plans/` with task-parity against
  their openspec source (checkbox counts match).
- A6. CLAUDE.md process gate names the ce-plan → ce-compound → ce-code-review
  loop with no openspec reference.

## Risks

- **Behavioral contract erosion (D1 trade-off).** Demoting 37 living specs to
  historical knowledge removes the only machine-checkable behavioral-contract
  layer. *Mitigation:* U1 keeps the high-value contracts as
  `architecture-patterns/` docs and the team relies on tests + ce-code-review;
  if drift becomes a problem later, a lightweight `docs/specs/` can be
  re-introduced without re-adopting openspec.
- **Lost in-flight context.** Translating 4 changes could drop nuance from
  `design.md`. *Mitigation:* U2 verification requires checkbox parity and a
  diff pass; the openspec originals remain in git history after U5.
- **Client-sync plan breakage.** U4 of `2026-06-15-002` is not yet implemented;
  repointing its scaffold target (U3) must land before anyone implements it.
  *Mitigation:* U3 runs before U5; the plan is edited, not the code.
- **Missed references.** A stale openspec mention could survive in a plan or
  doc not surveyed here. *Mitigation:* U6's repo-wide grep is the final gate;
  `docs/plans/archive/` is explicitly allow-listed as frozen history.

## File Inventory (precise)

**Migrate (U1):** 37 `openspec/specs/*/spec.md` → subset kept in
`docs/solutions/{architecture-patterns,conventions,best-practices}/`.

**Translate (U2):** 4 `openspec/changes/<active>/` → 4 `docs/plans/` plans.

**Edit (U3, U4):** `docs/plans/2026-06-15-002-*.md`, `CLAUDE.md`,
`PROJECT_MEMORY.md`.

**Delete (U5):**

- `openspec/` (config.yaml + specs/ + changes/ + changes/archive/)
- `.claude/commands/opsx/` (4 files)
- `.claude/skills/openspec-*/` (4 dirs)
- `.codex/skills/openspec-*/` (4 dirs)
- `.github/prompts/opsx-*.prompt.md` (4 files)
- `.github/skills/openspec-*/` (4 dirs)
- `.opencode/command/opsx-*.md` (4 files)

**Create (U1):** `docs/migration/openspec-sunset-log.md`.

## References

- `openspec/config.yaml` — schema `spec-driven` + the 4 rule sections being retired.
- `openspec/specs/` — 37 capability contracts (full list in KTD2 table).
- `openspec/changes/{fix-battle-attack-system,fix-sortie-state-and-routing,
  harden-battle-refactor-followup,fix-batch-craft-quest-progress}/` — U2 sources.
- `CLAUDE.md` lines 177, 198, 216, 221 — openspec gate wording (U4 targets).
- `PROJECT_MEMORY.md` — openspec notes + §Next Session (U4 target).
- `docs/plans/2026-06-15-002-feat-battle-map-client-sync-loop-plan.md` U4/KTD6 — U3 target.
- `docs/plans/2026-06-15-001-fix-battle-sim-harness-hardening-plan.md` — plan-format reference for U2.
- `docs/solutions/logic-errors/cache-list-character-holes-exclusion-2026-06-15.md` — ce-compound frontmatter reference for U1.
- ce-compound skill: `~/.pi/agent/skills/ce-compound/` (resolution template, knowledge-track categories).
- ce-compound-refresh skill: `~/.pi/agent/skills/ce-compound-refresh/`.
- PROJECT_MEMORY.md §Verified Facts — layered-crate architecture, `_impl` pattern, do-not-modify list (unchanged by this migration).
