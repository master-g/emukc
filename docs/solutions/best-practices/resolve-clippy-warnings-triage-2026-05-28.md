---
title: "Clippy Warning Triage: Mechanical vs Analysis-Required vs Suppress"
date: 2026-05-28
category: best-practices
module: workspace
problem_type: best_practice
component: tooling
severity: low
applies_when:
  - "Running cargo clippy --workspace produces warnings across multiple crates"
  - "match_same_arms warnings appear on CI-type enums where identical values may hide bugs"
  - "Deciding whether to suppress too_many_arguments or refactor a function signature"
  - "Choosing between #[allow] and #[expect] for intentionally dead code"
tags: [clippy, linting, code-quality, rust-idioms, match-same-arms, let-else]
---

# Clippy Warning Triage: Mechanical vs Analysis-Required vs Suppress

## Context

Commit `b7d4971` resolved 34 clippy warnings across 4 crates in the emukc workspace. The warnings fell into 7 categories spanning mechanical formatting to design smells. The challenge was not the fixes themselves but deciding *which fixes were safe to apply mechanically* versus *which required domain analysis* — particularly `match_same_arms` warnings that could mask real bugs.

This was the third clippy cleanup pass in the project's history: 9 warnings in bootstrap crate (2026-05-07), 2 in gameplay crate (2026-05-15), and now 34 across all 4 crates. The pattern recurred each time new code was added without clippy-zeroing as part of the workflow. (session history)

## Guidance

Classify every clippy warning into one of three tiers before acting:

### Tier 1 — Mechanical (auto-fix safe)

Warnings where the fix is provably semantics-preserving. Batch-fix with `cargo clippy --fix --workspace --allow-dirty`:

- `doc_markdown` — backtick-wrapping type names in doc comments
- `collapsible_if` — merging nested conditions with `&&`
- `semicolon_if_nothing_returned` — adding a semicolon

`cargo clippy --fix` resolved 15 of 34 warnings automatically in this pass.

### Tier 2 — Needs analysis (verify intent before merging)

`match_same_arms` — two match arms produce the same value. The fix is mechanical (merge with `|`), but the *decision* is not. Each pair must be verified against domain knowledge.

Example: `MainApSecCI` and `CarrierCI` both produce `140.0` damage multiplier — confirmed intentional (both are CI attacks with the same modifier). Had one been `140.0` by accident when it should have been `120.0`, silently merging would hide the bug.

```rust
// Before (verified that both CI types intentionally share the modifier):
DayAttackType::MainApSecCI => 140.0,
DayAttackType::CarrierCI => 140.0,
// After:
DayAttackType::MainApSecCI | DayAttackType::CarrierCI => 140.0,
```

`let_and_return` / `let...else` rewrites also fall here — `cargo clippy --fix` does not handle them because they restructure control flow:

```rust
// Before:
let air = match air_state {
    Some(a) => a,
    None => return false,
};
// After:
let Some(air) = air_state else {
    return false;
};
```

### Tier 3 — Suppress with documented intent

Warnings pointing to a real design issue where refactoring carries more risk than the warning:

```rust
// too_many_arguments: 13 params, simple push logic, freshly audited battle code
#[allow(clippy::too_many_arguments)]
fn push_attack(...)
```

```rust
// dead_code: WIP field with known future intent (token validation)
#[expect(dead_code)]
pub token: String,
```

Prefer `#[expect]` over `#[allow]` when the warning has a known future resolution. `#[expect]` communicates "I have a plan" and will itself warn if the field eventually gets used, prompting cleanup. (session history: the `PaymentSession.token` field was kept after code review deliberately chose profile_id-only validation over token comparison)

## Why This Matters

Without triage, clippy cleanup falls into two traps:

1. **Auto-fix everything** — `match_same_arms` merges can hide copy-paste bugs where two arms *should* have differed. The warning was trying to tell you something.
2. **Refactor everything** — suppressing `too_many_arguments` on a 13-parameter function that does trivial push logic is cheaper than introducing a parameter struct, updating all call sites, and risking regressions in freshly-audited battle code.

The tier system makes intent explicit: mechanical fixes are safe to batch, analytical fixes require domain verification, and suppressed warnings carry forward a documented reason.

## When to Apply

- During any `cargo clippy --workspace` cleanup pass
- When clippy warnings block CI and must be resolved quickly
- When warnings span multiple crates with different domain complexity (mechanical crates can be batched; gameplay crates need per-warning review)
- When deciding between `#[allow]` and `#[expect]` — prefer `#[expect]` when the warning has a known future resolution

## Examples

### Collapsible if (Tier 1)

```rust
// Before:
if voice_flag & 2 != 0 {
    if let (Some(start), Some(count)) = (start_voice, voice_count) {
// After:
if voice_flag & 2 != 0
    && let (Some(start), Some(count)) = (start_voice, voice_count)
{
```

### Match same arms in night.rs (Tier 2)

```rust
// Before — 10 separate arms, many with identical values:
Self::MainTorpRadar => 115.0,
Self::DdGunTorpRadar | Self::DdGunTorpRadar2 => 115.0,
Self::TorpTorpTorp => 122.0,
Self::DdTorpDrumLookout | Self::DdTorpDrumLookout2 => 122.0,
// After — verified and merged:
Self::MainTorpRadar | Self::DdGunTorpRadar | Self::DdGunTorpRadar2 => 115.0,
Self::TorpTorpTorp | Self::DdTorpDrumLookout | Self::DdTorpDrumLookout2 => 122.0,
```

### Pitfall: match merge editing errors (session history)

After batch-merging match arms, two formatting errors occurred:
1. A closing brace accidentally removed from an `if !matches!` block in `day_cutin.rs`
2. A duplicate `DdTorpDrumLookout` entry remained in `night.rs` because the merge was added to the `TorpTorpTorp` pattern line but the original standalone entry was not deleted

Both caught by `cargo fmt` failing. Always run `cargo fmt --all` and `cargo clippy --workspace` after batch match merges.

## Related

- Plan: `docs/plans/2026-05-24-002-chore-clippy-warnings-housekeeping-plan.md`
- Predecessor: `docs/plans/archive/2026-05-07-002-fix-route-re-export-clippy-warnings-plan.md` (9 warnings, bootstrap crate)
- Predecessor: `docs/plans/archive/2026-05-15-001-refactor-rust-best-practices-violations-plan.md` U1 (2 warnings, gameplay crate)
