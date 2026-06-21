# openspec Sunset Migration Log

Records the fate of every `openspec/specs/<cap>/spec.md` during the
openspec → ce-compound-engineering migration (plan:
`docs/plans/2026-06-22-001-chore-openspec-to-ce-compound-migration-plan.md`).

**Triage rule (plan KTD2):** a spec is kept-and-migrated if it documents an
invariant not already enforced by a passing test or the type system, AND the
capability is still live. Otherwise it is dropped with a logged reason.

**Outcome:** all 37 specs kept. None dropped — every spec describes a live
subsystem (core gameplay, battle engine, or bootstrap/cache/decoder tooling),
and none is subsumed by the 3 pre-existing `docs/solutions/` docs. Living
contracts are demoted to captured knowledge per decision D1; the
machine-enforcement layer is retired with openspec.

Destination directories follow the plan (KTD1): `architecture-patterns/` for
core capability + battle contracts, `conventions/` for cross-cutting rules,
`best-practices/` for infra/tooling knowledge.

| # | Spec | Fate | Destination |
| --- | --- | --- | --- |
| 1 | sortie | keep | `docs/solutions/architecture-patterns/sortie.md` |
| 2 | material | keep | `docs/solutions/architecture-patterns/material.md` |
| 3 | quest | keep | `docs/solutions/architecture-patterns/quest.md` |
| 4 | fleet | keep | `docs/solutions/architecture-patterns/fleet.md` |
| 5 | map-unlock | keep | `docs/solutions/architecture-patterns/map-unlock.md` |
| 6 | map-data-authority | keep | `docs/solutions/architecture-patterns/map-data-authority.md` |
| 7 | user-lifecycle | keep | `docs/solutions/architecture-patterns/user-lifecycle.md` |
| 8 | useitem-response | keep | `docs/solutions/architecture-patterns/useitem-response.md` |
| 9 | equipment-improvement-bonus | keep | `docs/solutions/architecture-patterns/equipment-improvement-bonus.md` |
| 10 | night-battle-sinking-protection | keep | `docs/solutions/architecture-patterns/night-battle-sinking-protection.md` |
| 11 | battle-damage-foundation | keep | `docs/solutions/architecture-patterns/battle-damage-foundation.md` |
| 12 | battle-kouku-stage3 | keep | `docs/solutions/architecture-patterns/battle-kouku-stage3.md` |
| 13 | battle-sim-params | keep | `docs/solutions/architecture-patterns/battle-sim-params.md` |
| 14 | battle-crate-docs | keep | `docs/solutions/architecture-patterns/battle-crate-docs.md` |
| 15 | rng-facade | keep | `docs/solutions/architecture-patterns/rng-facade.md` |
| 16 | balance-defaults-policy | keep | `docs/solutions/conventions/balance-defaults-policy.md` |
| 17 | audit-config | keep | `docs/solutions/conventions/audit-config.md` |
| 18 | test-example-layout | keep | `docs/solutions/conventions/test-example-layout.md` |
| 19 | rules-default-strategy | keep | `docs/solutions/conventions/rules-default-strategy.md` |
| 20 | bootstrap-guide | keep | `docs/solutions/best-practices/bootstrap-guide.md` |
| 21 | cli-progress | keep | `docs/solutions/best-practices/cli-progress.md` |
| 22 | progress-logging-helper | keep | `docs/solutions/best-practices/progress-logging-helper.md` |
| 23 | populate-error-classification | keep | `docs/solutions/best-practices/populate-error-classification.md` |
| 24 | material-concurrency | keep | `docs/solutions/best-practices/material-concurrency.md` |
| 25 | cache-list-dedup | keep | `docs/solutions/best-practices/cache-list-dedup.md` |
| 26 | cache-make-list-versioning | keep | `docs/solutions/best-practices/cache-make-list-versioning.md` |
| 27 | cache-manifest-integration | keep | `docs/solutions/best-practices/cache-manifest-integration.md` |
| 28 | decoder-cachelist-comparison | keep | `docs/solutions/best-practices/decoder-cachelist-comparison.md` |
| 29 | decoder-coverage-assets | keep | `docs/solutions/best-practices/decoder-coverage-assets.md` |
| 30 | decoder-first-cachelist-pipeline | keep | `docs/solutions/best-practices/decoder-first-cachelist-pipeline.md` |
| 31 | decoder-rule-semantics | keep | `docs/solutions/best-practices/decoder-rule-semantics.md` |
| 32 | decoder-sound-rules | keep | `docs/solutions/best-practices/decoder-sound-rules.md` |
| 33 | pathrules-loading | keep | `docs/solutions/best-practices/pathrules-loading.md` |
| 34 | pathrules-makelist-integration | keep | `docs/solutions/best-practices/pathrules-makelist-integration.md` |
| 35 | manifest-damage-variants | keep | `docs/solutions/best-practices/manifest-damage-variants.md` |
| 36 | resource-manifest | keep | `docs/solutions/best-practices/resource-manifest.md` |
| 37 | web-asset-bootstrap | keep | `docs/solutions/best-practices/web-asset-bootstrap.md` |

**Dropped: 0.** No spec was found obsolete or already covered.
