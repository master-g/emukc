## Context

The root package currently defines four Cargo examples, but all four source files live under `tests/`:

- `Cargo.toml` -> `tests/model/load.rs`
- `Cargo.toml` -> `tests/bootstrap/download.rs`
- `Cargo.toml` -> `tests/dump_fs_tree/dump_tree.rs`
- `Cargo.toml` -> `tests/kache/test.rs`

At the same time, `tests/README.md` presents `tests/` as the integration-test area, centered on `tests/gameplay_tests.rs`, `tests/gameplay_tests/**`, and `tests/fixtures/**`. That means the directory has two different responsibilities: executable examples and test code.

This change does not touch gameplay traits such as `SortieOps`, `QuestOps`, or `MaterialOps`, and it does not change any KCSAPI handler or route group. The affected surface is repository structure, Cargo target metadata, and contributor-facing documentation.

## Goals / Non-Goals

**Goals:**
- Make `examples/` the single home for runnable Cargo examples in the root crate.
- Keep `tests/` limited to integration tests, fixtures, and test-specific support files.
- Preserve existing example target names so current `cargo run --example ...` workflows continue to work.
- Update documentation so contributors can tell where to place a new test versus a new example.

**Non-Goals:**
- Changing example runtime behavior, arguments, or logging.
- Changing integration-test coverage or fixture contents except for paths that must be updated after moving files.
- Changing any crate boundaries, gameplay traits, database entities, or KCSAPI handlers.

## Decisions

### 1. Introduce a top-level `examples/` directory for root-crate examples

**Decision:** Move the current example entrypoints from `tests/...` into `examples/...` and repoint the existing `[[example]]` entries in `/Users/mg/github/emukc/Cargo.toml`.

**Rationale:** Cargo already treats `examples/` as the conventional location for runnable examples. Aligning with that convention makes repository intent obvious and reduces cognitive overhead for contributors.

**Alternative considered:** Keep files under `tests/` and rely on comments or README updates. Rejected because the folder would still be semantically mixed, and Cargo metadata would continue pointing examples at a test namespace.

### 2. Preserve example names and Cargo UX

**Decision:** Keep existing example names such as `bootstrap_download`, `dump_tree`, `kache_test`, and `model_loader`; only change the source paths.

**Rationale:** The user-visible interface is the Cargo example name, not the source file path. Preserving names avoids unnecessary churn in local scripts, docs, and contributor habits.

**Alternative considered:** Rename example targets while moving them. Rejected because it couples a layout cleanup with command-surface churn for no functional benefit.

### 3. Keep `tests/` focused on test artifacts only

**Decision:** After the move, `tests/` will contain integration tests (`tests/gameplay_tests.rs`, `tests/gameplay_tests/**`), fixtures (`tests/fixtures/**`), and test helpers only. No standalone example binaries should remain there.

**Rationale:** A clean boundary simplifies future maintenance: `cargo test` assets live in `tests/`, `cargo run --example ...` assets live in `examples/`.

**Alternative considered:** Split examples into a nested `tests/examples/` subtree. Rejected because it still places examples inside the test namespace and weakens the boundary this change is meant to establish.

### 4. Update docs at the same time as layout

**Decision:** Refresh `tests/README.md` and any nearby guidance that currently implies or demonstrates the old mixed layout.

**Rationale:** Moving files without updating docs would leave contributor guidance inconsistent and recreate the same confusion that prompted the change.

**Alternative considered:** Defer docs cleanup to a later follow-up. Rejected because documentation is part of the contract for where new tests/examples belong.

## Risks / Trade-offs

- `[Path drift after move]` -> Example files may rely on relative paths or comments that mention the old `tests/` location. Mitigation: verify each moved example for path assumptions and update comments/docs together with the move.
- `[Incomplete cleanup]` -> Cargo metadata could be updated while stale references to `tests/...` remain in docs or scripts. Mitigation: search for old paths and update all matches as part of the same change.
- `[Over-scoping]` -> It is easy to turn a layout cleanup into a broader test refactor. Mitigation: keep the change limited to file moves, path updates, and documentation needed to preserve current behavior.

## Migration Plan

1. Create `examples/` and move the four current example source files into it.
2. Update `/Users/mg/github/emukc/Cargo.toml` `[[example]]` paths to point at the new locations.
3. Update `/Users/mg/github/emukc/tests/README.md` to describe `tests/` as test-only and reference `examples/` for runnable samples.
4. Search for stale references to `tests/bootstrap`, `tests/model`, `tests/dump_fs_tree`, and `tests/kache` and fix them.
5. Verify both `cargo test` and representative `cargo run --example ...` commands still work.

Rollback is straightforward: move the files back to `tests/` and restore the previous `Cargo.toml` paths if an unexpected path dependency appears.

## Open Questions

- Should the root `README.md` gain a short “Examples” section now, or is `tests/README.md` plus Cargo metadata sufficient for this change?
- Do any local-only scripts outside the tracked repository still assume the old `tests/...` paths for examples?
