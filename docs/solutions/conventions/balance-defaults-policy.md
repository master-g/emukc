---
title: "Balance defaults policy: stock behavior unless explicitly justified"
date: 2026-06-22
category: conventions
module: emukc_model
problem_type: convention
component: documentation
severity: high
applies_when:
  - "Changing a numeric Default value in crates/emukc_model/src/codex/"
  - "Reviewing a commit that touches game_config.rs or balance structs"
  - "Adding a boolean QoL Default"
tags: [balance, defaults, game-config, codex, xp-boost, regression-test]
related_components: [emukc_model]
---

# Balance defaults policy: stock behavior unless explicitly justified

## Context

`emukc_model::codex` holds the game's balance knobs (XP multipliers, repair
times, material caps). A silent flip of a numeric default — e.g., an XP
multiplier left at a debug-time 250× — changes the entire game's feel without
leaving a trace. This convention binds every default to stock KanColle
behavior and makes any deviation an explicit, reviewable, tested act.

## Guidance

The following conventions hold for balance defaults:

### Numeric defaults match stock

- **No-op multipliers by default.** The `Default` impl of every numeric balance
  struct in `crates/emukc_model/src/codex/game_config.rs` SHALL produce values
  matching stock KanColle behavior, except where explicitly documented.
  Numeric multipliers (XP boost, time factor, cost factor) SHALL default to
  `1.0` (the no-op multiplier) unless a proposal explicitly justifies a
  different baseline.
- **`ExpConfig` no XP boost.** `ExpConfig::default()` SHALL have
  `ct_exp_boost == 1.0` and `practice_exp_boost == 1.0`.
- **`DockingConfig` no adjustment.** `DockingConfig::default()` SHALL have
  `time_factor == 1.0` and `cost_factor == 1.0`.
- **Regression test enforces it.** A test in
  `crates/emukc_model/src/codex/game_config.rs` SHALL assert each default
  value; a silent future flip SHALL fail `cargo test -p emukc_model`.

### Boolean QoL defaults documented

- **Rationale present.** The `Default` impl of `PicturebookConfig` (and any
  future bool-only QoL struct) SHALL carry a docstring explaining the chosen
  defaults, and SHALL state explicitly when the default differs from stock
  KanColle (e.g., EmuKC is single-player, so the picture book defaults to
  fully unlocked), and note the override path (`[game.picturebook]` in
  `emukc.config.toml`).

### Change policy

- **Dedicated commit.** Any change to a numeric `Default` value in
  `crates/emukc_model/src/codex/` SHALL be in its own commit, separate from
  infrastructure or refactor work.
- **Commit prefix.** The commit SHALL be prefixed `feat(balance):` (new
  behavior) or `chore(balance):` (value tuning).
- **Previous values in body.** The commit body SHALL list the previous
  value(s).
- **Linked proposal.** The change SHALL be linked to a proposal (now via
  ce-plan).
- **Regression test.** The change SHALL add or update a regression test
  asserting the new value.
- **Pure boolean QoL defaults** (e.g., picture-book unlocks) are exempt from
  the regression-test rule but still subject to the dedicated-commit, prefix,
  previous-values, and proposal rules.

### Discoverability

- **CLAUDE.md section.** A section titled "Balance defaults policy" SHALL be
  present in `CLAUDE.md` enumerating the rules above, so a contributor
  discovers the policy at the project root.
- **Review blocks violations.** A code review observing a numeric default flip
  inside a commit whose subject does not begin with `feat(balance):` or
  `chore(balance):` SHALL block merge until split into a dedicated commit.

## Why This Matters

The `ct_exp_boost = 250.0` incident (silently introduced in `16c112f`, a
bootstrap-rollback fix commit) shipped a 250× Command Ship flagship XP
multiplier with no rationale and no test. The dedicated-commit + prefix +
previous-values + regression-test chain makes such a flip loud, reviewable,
and reversible.

## When to Apply

- Whenever editing a `Default` impl under `crates/emukc_model/src/codex/`.
- During review of any commit touching `game_config.rs`.

## Examples

A balance change commit:

```
chore(balance): reduce ct_exp_boost default to 1.0

Previous: ct_exp_boost = 250.0 (silent debug tweak from 16c112f)
New:      ct_exp_boost = 1.0   (stock no-op)

Users wanting 250× set [exp] ct_exp_boost = 250.0 in emukc.config.toml.
Linked plan: docs/plans/2026-04-20-001-...md
Regression test: game_config.rs::test_exp_config_default.
```

## Related

- `crates/emukc_model/src/codex/game_config.rs` — the `Default` impls this governs.
- `CLAUDE.md` § Balance Defaults Policy — the discoverable copy of this convention.
