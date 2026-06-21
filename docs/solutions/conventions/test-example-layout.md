---
title: "Test/example layout: tests/ is test-only, examples/ is runnable samples"
date: 2026-06-22
category: conventions
module: root
problem_type: convention
component: testing_framework
severity: medium
applies_when:
  - "Adding a runnable sample program or an integration test"
  - "Reviewing where a new test/example file should land"
  - "Editing root Cargo.toml example targets"
tags: [tests, examples, cargo, layout, fixtures, contributor-guide]
related_components: []
---

# Test/example layout: tests/ is test-only, examples/ is runnable samples

## Context

Mixing runnable Cargo examples into `tests/` blurs the boundary between
integration tests and sample programs, making it unclear which command runs
which artifact. This convention keeps the two directories cleanly separated
so contributors always know where a file belongs and how to run it.

## Guidance

The following conventions hold for repository layout:

- **Examples live under `examples/`.** Runnable root-crate Cargo examples
  (`model_loader`, `bootstrap_download`, `dump_tree`, `kache_test`, etc.)
  SHALL live under `examples/`, and the root package metadata SHALL reference
  those files from `Cargo.toml` without changing the public example names.
  Relocating an example SHALL NOT rename its target.
- **`tests/` is test-only.** The repository SHALL reserve `tests/` for
  integration tests, fixtures, and test-only support code. Standalone runnable
  examples SHALL NOT be placed there. A contributor inspecting or adding files
  under `tests/` SHALL find only test entrypoints, test modules, fixtures, or
  test helpers.
- **Guidance distinguishes the two.** Contributor-facing documentation SHALL
  describe the boundary between `tests/` and `examples/` and SHALL preserve the
  commands used to run tests versus examples. The documentation SHALL direct
  runnable samples to `examples/`, direct integration tests to `tests/`, and
  keep `cargo test` and `cargo run --example ...` commands unambiguous.

## Why This Matters

When a sample program is misfiled in `tests/`, a contributor running
`cargo test` may execute slow bootstrap/download code unintentionally, or fail
to find the sample when looking in `examples/`. The split keeps `cargo test`
fast and focused, and makes sample programs discoverable via
`cargo run --example`.

## When to Apply

- When adding any new runnable sample (→ `examples/`).
- When adding any integration test or fixture (→ `tests/`).
- When editing root `Cargo.toml` example targets.

## Examples

```toml
# Cargo.toml — examples live under examples/, names unchanged
[[example]]
name = "model_loader"
path = "examples/model_loader.rs"
```

```text
tests/        # integration tests + fixtures only
examples/     # runnable sample programs
```

## Related

- `CLAUDE.md` § Build & Development Commands — the `cargo test` vs
  `cargo run --example` commands this layout keeps unambiguous.
