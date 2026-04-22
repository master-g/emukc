## Why

The repository currently mixes runnable examples and actual test assets under `tests/`. `Cargo.toml` points `[[example]]` entries at `tests/...`, while `tests/README.md` documents the folder as integration-test-only, so the current layout obscures intent and makes future maintenance harder.

## What Changes

- Create a first-class `examples/` directory and move the current example binaries out of `tests/`.
- Keep `tests/` scoped to integration tests, fixtures, and test-only support files.
- Update `Cargo.toml` `[[example]]` paths so example names stay stable while their source files live under `examples/`.
- Refresh repository documentation and contributor guidance so adding a new test vs a new example has a clear home and command path.
- Preserve current example behavior and test behavior; this change is about repository layout and developer workflow, not runtime features.

## Capabilities

### New Capabilities
- `test-example-layout`: Define a repository convention where runnable examples live under `examples/` and `tests/` contains only test code, fixtures, and test documentation.

### Modified Capabilities

## Non-goals

- Changing gameplay logic, including traits such as `SortieOps`, `QuestOps`, or `MaterialOps`.
- Changing any KCSAPI route groups such as `api_req_sortie/`, `api_req_quest/`, or `api_start2/`.
- Renaming existing example commands exposed through Cargo.
- Reworking test coverage, fixtures, or bootstrap behavior beyond what is needed to separate folder responsibilities.

## Impact

- **Cargo packaging**: `Cargo.toml` example path entries move from `tests/...` to `examples/...`.
- **Repository layout**: example source files relocate; `tests/` becomes test-only by convention.
- **Developer documentation**: `tests/README.md` and any related docs or comments need to reflect the new boundary.
- **Verification workflow**: `cargo test` and `cargo run --example ...` must continue to work after the move.
