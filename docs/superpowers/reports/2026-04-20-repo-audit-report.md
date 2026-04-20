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
| Delivery Completeness |  |  |  |
| Architectural Rationality |  |  |  |
| Domain Modeling Quality |  |  |  |
| Engineering Maturity |  |  |  |
| Technology Selection Fit |  |  |  |
| Evolution Risk Control |  |  |  |

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
