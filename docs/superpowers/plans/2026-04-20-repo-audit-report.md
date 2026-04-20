# Repo Audit Report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a systematic repository audit report for `emukc` that evaluates delivery completeness, architecture, domain modeling, engineering maturity, technology selection, and next-stage evolution risk.

**Architecture:** Create one report document under `docs/superpowers/reports/`, then fill it section by section from a fixed evidence set. Keep the workflow linear: scaffold the report, gather evidence in focused batches, write each audit dimension from concrete repo artifacts, then synthesize the scorecard and remediation roadmap.

**Tech Stack:** Markdown, `git`, `rg`, `find`, `wc`, `sed`, existing repo docs, Rust workspace metadata already present in the repository

---

### Task 1: Scaffold the Audit Report Document

**Files:**
- Create: `docs/superpowers/reports/2026-04-20-repo-audit-report.md`
- Reference: `docs/superpowers/specs/2026-04-20-repo-audit-design.md`

- [ ] **Step 1: Create the reports directory**

Run:

```bash
mkdir -p docs/superpowers/reports
```

Expected: command exits successfully and `docs/superpowers/reports` exists.

- [ ] **Step 2: Create the report scaffold**

Write this exact file to `docs/superpowers/reports/2026-04-20-repo-audit-report.md`:

```markdown
# EmuKC Repository Audit Report

Date: 2026-04-20
Branch: `feat/vibe`
Audit perspective: Senior Rust engineer and large-application architect

## Executive Summary

## Scorecard

| Dimension | Score | Rating | Key Reason |
| --- | ---: | --- | --- |
| Delivery Completeness |  |  |  |
| Architectural Rationality |  |  |  |
| Domain Modeling Quality |  |  |  |
| Engineering Maturity |  |  |  |
| Technology Selection Fit |  |  |  |
| Evolution Risk Control |  |  |  |

## Context

## Delivery Completeness

## Architectural Rationality

## Domain Modeling Quality

## Engineering Maturity

## Technology Selection Fit

## Evolution Risk Control

## Structural Issues

### P0

### P1

### P2

## Recommended Roadmap

### Immediate Actions

### Next-Stage Mandatory Improvements

### Deferrable Improvements

## Appendix: Evidence Commands
```

- [ ] **Step 3: Verify the scaffold contains every required heading**

Run:

```bash
rg -n "^#|^##|^###" docs/superpowers/reports/2026-04-20-repo-audit-report.md
```

Expected: output includes `## Executive Summary`, `## Scorecard`, all six audit dimensions, `## Structural Issues`, `## Recommended Roadmap`, and `## Appendix: Evidence Commands`.

- [ ] **Step 4: Commit the scaffold**

Run:

```bash
git add docs/superpowers/reports/2026-04-20-repo-audit-report.md
git commit -m "docs: scaffold repo audit report"
```

Expected: one commit containing only the new report scaffold.

### Task 2: Fill Context and Delivery Completeness

**Files:**
- Modify: `docs/superpowers/reports/2026-04-20-repo-audit-report.md`
- Reference: `README.md`
- Reference: `docs/plan.md`
- Reference: `docs/audit.md`
- Reference: `tests/README.md`

- [ ] **Step 1: Gather delivery-readiness evidence**

Run:

```bash
sed -n '1,220p' README.md
sed -n '1,260p' docs/plan.md
sed -n '1,220p' docs/audit.md
sed -n '1,220p' tests/README.md
git log --oneline -n 20
find tests -type f | sort | xargs wc -l | tail -n 20
```

Expected:

- `README.md` shows bootstrap and serve flows.
- `docs/plan.md` lists completed gameplay flows and remaining gaps.
- `docs/audit.md` captures recent correctness findings.
- `tests/README.md` describes gameplay integration tests.
- `git log` shows recent work concentrated on battle formula, RNG, quests, and bug fixes.

- [ ] **Step 2: Write the `Context` section**

Replace `## Context` with this structure and fill each bullet from Step 1 evidence:

```markdown
## Context

- Repository type: multi-crate Rust workspace for a KanColle emulator/runtime
- Current branch focus: battle correctness, quest coverage, RNG cleanup, and progression fixes on `feat/vibe`
- Primary assessment lens: next-stage engineering remediation and architectural evolution
- Secondary lens: current delivery readiness for internal playable validation
```

- [ ] **Step 3: Write the `Executive Summary` section**

Write a concise summary under `## Executive Summary` with exactly five bullets in this order:

- `Current stage:` followed by one of these exact stage labels: `prototype validation`, `sustainable development`, `internal playable validation`, or `near external usability`, then one sentence of justification.
- `Investment judgment:` followed by one sentence stating whether continued heavy investment is justified and what condition must be met.
- `Strongest assets:` followed by exactly three comma-separated repo-backed strengths.
- `Most serious risks:` followed by exactly three comma-separated structural risks.
- `Bottom line:` followed by two or three sentences with a hard architectural judgment.

Rules for this section:

- The stage must be chosen from the four options shown above.
- The three strongest assets must come from current repo evidence, not aspiration.
- The three most serious risks must be structural, not stylistic.
- The bottom line must clearly state whether the project can keep expanding features safely right now.

- [ ] **Step 4: Write the `Delivery Completeness` section**

Write `## Delivery Completeness` with exactly these elements in this order:

1. One `Score:` line with a final numeric score.
2. One `Rating:` line using exactly one of these labels: `Immature`, `Emerging`, `Functional but fragile`, `Strong`.
3. One paragraph describing which user-facing and system-level flows are already working.
4. An `Evidence:` list with exactly five bullets.
5. A `Judgment:` label followed by one paragraph answering whether the branch is beyond pure prototype status and what kind of usable system it currently is.

Mandatory evidence items to mention:

- bootstrap + serve flow exists
- single-fleet sortie flow is implemented
- practice flow is implemented
- map unlock progression is implemented
- combined fleet / LBAS / support remain missing or large gaps

- [ ] **Step 5: Verify these sections are populated**

Run:

```bash
rg -n "^\-\sCurrent stage:|^Score:|^Rating:|^Evidence:|^Judgment:" docs/superpowers/reports/2026-04-20-repo-audit-report.md
```

Expected: matches from `Executive Summary` and `Delivery Completeness`.

- [ ] **Step 6: Commit the context and completeness sections**

Run:

```bash
git add docs/superpowers/reports/2026-04-20-repo-audit-report.md
git commit -m "docs: add repo audit context and delivery assessment"
```

Expected: one commit with only report-content changes.

### Task 3: Fill Architecture and Domain Modeling Sections

**Files:**
- Modify: `docs/superpowers/reports/2026-04-20-repo-audit-report.md`
- Reference: `Cargo.toml`
- Reference: `crates/emukc_internal/src/lib.rs`
- Reference: `crates/emukc_gameplay/src/gameplay.rs`
- Reference: `crates/emukc_gameplay/src/game/battle/core.rs`
- Reference: `crates/emukc_gameplay/src/game/map_route.rs`
- Reference: `crates/emukc_gameplay/src/game/sortie_result.rs`
- Reference: `crates/emukc_gameplay/src/game/quest/update.rs`
- Reference: `src/bin/emukcd.rs`
- Reference: `src/bin/net/router/kcsapi/mod.rs`

- [ ] **Step 1: Gather architecture evidence**

Run:

```bash
sed -n '1,260p' Cargo.toml
sed -n '1,220p' crates/emukc_internal/src/lib.rs
sed -n '1,260p' crates/emukc_gameplay/src/gameplay.rs
sed -n '1,120p' src/bin/emukcd.rs
sed -n '1,220p' src/bin/net/router/kcsapi/mod.rs
wc -l \
  crates/emukc_gameplay/src/game/battle/core.rs \
  crates/emukc_gameplay/src/game/map_route.rs \
  crates/emukc_gameplay/src/game/sortie_result.rs \
  crates/emukc_gameplay/src/game/quest/update.rs \
  crates/emukc_model/src/codex/mod.rs
```

Expected:

- workspace uses multiple crates with clear domain labels
- `emukc_internal` is an aggregation facade
- `Gameplay` is built around context traits
- HTTP entry remains relatively thin
- `battle/core.rs` is dramatically larger than surrounding gameplay files

- [ ] **Step 2: Write the `Architectural Rationality` section**

Write `## Architectural Rationality` with exactly these elements in this order:

1. One `Score:` line with a final numeric score.
2. One `Rating:` line using exactly one of these labels: `Poor`, `Mixed`, `Good`, `Strong`.
3. A `What is working:` list with exactly three bullets.
4. A `What is straining:` list with exactly three bullets.
5. A `Judgment:` label followed by one paragraph on whether the current crate layering is still helping more than hurting.

Mandatory points to cover:

- workspace split is directionally correct
- `emukc_internal` is convenient but also acts as a broad umbrella layer
- gameplay owns most of the hard domain logic
- thin entry layer is a positive sign
- very large domain files are becoming an architecture smell

- [ ] **Step 3: Write the `Domain Modeling Quality` section**

Write `## Domain Modeling Quality` with exactly these elements in this order:

1. One `Score:` line with a final numeric score.
2. One `Rating:` line using exactly one of these labels: `Weak`, `Uneven`, `Promising but stressed`, `Strong`.
3. A `Key observations:` list with exactly four bullets.
4. A `Judgment:` label followed by one paragraph on whether the project has a reusable domain model or is still accumulating case-by-case logic.

Mandatory points to cover:

- battle logic is the most advanced and most stressed domain area
- route / sortie / quest logic exists as distinct concepts, not only endpoint glue
- correctness fixes in recent commits show active model refinement
- the size of `battle/core.rs` signals concentration risk even if the logic is real

- [ ] **Step 4: Verify both sections exist and mention the large battle core**

Run:

```bash
rg -n "Architectural Rationality|Domain Modeling Quality|battle/core\\.rs|Gameplay|emukc_internal" docs/superpowers/reports/2026-04-20-repo-audit-report.md
```

Expected: matches in both sections, including at least one explicit mention of `battle/core.rs`.

- [ ] **Step 5: Commit the architecture and domain sections**

Run:

```bash
git add docs/superpowers/reports/2026-04-20-repo-audit-report.md
git commit -m "docs: add architecture and domain modeling audit"
```

Expected: one commit with only report-content changes.

### Task 4: Fill Engineering Maturity and Technology Selection Sections

**Files:**
- Modify: `docs/superpowers/reports/2026-04-20-repo-audit-report.md`
- Reference: `.pre-commit-config.yaml`
- Reference: `.rustfmt.toml`
- Reference: `tests/README.md`
- Reference: `docs/plan.md`
- Reference: `crates/emukc_cache/benches/version_cache_bench.rs`
- Reference: `crates/emukc_cache/tests/basic_operations.rs`
- Reference: `crates/emukc_gameplay/tests/sortie_battle.rs`
- Reference: `crates/emukc_db/tests/user.rs`

- [ ] **Step 1: Gather engineering-maturity evidence**

Run:

```bash
sed -n '1,220p' .pre-commit-config.yaml
sed -n '1,220p' .rustfmt.toml
sed -n '1,220p' tests/README.md
sed -n '1,260p' docs/plan.md
find crates -path '*/tests/*' -o -path '*/benches/*' -type f | sort | sed -n '1,200p'
sed -n '1,220p' crates/emukc_cache/benches/version_cache_bench.rs
```

Expected:

- pre-commit runs `cargo fmt --check` and `cargo clippy`
- formatting rules are explicit
- there are integration tests and crate-local tests
- there is at least one benchmark
- battle verification is still partly documented as future work, not fully productized

- [ ] **Step 2: Write the `Engineering Maturity` section**

Write `## Engineering Maturity` with exactly these elements in this order:

1. One `Score:` line with a final numeric score.
2. One `Rating:` line using exactly one of these labels: `Ad hoc`, `Developing`, `Disciplined but incomplete`, `Strong`.
3. A `What is in place:` list with exactly three bullets.
4. A `What is missing:` list with exactly three bullets.
5. A `Judgment:` label followed by one paragraph answering whether the repo enforces correctness systematically or still depends too much on careful authors.

Mandatory points to cover:

- `fmt` and `clippy` are enforced through pre-commit
- test coverage exists in multiple layers, especially gameplay
- benchmark presence is a positive but narrow signal
- battle verification infrastructure is still incomplete relative to domain complexity
- codex/data snapshot freshness is a real operational concern

- [ ] **Step 3: Write the `Technology Selection Fit` section**

Write `## Technology Selection Fit` with exactly these elements in this order:

1. One `Score:` line with a final numeric score.
2. One `Rating:` line using exactly one of these labels: `Misaligned`, `Mostly fit`, `Good fit`, `Very good fit`.
3. An `Assessment:` list with exactly five bullets.
4. A `Judgment:` label followed by one paragraph on whether the stack matches the current project stage and team operating model.

Mandatory points to cover:

- Rust is a good fit for correctness-heavy emulator logic
- multi-crate workspace is appropriate for the current repo shape
- axum + tokio is a practical server-side choice here
- sqlite + SeaORM is acceptable for current stage, even if not the final scaling story
- the main risks are not fashionable-stack problems but domain-complexity management problems

- [ ] **Step 4: Verify both sections mention tooling and validation gaps**

Run:

```bash
rg -n "Engineering Maturity|Technology Selection Fit|pre-commit|clippy|battle verification|codex|SeaORM|axum|tokio" docs/superpowers/reports/2026-04-20-repo-audit-report.md
```

Expected: both sections contain explicit discussion of tooling and one or more validation gaps.

- [ ] **Step 5: Commit the maturity and tech sections**

Run:

```bash
git add docs/superpowers/reports/2026-04-20-repo-audit-report.md
git commit -m "docs: add engineering maturity and tech selection audit"
```

Expected: one commit with only report-content changes.

### Task 5: Fill Evolution Risk, Scorecard, and Roadmap

**Files:**
- Modify: `docs/superpowers/reports/2026-04-20-repo-audit-report.md`
- Reference: `docs/plan.md`
- Reference: `docs/audit.md`
- Reference: `docs/superpowers/specs/2026-04-20-repo-audit-design.md`

- [ ] **Step 1: Gather risk evidence**

Run:

```bash
sed -n '1,260p' docs/plan.md
sed -n '1,220p' docs/audit.md
sed -n '1,260p' docs/superpowers/specs/2026-04-20-repo-audit-design.md
```

Expected:

- open gaps include day-battle fidelity, special OASW, taxonomy, combined fleet, LBAS, support, and battle verification
- recent audit findings identify correctness and consistency risks
- the design spec defines P0/P1/P2 priority rules

- [ ] **Step 2: Write the `Evolution Risk Control` section**

Write `## Evolution Risk Control` with exactly these elements in this order:

1. One `Score:` line with a final numeric score.
2. One `Rating:` line using exactly one of these labels: `High risk`, `Risky but manageable`, `Manageable`, `Well controlled`.
3. A `Current risk drivers:` list with exactly three bullets.
4. A `Judgment:` label followed by one paragraph on whether the repo can safely absorb the next wave of gameplay complexity without first hardening its engineering structure.

Mandatory points to cover:

- upcoming battle-fidelity work increases pressure on current modeling
- combined fleet / LBAS / support are not small additive features
- incomplete validation infrastructure amplifies future regression risk
- not all risk comes from architecture; some comes from correctness surface area

- [ ] **Step 3: Fill the `Scorecard` table**

Fill every blank cell in the existing scorecard. Apply this scoring rule:

```markdown
- 80-100: strong with no major expansion blocker in the dimension
- 65-79: good enough to continue, but notable debt exists
- 50-64: workable but constraining
- below 50: major blocker territory
```

Use one short reason per dimension in the `Key Reason` column.

- [ ] **Step 4: Fill `Structural Issues` and `Recommended Roadmap`**

Write `## Structural Issues` and `## Recommended Roadmap` with these exact counts:

- `### P0` must contain exactly two bullets.
- `### P1` must contain exactly two bullets.
- `### P2` must contain exactly two bullets.
- `### Immediate Actions` must contain exactly three bullets.
- `### Next-Stage Mandatory Improvements` must contain exactly three bullets.
- `### Deferrable Improvements` must contain exactly two bullets.

Mandatory P0 themes to consider and explicitly decide on:

- battle correctness / validation gap
- oversized concentrated domain logic
- whether further major feature expansion should pause until verification hardens

- [ ] **Step 5: Add the evidence commands appendix**

Under `## Appendix: Evidence Commands`, add these four subsections with these exact command blocks:

```markdown
## Appendix: Evidence Commands

### Delivery
```bash
sed -n '1,220p' README.md
sed -n '1,260p' docs/plan.md
sed -n '1,220p' docs/audit.md
sed -n '1,220p' tests/README.md
git log --oneline -n 20
find tests -type f | sort | xargs wc -l | tail -n 20
```

### Architecture
```bash
sed -n '1,260p' Cargo.toml
sed -n '1,220p' crates/emukc_internal/src/lib.rs
sed -n '1,260p' crates/emukc_gameplay/src/gameplay.rs
sed -n '1,120p' src/bin/emukcd.rs
sed -n '1,220p' src/bin/net/router/kcsapi/mod.rs
wc -l \
  crates/emukc_gameplay/src/game/battle/core.rs \
  crates/emukc_gameplay/src/game/map_route.rs \
  crates/emukc_gameplay/src/game/sortie_result.rs \
  crates/emukc_gameplay/src/game/quest/update.rs \
  crates/emukc_model/src/codex/mod.rs
```

### Engineering Maturity
```bash
sed -n '1,220p' .pre-commit-config.yaml
sed -n '1,220p' .rustfmt.toml
sed -n '1,220p' tests/README.md
sed -n '1,260p' docs/plan.md
find crates -path '*/tests/*' -o -path '*/benches/*' -type f | sort | sed -n '1,200p'
sed -n '1,220p' crates/emukc_cache/benches/version_cache_bench.rs
```

### Risk
```bash
sed -n '1,260p' docs/plan.md
sed -n '1,220p' docs/audit.md
sed -n '1,260p' docs/superpowers/specs/2026-04-20-repo-audit-design.md
```
```

- [ ] **Step 6: Commit the final synthesized report**

Run:

```bash
git add docs/superpowers/reports/2026-04-20-repo-audit-report.md
git commit -m "docs: complete repo audit report"
```

Expected: one commit with the finished audit report.

### Task 6: Final Review and Cleanup

**Files:**
- Modify: `docs/superpowers/reports/2026-04-20-repo-audit-report.md`
- Reference: `docs/superpowers/specs/2026-04-20-repo-audit-design.md`

- [ ] **Step 1: Check spec coverage**

Run:

```bash
sed -n '1,260p' docs/superpowers/specs/2026-04-20-repo-audit-design.md
sed -n '1,320p' docs/superpowers/reports/2026-04-20-repo-audit-report.md
```

Expected: the report contains all six evaluation dimensions, an executive summary, scorecard, structural issues, and roadmap.

- [ ] **Step 2: Scan for placeholders and empty score fields**

Run:

```bash
rg -n "TODO|TBD|FIXME|\\|\\s*\\|\\s*\\|" docs/superpowers/reports/2026-04-20-repo-audit-report.md
```

Expected: no matches. If matches exist, remove them before continuing.

- [ ] **Step 3: Check consistency of scores, ratings, and conclusions**

Read the report top to bottom and verify:

- scorecard ratings match the detailed sections
- P0 issues actually appear severe enough to justify P0
- the executive summary does not claim a stronger maturity stage than the body supports

- [ ] **Step 4: Run a final diff review**

Run:

```bash
git diff -- docs/superpowers/reports/2026-04-20-repo-audit-report.md
git log --oneline -n 5
```

Expected: the diff shows only audit-report changes and the recent commit sequence reflects scaffold -> evidence-backed sections -> final synthesis.

- [ ] **Step 5: Amend only if review fixes are needed, otherwise leave commits as-is**

If no fixes are needed, do nothing.

If fixes are needed, run:

```bash
git add docs/superpowers/reports/2026-04-20-repo-audit-report.md
git commit -m "docs: refine repo audit report"
```

Expected: final report is internally consistent and ready to hand back to the user.
