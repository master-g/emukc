## 1. Revert ct_exp_boost default

- [x] 1.1 Edit `crates/emukc_model/src/codex/game_config.rs:58` — change `ct_exp_boost: 250.0,` to `ct_exp_boost: 1.0,`.
- [x] 1.2 Run `cargo test --workspace` and confirm no gameplay test was implicitly depending on the 250× value (e.g., expected XP values in practice/sortie tests).
- [x] 1.3 If a test does rely on 250×, update the test to construct an explicit `ExpConfig { ct_exp_boost: 250.0, ..Default::default() }` rather than depending on the default.

## 2. Document PicturebookConfig default

- [x] 2.1 Edit `crates/emukc_model/src/codex/game_config.rs`: add a doc comment on `impl Default for PicturebookConfig`. Suggested wording: `/// EmuKC is a single-player emulator; the picture book defaults to fully unlocked because gating it behind in-game progress is not the typical user expectation. Override in [game.picturebook] in emukc.config.toml to opt into the original gating.`
- [x] 2.2 Verify the docstring is visible by running `cargo doc --workspace --no-deps --open` and navigating to `PicturebookConfig::default`.

## 3. Add regression test

- [x] 3.1 Append a `#[cfg(test)]` module at the bottom of `crates/emukc_model/src/codex/game_config.rs`. The module SHALL contain three test functions: `exp_config_default_pins_no_boost`, `docking_config_default_pins_no_adjustment`, and `picturebook_config_default_documented` (the third asserts only that the field types compile, not the bool values).
- [x] 3.2 The first test asserts `ExpConfig::default().ct_exp_boost == 1.0` and `ExpConfig::default().practice_exp_boost == 1.0`.
- [x] 3.3 The second test asserts `DockingConfig::default().time_factor == 1.0` and `DockingConfig::default().cost_factor == 1.0`.
- [x] 3.4 Run `cargo test -p emukc_model` and confirm the new tests pass.

## 4. Add policy section to CLAUDE.md

- [x] 4.1 Edit the project root `/Users/mg/github/emukc/CLAUDE.md`. Add a new section titled `## Balance defaults policy` after the existing `## Code Style` section.
- [x] 4.2 Section text SHALL list the five rules from the proposal: dedicated commit, `feat(balance):` / `chore(balance):` prefix, previous-value disclosure in commit body, linked openspec proposal, regression test.
- [x] 4.3 Section text SHALL note the exemption: pure boolean QoL defaults (e.g., picturebook unlocks) are exempt from rule 5 but still bound by rules 1-4.

## 5. CHANGELOG note

- [x] 5.1 If a project CHANGELOG.md exists, append an entry under "Breaking changes" stating the `ct_exp_boost` default revert from 250.0 to 1.0 and the override path. If no CHANGELOG.md exists, skip this task.

## 6. Verification

- [x] 6.1 Run `cargo build --workspace` clean.
- [x] 6.2 Run `cargo test --workspace` clean.
- [x] 6.3 Run `cargo clippy --workspace -- -D warnings` clean.
- [x] 6.4 Run `openspec validate audit-game-config-defaults --strict` clean.
- [ ] 6.5 Manual sanity: start the server with no `[exp]` section in `emukc.config.toml`, run a practice battle with a CT flagship, confirm XP gained matches stock 1× behavior.
