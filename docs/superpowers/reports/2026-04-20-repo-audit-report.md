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
- Workspace crate splitting is directionally correct, with clear domain labels across gameplay, model, database, and runtime layers.
- `Gameplay` is built around context traits (`HasContext` plus composed ops traits), which keeps domain logic reusable and test-oriented instead of HTTP-bound.
- The entry layer remains relatively thin (`src/bin/emukcd.rs` bootstrap and router composition in `src/bin/net/router/kcsapi/mod.rs`), which is a positive architectural sign.

What is straining:
- `emukc_internal` is convenient, but it currently acts as a broad umbrella facade that can blur explicit dependency boundaries at usage sites.
- Gameplay owns most of the hard domain logic, and `battle/core.rs` at 4,481 lines is dramatically larger than nearby gameplay modules, which is now an architecture smell.
- Large-file concentration increases coupling and change blast radius inside the domain core even when crate-level layering looks clean.

Judgment: The current crate layering is still helping more than hurting because boundaries between runtime entry, shared facade, and gameplay domain are visible and mostly coherent. However, the benefit margin is narrowing as oversized domain files accumulate, so the architecture remains net-positive only if the team now decomposes stressed gameplay internals instead of continuing to scale through a single dominant core file.

## Domain Modeling Quality

Score: 71/100
Rating: Promising but stressed

Key observations:
- Battle logic is the most advanced domain area and the most stressed one, with rich behavior concentrated in gameplay battle resolution and settlement flows.
- Route, sortie, and quest logic exist as distinct concepts (`map_route`, `sortie_result`, `quest/update`) rather than only endpoint glue, which indicates real domain modeling intent.
- Correctness fixes in recent commits and associated regression coverage show active model refinement, especially around battle outcomes, progression, and unlock-sensitive behaviors.
- The size of `battle/core.rs` signals concentration risk: the logic is real domain logic, but too much of it is packed into one file.

Judgment: The project has a reusable domain model foundation rather than a pure case-by-case API shim, because key gameplay concepts are represented in dedicated modules and exercised through integration behavior. At the same time, it is still accumulating case-by-case complexity inside the battle core, so reuse exists but is under structural stress and will degrade if decomposition does not keep pace.

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
