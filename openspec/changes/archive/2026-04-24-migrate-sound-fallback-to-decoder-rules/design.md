## Context

The decoder-first pipeline is already strong on recall and nearly clean on precision, but the sound domain remains mostly owned by legacy Rust logic.

Current state:

- `examples/decoder_cachelist_compare.rs` reports `100%` baseline recall and only `229` candidate-only paths, but still reports `38685` fallback-authored candidate paths.
- The dominant residual is `kcs/sound/*`, which contributes roughly `33786` fallback-authored paths by itself.
- Rust sound generation currently lives under:
  - `crates/emukc_bootstrap/src/make_list/source/kcs/voice.rs`
  - `crates/emukc_bootstrap/src/make_list/source/kcs/kc9997.rs`
  - `crates/emukc_bootstrap/src/make_list/source/kcs/kc9998.rs`
  - `crates/emukc_bootstrap/src/make_list/source/kcs/kc9999.rs`
- Those generators rely on Rust-owned formulas, tables, and `Codex.cache_source` buckets, so even the `Rules` strategy still treats most sound output as fallback-authored.

The decoder side already shows that migration is plausible:

- `main-decoder/out/modules` contains direct `sound.voice.play("9998", ...)` and `playAtRandom("9999", [...])` calls.
- `main-decoder/out/main.decoded.js` exposes `api_voicef` access and many `sound.bgm.play(...)` / `SE.play(...)` call sites.
- `main-decoder/src/audio-resources.ts` already extracts explicit `se`, `bgm`, `titlecall`, tutorial voice stems, and explicit voice paths, but it does not yet model algorithmic `kcs/sound/*` families as decoder-authored rules.

No gameplay trait (`SortieOps`, `MaterialOps`, etc.), database entity scope (`entity::user` / `entity::profile`), or KCSAPI route group needs to change for this work.

## Goals / Non-Goals

**Goals:**

- Move the highest-leverage `kcs/sound/*` cache-list families from Rust fallback logic into decoder-authored rules.
- Preserve the current explicit `audio_resources.json` role for `se` / `bgm` / titlecall / tutorial voice coverage while adding a clear home for algorithmic sound rules.
- Make the sound portion of the `Rules` path measurably decoder-authored in the comparison report.
- Keep existing Rust sound generators available as fallback for unresolved sound families during the migration.

**Non-Goals:**

- Do not switch the default cache-list strategy to `Rules`.
- Do not broaden this change into map, furniture, gauge, or remaining ship/slot cleanup.
- Do not require new Codex fields beyond the existing manifest/shipgraph/cache-source data already loaded today.
- Do not remove the current Rust sound generators in this change.

## Decisions

### D1: Model algorithmic sound generation in `cache_rules.json`, not `audio_resources.json`

Explicit audio path groups and numeric asset lists will remain in `audio_resources.json`, while algorithmic sound families will be modeled in `cache_rules.json` under a new sound-rules section.

That split follows the current system boundary:

- `audio_resources.json` is already good for directly observed explicit assets (`se`, `bgm`, titlecall buckets, tutorial stems).
- `kcs/sound/*` generation is not a flat list problem; it is a semantic rule problem involving:
  - ship voice formulas
  - voice-id families gated by `api_voicef`
  - special CG / repair / special-case voice sets
  - `9997` / `9998` / `9999` family selectors and random-choice buckets

Rationale:

- Algorithmic sound generation belongs next to other semantic rule systems (`shipRules`, `slotRules`) rather than in a list-only asset.
- The `Rules` path already treats `cache_rules.json` as the semantic root bundle.

Alternative considered: extend only `audio_resources.json` with large explicit expansions. Rejected because that would flatten algorithmic behavior into data dumps and would be brittle across script versions.

### D2: Represent sound migration as two subdomains: formula-driven ship voices and bucket-driven special families

The sound rules model will separate:

1. **Formula-driven ship voices**
   - derive `kcs/sound/kc{api_filename}/{voice}.mp3`
   - use existing `ApiMstShipgraph` / ship metadata already present in `Codex`
   - preserve special voice groups such as repair voices and special-CG voice IDs

2. **Bucket-driven special families**
   - `kc9997`
   - `kc9998`
   - `kc9999`
   - random-choice voice groups and special event/duty buckets visible in decoded modules

Rationale:

- These two shapes have different evidence sources and different failure modes.
- Ship voices are deterministic over manifest data plus stable rule metadata.
- `9997` / `9998` / `9999` are more like semantic buckets inferred from decoded call sites.

Alternative considered: force all sound families into one generic `soundRules.entries[]` table. Rejected because it would hide important differences between deterministic formulas and explicit decoder-observed buckets.

### D3: Rust `Rules` generation should consume decoder sound rules first, with existing `kcs` sound modules demoted to fallback

`crates/emukc_bootstrap/src/make_list/source/kcs/mod.rs` will continue to own the implementation boundary for sound generation, but its role changes:

- decoder-authored sound rules produce covered sound paths first
- existing `voice.rs`, `kc9997.rs`, `kc9998.rs`, and `kc9999.rs` remain as fallback emitters for unresolved rule families
- comparison diagnostics classify those fallback-authored sound paths explicitly

Rationale:

- This minimizes churn in the existing make-list structure.
- It preserves a safe fallback path while making progress measurable at the domain level.

Alternative considered: move all sound generation out of `source/kcs/` immediately. Rejected because it would create unnecessary structural churn during a migration-focused change.

### D4: The acceptance signal is fallback reduction in the sound domain, not just unchanged global recall

The comparison loop in `examples/decoder_cachelist_compare.rs` will remain the primary gate, but for this change the key success metric is:

- substantial reduction in sound-domain fallback residuals
- while preserving `100%` baseline recall

The change does not need to eliminate all sound fallback in one shot, but it must materially reduce the `kcs/sound/*` residual and make the remaining unresolved sound families explicit.

Rationale:

- Global recall is already solved.
- The point of this change is to move authority, not just maintain overlap.

Alternative considered: accept any sound-rule work that keeps overlap unchanged. Rejected because it would not ensure that the migration actually reduced the dominant fallback block.

## Risks / Trade-offs

- [Risk] Some `kc9997` / `kc9998` / `kc9999` families may not be fully derivable from decoded `main.js`. → Mitigation: keep unresolved sound rules explicit and preserve current Rust generators as fallback for those families.
- [Risk] Sound-rule schema may become too tied to current decoded call shapes. → Mitigation: model stable semantic categories and formulas rather than raw AST fragments.
- [Risk] A large sound migration could preserve recall but accidentally over-generate sound paths. → Mitigation: use the compare report as the acceptance signal and keep candidate-only deltas visible by prefix.
- [Risk] Mixing explicit audio assets and algorithmic sound rules may confuse future contributors. → Mitigation: document the split clearly in the spec and keep the asset responsibilities distinct.

## Migration Plan

1. Extend decoder schema and extraction so algorithmic sound rule metadata is emitted alongside the existing audio coverage assets.
2. Add Rust serde/loading support for the new sound rule section in `cache_rules.json`.
3. Update `crates/emukc_bootstrap/src/make_list/source/kcs/` so the `Rules` path generates covered sound families from decoder rules first and records unresolved fallback explicitly.
4. Extend the comparison report to show sound fallback reduction and remaining unresolved sound-rule blockers.
5. Re-run the decoder bundle and compare loop, validating that sound fallback residuals drop materially while global recall stays at `100%`.

Rollback strategy:

- Ignore the new decoder sound rule section and fall back entirely to the current Rust sound generators.
- Keep explicit `audio_resources.json` behavior unchanged.
- Since the default strategy is unchanged, rollback does not change user-facing bootstrap defaults.

## Open Questions

- Can `kc9998` abyssal quote coverage be recovered fully from decoded modules, or will part of it remain cache-source fallback in this phase?
- Should random-choice `playAtRandom("9999", [...])` buckets be stored as weighted groups in the rule bundle, or flattened to the union of reachable IDs for cache-list purposes?
- Should sound-rule progress be reported as a dedicated sound-authority metric in the compare output, or is prefix-group residual reporting sufficient for now?
