---
title: "Audit config: cargo-audit ignore rules with documented rationale"
date: 2026-06-22
category: conventions
module: tooling
problem_type: convention
component: tooling
severity: medium
applies_when:
  - "Running cargo audit on the workspace"
  - "Adding or modifying .cargo/audit.toml ignore rules"
  - "Resolving RUSTSEC advisories in Cargo.lock"
tags: [cargo-audit, security, advisories, rustls-webpki, rand, indicatif, audit-toml]
related_components: [emukc_app]
---

# Audit config: cargo-audit ignore rules with documented rationale

## Context

`cargo audit` flags advisories in the dependency graph. Some are real and
must be fixed by upgrading; others are false positives (a dependency pulled
but never compiled) or unpatchable (a transitive pin with no upstream fix).
This convention records which advisories are suppressed, why, and which must
stay visible.

## Guidance

The following conventions hold for `.cargo/audit.toml` and the dependency
graph:

### Suppressible false positives

- **RUSTSEC-2023-0071 (rsa) SHALL be ignored.** `sqlx-mysql` depends on `rsa`
  but is a `Cargo.lock` residual not present in the compile graph. The ignore
  entry SHALL include a note explaining it is a false positive due to unused
  `sqlx-mysql`. After configuration, `cargo audit` SHALL NOT report it.

### Must-fix via upgrade

- **rustls-webpki ≥ 0.103.13.** The workspace SHALL resolve `rustls-webpki`
  to ≥ 0.103.13, eliminating RUSTSEC-2026-0104, RUSTSEC-2026-0098, and
  RUSTSEC-2026-0099. The build SHALL complete without errors after the
  upgrade.
- **unicode-segmentation ≥ 1.13.2.** SHALL be resolved past the yanked
  version, eliminating the yanked warning.
- **indicatif ≥ 0.18.** The workspace SHALL specify `indicatif = "0.18"` or
  higher, resolving `number_prefix` (RUSTSEC-2025-0119) via `unit_prefix` in
  the upstream 0.18 release. Progress bars SHALL still function correctly.
- **rand 0.9.x ≥ 0.9.3 and 0.10.x ≥ 0.10.1.** SHALL be resolved to patched
  versions, eliminating RUSTSEC-2026-0097 for those version lines.

### Accepted-and-suppressed (unpatchable)

- **rand 0.8.x (RUSTSEC-2026-0097) SHALL be ignored with documented
  blocker.** `rand` 0.8.5 is pulled by `tera` and `phf_generator`, has no
  available patch (the 0.8.x line is unpatched), and the unsound condition is
  unreachable because EmuKC uses `tracing` rather than `log::set_logger`. The
  ignore entry SHALL explain this. After configuration, the rand 0.8.x
  warning SHALL NOT appear.

### Never suppressed

- **RUSTSEC-2026-0097 (general) SHALL remain visible** as an informational
  warning for any version line not covered by an upgrade or the documented
  0.8.x ignore. No blanket ignore SHALL hide patched-but-not-yet-upgraded
  occurrences.

## Why This Matters

Silent ignore rules hide real vulnerabilities; blanket upgrades break the
build. Recording each advisory's disposition (fix / false-positive /
accepted-unpatchable) turns the audit into a trustworthy signal and makes a
future contributor's `cargo audit` output actionable.

## When to Apply

- When `cargo audit` reports a new advisory.
- When editing `.cargo/audit.toml`.
- When upgrading a dependency that resolves an advisory.

## Examples

`.cargo/audit.toml` shape:

```toml
[advisories]
ignore = [
    # RUSTSEC-2023-0071: rsa — false positive; sqlx-mysql not in compile graph.
    "RUSTSEC-2023-0071",
    # RUSTSEC-2026-0097: rand 0.8.x — unpatchable; pinned by tera/phf_generator;
    # unsound condition unreachable (EmuKC uses tracing, not log::set_logger).
    "RUSTSEC-2026-0097",
]
```

## Related

- `.cargo/audit.toml` — the configuration this convention governs.
- `Cargo.lock` — the resolutions the upgrade rules enforce.
