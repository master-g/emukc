# EmuKC Repository Audit Report

Date: 2026-04-20
Branch: `feat/vibe`
Audit perspective: Senior Rust engineer and large-application architect

## Executive Summary

- Current stage: `internal playable validation` because bootstrap/serve plus core single-fleet gameplay loops are runnable, but major combat topology and validation gaps remain.
- Investment judgment: Continued heavy investment is justified only if the team first hardens battle correctness/verification and resolves the current structural feature-surface gaps before large new feature expansion.
- Strongest assets: runnable bootstrap and local serve flow, implemented single-fleet sortie and practice battle loops on a shared battle core, map unlock progression wired through sortie result settlement with regression coverage
- Most serious risks: incomplete battle formula and behavioral verification stack, missing combined fleet/LBAS/support architecture across many endpoints, codex snapshot/data freshness coupling that can mask map-routing and behavior regressions
- Bottom line: This branch is no longer a throwaway prototype, but it is not yet architecturally safe for aggressive parallel feature expansion. It can support focused internal playable validation and correctness hardening. Broad new-system expansion should wait until battle verification and missing topology foundations are in place.

## Scorecard

| Dimension | Score | Rating | Key Reason |
| --- | ---: | --- | --- |
| Delivery Completeness | 72 | Functional but fragile | Playable core loop exists, but major topology is still missing. |
| Architectural Rationality | 74 | Good | Crate layering is coherent, but battle logic is over-concentrated. |
| Domain Modeling Quality | 71 | Promising but stressed | Real gameplay models exist, but battle complexity is concentrating quickly. |
| Engineering Maturity | 69 | Disciplined but incomplete | Tooling exists, but end-to-end battle verification is still incomplete. |
| Technology Selection Fit | 84 | Good fit | Rust workspace and runtime stack fit the emulator stage well. |
| Evolution Risk Control | 58 | Risky but manageable | Next-stage fidelity pressure currently exceeds verification maturity. |

## Context

- Repository type: multi-crate Rust workspace for a KanColle emulator/runtime
- Current branch focus: battle correctness, quest coverage, RNG cleanup, and progression fixes on `feat/vibe`
- Primary assessment lens: next-stage engineering remediation and architectural evolution
- Secondary lens: current delivery readiness for internal playable validation

## Delivery Completeness

Score: 72/100
Rating: Functional but fragile

The repository currently supports end-to-end local startup (`emukc bootstrap` then `emukc serve`) and a meaningful gameplay slice: single-fleet sortie start/next, day and standard night battle resolution, practice flow, and map-clear unlock progression surfaced through runtime APIs. At the system level, the implementation and test suite show those loops exercising profile/session setup, battle/result settlement, next-node advancement, quest progress, and unlock state transitions, while the planning artifacts still show major feature-surface gaps in advanced battle topologies.

Evidence:
- `README.md` documents a complete bootstrap + serve local execution path (`emukc bootstrap`, `emukc serve`, then browser open).
- `crates/emukc_gameplay/tests/sortie_battle.rs` includes direct integration coverage for single-fleet sortie flow, including `sortie_start_battle_result_flow_updates_stats`, `next_sortie`, air battle reuse, ship drop settlement, and battle-response validation.
- `crates/emukc_gameplay/tests/practice_battle.rs` includes direct integration coverage for practice flow, including `practice_battle_and_result_flow_updates_rival_status`, result settlement, resource consumption, and exercise quest progression across repeated battles.
- `tests/gameplay_tests/map/unlock.rs` verifies the public unlock path: a new profile only sees 1-1, clearing 1-1 through repeated sortie/battle/result calls unlocks 1-2 in `get_map_infos`, and sortie to a locked map fails.
- `docs/plan.md` also lists combined fleet / LBAS / support as a large remaining gap (14+ endpoints, major feature gap), confirming readiness limits.

Judgment: The branch is beyond pure prototype status and should be treated as an internally usable, feature-partial validation build. It is functionally playable for a constrained single-fleet core loop, but still structurally fragile for broad production-like expansion because high-impact combat correctness and major battle-topology capabilities remain incomplete.

## Architectural Rationality

Score: 74/100
Rating: Good

What is working:
- Workspace crate splitting is directionally correct: `Cargo.toml` defines `[workspace] members = ["crates/*"]`, and crate labels like `emukc_gameplay`, `emukc_model`, and `emukc_db` map to clear domain boundaries.
- `Gameplay` in `crates/emukc_gameplay/src/gameplay.rs` is built around `HasContext` plus composed ops traits (`AccountOps`, `ProfileOps`, `GameOps`), keeping domain logic reusable and test-oriented instead of HTTP-bound.
- The entry layer remains relatively thin: `src/bin/emukcd.rs` is bootstrap wiring, while `src/bin/net/router/kcsapi/mod.rs` primarily composes route modules and middleware.

What is straining:
- `emukc_internal` is convenient, but `crates/emukc_internal/src/lib.rs` re-exports a wide surface (`app`, `bootstrap`, `cache`, `db`, `gameplay`, `model`, `network`, etc.), so usage sites can lose explicit boundary signals.
- Gameplay owns most of the hard domain logic, and `crates/emukc_gameplay/src/game/battle/core.rs` at 4,481 lines is dramatically larger than nearby modules like `crates/emukc_gameplay/src/game/map_route.rs` (668), `crates/emukc_gameplay/src/game/sortie_result.rs` (539), and `crates/emukc_gameplay/src/game/quest/update.rs` (481).
- Large-file concentration in `crates/emukc_gameplay/src/game/battle/core.rs` increases coupling and change blast radius inside the domain core even though crate-level layering in `Cargo.toml` looks clean.

Judgment: The current crate layering is still helping more than hurting because boundaries between runtime entry (`src/bin/emukcd.rs`), shared facade (`crates/emukc_internal/src/lib.rs`), and gameplay domain (`crates/emukc_gameplay/src/gameplay.rs`) remain visible and coherent. The margin is narrower than before, with architectural strain concentrated in the oversized `crates/emukc_gameplay/src/game/battle/core.rs`.

## Domain Modeling Quality

Score: 71/100
Rating: Promising but stressed

Key observations:
- Battle logic is the most advanced and most stressed domain area, with major behavior concentrated in `crates/emukc_gameplay/src/game/battle/core.rs` (battle resolution and settlement paths).
- Route, sortie, and quest logic are modeled as distinct concepts in `crates/emukc_gameplay/src/game/map_route.rs`, `crates/emukc_gameplay/src/game/sortie_result.rs`, and `crates/emukc_gameplay/src/game/quest/update.rs`, not only as endpoint glue.
- Correctness fixes in recent commits are reflected in regression-oriented coverage already anchored in this report (`crates/emukc_gameplay/tests/sortie_battle.rs`, `crates/emukc_gameplay/tests/practice_battle.rs`, `tests/gameplay_tests/map/unlock.rs`), indicating active model refinement around outcomes and progression.
- The size of `crates/emukc_gameplay/src/game/battle/core.rs` (4,481) versus nearby domain files (481-668) signals concentration risk even though the logic itself is real domain logic.

Judgment: The project has a reusable domain model foundation rather than a pure case-by-case API shim, because key gameplay concepts are represented in dedicated modules and exercised through integration behavior. At the same time, complexity is accumulating unevenly inside `crates/emukc_gameplay/src/game/battle/core.rs`, so current reuse quality is real but structurally stressed.

## Engineering Maturity

Score: 69/100
Rating: Disciplined but incomplete
What is in place:
- `.pre-commit-config.yaml` defines local hooks for `cargo fmt --check` and `cargo clippy` (`-W warnings`) in the expected pre-commit workflow.
- Tests exist in multiple layers: repository-level integration tests under `tests/` and crate-local tests in `emukc_cache`, `emukc_db`, and gameplay-heavy suites like `crates/emukc_gameplay/tests/sortie_battle.rs`.
- Benchmarking is present via Criterion (`crates/emukc_cache/benches/version_cache_bench.rs`), giving at least one concrete performance signal.
What is missing:
- battle verification is still incomplete for this domain: `docs/plan.md` explicitly treats V2 behavioral and V3 resource validation as ongoing track work, not a finished enforcement layer.
- codex snapshot freshness is an operational risk (`tests/README.md`): `.data/codex` can drift from repo-tracked assets and produce misleading test outcomes unless manually refreshed.
- The repository has lint/format hooks and layered tests, but available evidence does not show a uniformly automated, always-on gate for combat legality and battle-resource consistency invariants.
Judgment: The repository has moved beyond ad hoc process and now has repeatable tooling support, but it does not yet enforce correctness systematically end to end. For battle semantics and data freshness, it still depends too much on careful authors consistently following validation and codex-refresh practices.

## Technology Selection Fit

Score: 84/100
Rating: Good fit
Assessment:
- Rust is a strong fit for correctness-heavy emulator logic where deterministic state transitions and strict typing reduce accidental behavior drift.
- The multi-crate workspace structure matches the current repository shape and keeps `gameplay`, `db`, `model`, `bootstrap`, and cache concerns reasonably separable.
- `axum` + `tokio` is a practical server-side combination for async API handling and iterative local validation in this project.
- `sqlite` + `SeaORM` is acceptable for the current stage: it supports rapid iteration and testability, even if it is not the final scaling story.
- The dominant risks are domain-complexity management problems (rule depth, validation surface, and correctness control), not fashionable-stack misalignment problems.
Judgment: The stack matches the current project stage well: it is pragmatic, productive, and technically coherent for internal validation and correctness hardening. Near-term leverage comes from strengthening verification and domain controls rather than replacing core technologies.

## Evolution Risk Control

Score: 58/100
Rating: Risky but manageable

Current risk drivers:
- Remaining day-battle fidelity work, special OASW, and taxonomy corrections are increasing pressure on an already dense battle model.
- Combined fleet, LBAS, and support expedition are cross-cutting topology expansions, not small additive features.
- Validation is incomplete beyond structural checks, so the expanding correctness surface has more room to generate silent regressions.

Judgment: The repository can continue targeted internal validation work, but it cannot safely absorb the next wave of gameplay complexity as straight feature accretion. Upcoming battle-fidelity work will add more interacting rule branches to a model that is already heavily concentrated, and combined fleet, LBAS, and support would widen both architecture and runtime-state complexity across many endpoints. Because behavioral and resource-level validation are still unfinished, future regressions will be harder to detect and localize. The risk is therefore mixed rather than purely architectural: concentrated battle logic is one problem, but the larger issue is that correctness surface area is growing faster than the repo’s ability to verify it. Major topology expansion should pause until verification hardening materially improves control.

## Structural Issues

### P0
- Battle correctness is still under-controlled: formula gaps, special OASW/taxonomy omissions, and missing behavioral/resource validation create persistent correctness risk, and further major feature expansion should pause until verification hardens.
- Core battle/domain logic is too concentrated in `crates/emukc_gameplay/src/game/battle/core.rs`, which increases change blast radius and makes next-stage fidelity work materially harder to reason about.

### P1
- Combined fleet, LBAS, and support expedition require cross-cutting changes across battle modeling, settlement, and API surface, so treating them as additive endpoint work would compound current structural debt.
- Codex/data freshness and battle knowledge synchronization remain operationally fragile, which can hide routing or behavior regressions behind stale assets and weaken confidence in results.

### P2
- Display/response rules and arrival-context routing are still partly hardcoded or incomplete, adding local maintenance friction without yet being the primary expansion blocker.
- Test and benchmark coverage exists but remains uneven across crates and subsystems, limiting trend visibility more than near-term delivery.

## Recommended Roadmap

### Immediate Actions
- Freeze further major combined-fleet, LBAS, and support expansion until battle verification and invariants are hardened.
- Treat battle correctness hardening as a dedicated workstream: close the highest-impact day-battle, special OASW, and taxonomy gaps with regression coverage.
- Start decomposing `crates/emukc_gameplay/src/game/battle/core.rs` around stable simulation boundaries so the next changes land into smaller control surfaces.

### Next-Stage Mandatory Improvements
- Implement behavioral validation for phase ordering, attacker legality, `api_si_list`, HP delta accounting, win rank, and MVP consistency.
- Implement resource-existence validation tied to battle slot/resource trigger data and make stale-asset detection part of the normal refresh workflow.
- Design combined fleet, LBAS, and support around explicit topology abstractions and settlement contracts instead of extending single-fleet assumptions in place.

### Deferrable Improvements
- Normalize the remaining display/response-rule hardcoding after correctness and topology hardening stabilize.
- Expand broader benchmark and property-style coverage once the validation model and battle-module boundaries stop shifting.

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
