## Why

Commit `16c112f` ("fix(bootstrap): detect and handle version rollback in cache population") bundled three independent changes:

1. **Cache version-rollback handling** — the headline fix.
2. **`Kache::get_cached_version()` exposure** — supporting API for (1).
3. **Game-balance default flips** — `PicturebookConfig::default()` switched to `unlock_all_ships: true, unlock_all_slotitems: true`, and `ExpConfig::ct_exp_boost` switched from `1.0` to **`250.0`**.

The third change is **buried** inside an unrelated bootstrap fix and changes runtime gameplay behavior for every fresh profile that does not override these fields in `emukc.config.toml`:

- `unlock_all_ships: true` and `unlock_all_slotitems: true` mean the picture-book is fully unveiled by default. Defensible as a player-experience preference for a self-hosted emulator, but it is a behavior change worth surfacing in a proposal of its own.
- `ct_exp_boost: 250.0` means a Command Ship (`stype == 21` flagship: Akashi, Mizuho, Katori, etc.) yields **250×** the normal sortie/practice XP. This is overwhelmingly likely to be a debug/cheat value committed by accident. The previous default `1.0` matches stock KanColle behavior.

Both changes share the same root-cause: there is currently no policy in the repo for distinguishing **debug-time tweaks** from **intentional balance defaults**, so a value tuned during testing slid into `Default` and shipped silently.

## What Changes

- **Revert `ct_exp_boost` default to `1.0`** in `crates/emukc_model/src/codex/game_config.rs::ExpConfig::default()`. Players who want the boost set it explicitly in `emukc.config.toml`.
- **Document `PicturebookConfig` default rationale**: keep `unlock_all_ships: true, unlock_all_slotitems: true` if intentional, but add a doc comment to the impl explaining why the default differs from stock KanColle (single-player emulator UX choice). Otherwise revert to `false, false`. The decision is captured in `design.md` and surfaced to the user via this proposal.
- **Add a "balance defaults" rule to CLAUDE.md** (or AGENTS.md): any change to a `Default` impl in `emukc_model::codex` that affects gameplay numbers (XP multipliers, drop rates, repair times, material caps) MUST be its own commit with a `feat(balance):` or `chore(balance):` prefix, MUST list the previous value in the commit body, and SHOULD reference an openspec proposal.
- **Add a regression test** asserting `ExpConfig::default().ct_exp_boost == 1.0` and `practice_exp_boost == 1.0` so future drift fails CI.

## Capabilities

### New Capabilities

- `balance-defaults-policy`: defines the contract for game-balance default values: what may be flipped silently, what requires its own commit, and how to test for drift.

### Modified Capabilities

- None. (No existing spec covers `game_config.rs` defaults.)

## Non-goals

- Removing `ct_exp_boost` and `practice_exp_boost` as configurable knobs. They remain configurable; only the `Default` value reverts.
- Auditing every other `Default` impl in `emukc_model`. Future proposals can extend the policy to materials, repair, etc.
- Restructuring the commit `16c112f`. The cache-rollback fix itself is correct; we do not revert the headline change.
- Adding a runtime warning when `ct_exp_boost > 1.0` is configured. The configuration is intentional when set explicitly.

## Impact

- **Affected file**: `crates/emukc_model/src/codex/game_config.rs` only (one-line revert + optional doc comment).
- **New file**: `crates/emukc_model/src/codex/game_config.rs` gains a `#[test]` module with the regression test.
- **Documentation**: `CLAUDE.md` (project root) gains a "Balance defaults policy" section.
- **Behavior change for users with no `[exp]` section in their `emukc.config.toml`**: CT flagship XP returns to 1×. Users who rely on the 250× boost see it disappear; they can restore it explicitly in their config file. A migration note in CHANGELOG calls this out.
- **No DB schema changes, no Codex regeneration, no KCSAPI changes.**
