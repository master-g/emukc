## Context

`crates/emukc_model/src/codex/game_config.rs` defines the in-memory representation of the `[game]` section of `emukc.config.toml`. Each substruct has a `Default` impl that fires when:

1. The user has no `emukc.config.toml` section for that struct (e.g., no `[exp]` block).
2. A test calls `ExpConfig::default()` directly (most gameplay tests do).
3. A new profile is constructed and the global `Codex::game_cfg` is consulted with no overrides.

Two of the four substructs got their defaults flipped in `16c112f`:

```text
PicturebookConfig {
    unlock_all_ships:     false → true
    unlock_all_slotitems: false → true
}

ExpConfig {
    ct_exp_boost:         1.0 → 250.0
    practice_exp_boost:   1.0  (unchanged)
}
```

Call sites for `ct_exp_boost`:

- `crates/emukc_gameplay/src/game/battle/practice/exp.rs:29` — multiplies practice base XP for CT flagship.
- `crates/emukc_gameplay/src/game/battle/practice/orchestrate.rs:57` — passes through to `calculate_ship_exp`.
- `crates/emukc_gameplay/src/game/practice.rs:223` — same.
- `crates/emukc_gameplay/src/game/sortie.rs:704, 806, 965` — three sortie-side XP grants for CT flagship.
- `crates/emukc_gameplay/src/game/sortie_result.rs:114` — sortie result XP application.

A 250× multiplier in those paths means: a fleet led by Akashi (`stype == 21`) earns ~250× the XP per battle compared to stock KanColle. Level-up cadence becomes minutes instead of weeks. No comment, doc, or release note in the repo flags this.

## Goals / Non-Goals

**Goals:**

- Reset `ExpConfig::default().ct_exp_boost` to `1.0` so fresh profiles match stock behavior.
- Make the `PicturebookConfig` default deliberate: either keep both `true` (with a docstring explaining why) or revert. Decision is asked of the user; default plan is **keep `true` + document**, since the intent (single-player emulator UX) is plausible and revertible.
- Add a CI-enforceable regression test pinning the numeric defaults.
- Establish a written policy that prevents future "drive-by" balance changes inside unrelated commits.

**Non-Goals:**

- Auditing every default in `emukc_model` (material caps, docking factors, etc.). Out of scope; can be a follow-up.
- Adding telemetry / runtime warning when configured values diverge from defaults.
- Restructuring the `[game]` config schema.

## Decisions

### D1. Revert `ct_exp_boost` to `1.0`, do not preserve a deprecation alias

**Decision**: change line 58 to `ct_exp_boost: 1.0,`. No fallback, no warning, no migration alias — the field is configurable, and any user who actually wanted 250× had to either (a) accept the silent gift or (b) override it explicitly. Group (b) is unaffected by the revert; group (a) experiences a one-time correction.

**Alternative considered**: emit a `tracing::warn!` if `ct_exp_boost > 10.0` at server startup. Rejected — the user's config is authoritative; the server should not nag about valid values.

### D2. `PicturebookConfig` defaults: keep `true`, add docstring

**Decision**: leave `unlock_all_ships: true, unlock_all_slotitems: true` but add a docstring on the `Default` impl reading: "EmuKC is a single-player emulator; the picture book defaults to fully unlocked because gating it behind in-game progress is not the typical user expectation. Override in `[game.picturebook]` to opt into the original gating."

**Alternative considered**: revert to `false, false`. Rejected as a separate decision the user can make later — the current behavior has been live for some time and the player-experience argument is plausible.

If the user disagrees with this decision, the change to revert is a one-line edit in the same file; it does not warrant its own openspec proposal.

### D3. Regression test pins numeric defaults

**Decision**: add a `#[cfg(test)]` module to `game_config.rs` with three asserts:

```rust
assert_eq!(ExpConfig::default().ct_exp_boost, 1.0);
assert_eq!(ExpConfig::default().practice_exp_boost, 1.0);
assert_eq!(DockingConfig::default().time_factor, 1.0);
assert_eq!(DockingConfig::default().cost_factor, 1.0);
```

Picturebook defaults are explicitly NOT pinned, since the user-facing decision in D2 may flip later. The non-numeric (bool) defaults are documented but not asserted.

### D4. Repository policy in CLAUDE.md

**Decision**: add a section to the project root `CLAUDE.md` titled "Balance defaults policy":

> Any change to a `Default` impl in `crates/emukc_model/src/codex/` that affects gameplay numerics (XP multipliers, drop rates, repair times, material caps) MUST:
>
> 1. Be in its own commit, separate from infrastructure or refactor work.
> 2. Use commit prefix `feat(balance):` for new behavior or `chore(balance):` for value tuning.
> 3. List the previous value(s) in the commit body.
> 4. Update or reference an openspec proposal under `openspec/changes/`.
> 5. Add or update a regression test asserting the new value, so future accidental flips fail CI.
>
> Boolean defaults (e.g., picture-book unlocks) are exempt from rule 5 but still subject to rules 1-4.

This is enforced by code review, not tooling.

## Risks / Trade-offs

- [User experience regression for accidental beneficiaries] → users currently enjoying 250× XP without explicitly configuring it will see normal rates after upgrade. Mitigation: explicit CHANGELOG entry; commit message states the previous value.
- [Policy not enforceable by tooling] → the CLAUDE.md rule depends on contributor discipline. Mitigation: the regression test in D3 catches the most damaging class of drift (numeric XP / repair / material) automatically.
- [Picturebook decision deferred] → leaving the bool defaults at `true` accepts the implicit choice from `16c112f`. If the user disagrees, that is a one-line follow-up.

## Migration Plan

1. Edit `crates/emukc_model/src/codex/game_config.rs`: change line 58 to `ct_exp_boost: 1.0,`. Add docstring on `PicturebookConfig::default`.
2. Add the regression test module at the bottom of the same file.
3. Run `cargo test -p emukc_model` clean.
4. Run `cargo test --workspace` to confirm no gameplay test was implicitly relying on 250× XP.
5. Update root `CLAUDE.md` with the "Balance defaults policy" section.
6. Add a short CHANGELOG entry under "Breaking changes" noting the revert.

Rollback: revert the single-line value back to `250.0` if a user explicitly wants the prior behavior — but the configurable knob (`emukc.config.toml [exp] ct_exp_boost = 250.0`) is the supported path.
