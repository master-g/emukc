# EmuKC Repo Audit Design

Date: 2026-04-20
Author: Codex
Status: Drafted for user review

## Objective

Produce a systematic audit report for the `emukc` repository from the perspective of a senior Rust engineer and large-application architect.

The report is intended primarily to guide the next stage of engineering remediation and architectural evolution, with secondary value as a high-level status assessment for current delivery readiness.

## Audit Focus

The audit emphasizes:

1. Current `feat/vibe` delivery readiness and practical completeness.
2. Medium-term architectural health of the full repository.
3. Whether the project is worth continued investment, and under what constraints.
4. Which issues are structural blockers versus manageable debt.

The audit does not try to be a generic style review or a line-by-line bug hunt. It is a repo-level engineering assessment with targeted code evidence.

## Deliverable Style

The final output will follow a balanced engineering-audit format:

1. Executive summary first.
2. Numeric scores plus textual ratings.
3. Clear judgments on readiness, asset quality, debt load, and next-stage risk.
4. Concrete remediation priorities.

This is intentionally not:

- a pure management summary with weak technical grounding
- a PR-style list of isolated findings without synthesis
- a full due-diligence teardown of every crate and subsystem

## Primary Decision Questions

The report must answer the following questions directly:

1. What stage is the project currently in?
2. Is it worth continued heavy investment?
3. Which parts of the repository are durable assets?
4. Which parts are accumulating debt fast enough to threaten the next phase?
5. What should be fixed first before the next major capability expansion?

## Evaluation Dimensions

The audit will score and discuss six dimensions:

1. Delivery completeness
2. Architectural rationality
3. Domain modeling quality
4. Engineering maturity
5. Technology selection fit
6. Evolution risk control

For each dimension, the report will include:

- a numeric score
- a textual rating
- key supporting evidence
- impact on the next stage of development

## Evidence Strategy

The audit will use a combination of repo-level inspection and targeted code sampling.

### Repo-Level Evidence

Inspect:

- workspace layout
- crate graph and dependency direction
- root tooling and conventions
- key project documents
- recent commit trajectory
- existing audit and planning artifacts

Purpose:

- determine whether the repository has a coherent engineering direction
- distinguish active design intent from accidental structure

### Core Flow Evidence

Trace the main path:

`bootstrap -> codex/model -> db -> gameplay -> net/router`

Purpose:

- determine whether crate boundaries align with actual runtime responsibilities
- verify whether the project is evolving around stable layers or blurred coupling

### High-Risk Module Sampling

The audit will draw deeper evidence from modules that concentrate gameplay complexity and architectural pressure:

- `crates/emukc_gameplay/src/game/battle/core.rs`
- `crates/emukc_gameplay/src/game/map_route.rs`
- `crates/emukc_gameplay/src/game/sortie_result.rs`
- `crates/emukc_gameplay/src/game/quest/update.rs`
- `crates/emukc_gameplay/src/gameplay.rs`

These files are chosen because they sit on the project’s most complex domain path and are likely to determine whether future expansion remains tractable.

### Engineering Maturity Evidence

Inspect:

- linting and formatting setup
- pre-commit enforcement
- integration tests and crate-local tests
- benches and performance scaffolding
- battle verification plans versus actual implemented validation
- codex/data snapshot workflows

Purpose:

- determine whether correctness is enforced by workflow or still depends mostly on developer discipline

## Report Structure

The final audit report will be organized in this order:

### 1. Executive Summary

One concise section that states:

- whether the project is worth continued investment
- the current maturity stage
- the strongest assets
- the most serious risks

### 2. Scorecard

A compact score table for the six evaluation dimensions, with both numeric scores and textual ratings.

### 3. Core Judgments

Direct statements on:

- current readiness level
- sustainability of the present architecture
- where feature expansion is still safe
- where the project must stop accumulating complexity and start paying down engineering debt

### 4. Dimension-by-Dimension Audit

For each dimension:

- score
- evidence
- architectural judgment
- practical implication

### 5. Structural Issue List

Only issues that materially affect the next phase of development. Minor stylistic or low-leverage concerns are excluded.

### 6. Remediation Roadmap

Prioritized as:

- immediate actions
- next-stage mandatory improvements
- deferrable but important improvements

## Priority Model

Priorities in the report will follow these rules:

### P0

Must be addressed before major next-stage expansion because the issue either:

- corrupts correctness
- destabilizes core domain evolution
- or sharply increases delivery risk

### P1

Not immediately fatal, but likely to increase cost significantly when extending battle fidelity, combined fleets, LBAS, support systems, or broader gameplay correctness.

### P2

Useful quality improvements with meaningful long-term payoff, but not a present expansion blocker.

## Tone and Judgment Standard

The report should be explicit and pragmatic.

It should avoid:

- vague praise
- generic criticism
- checklist theater

It should read like a firm engineering judgment, for example:

> This is a directionally strong project that has moved beyond pure prototype status, but its core gameplay complexity is starting to approach the carrying capacity of the current engineering structure. Continued investment is justified, but the next stage cannot rely on feature accretion alone. Correctness infrastructure, module boundaries, and domain-level verification now need to be treated as productized engineering assets.

## Out of Scope

The audit will not attempt to:

- rewrite architecture during the audit itself
- produce code changes
- produce a full implementation plan for remediation in this step
- provide a commit-by-commit bug review of the entire branch history

## Completion Criteria

The design is complete when it enables the next step: writing the actual audit report with a stable scope, evidence model, scoring framework, and priority system.
