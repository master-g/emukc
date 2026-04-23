## Why

`decoder_cachelist_compare` shows that the current decoder-driven candidate only covers `34.67%` of the Default cache list overall, even though ship+slot path coverage is already `79.11%`. The remaining gap is split between two concrete problems: sparse ship/slot categories are over-generated because the Rust manifest resolver expands many expressions to universal ID sets, and entire domains such as sound, map, furniture, BGM, voice, and useitem are still absent from decoder-driven generation.

## What Changes

- Add decoder-produced coverage assets that describe directly observable sparse ship/slot subsets and non-ship/slot resource domains needed by cache list generation.
- Extend Rust bootstrap cache-list generation to consume those assets, so decoder-driven generation can both constrain sparse categories and add currently missing audio/UI domains.
- Keep the current comparison example as the validation loop, but make the generated assets and cache-list behavior align with that loop so coverage improvements are measurable after each iteration.

## Non-goals

- Replacing the Greedy strategy or CDN existence probing.
- Claiming pure `main.js` completeness for categories that are only discoverable through runtime API state or remote probing.
- Changing gameplay traits, KCSAPI route behavior, or non-bootstrap server features.
- Removing the existing `resource_manifest.json` workflow before the new assets prove stable.

## Capabilities

### New Capabilities
- `decoder-coverage-assets`: Extract and sync decoder-produced assets for sparse ship/slot subsets plus audio/UI resource domains, with explicit provenance and completeness metadata.

### Modified Capabilities
- `cache-manifest-integration`: Expand cache-list generation so decoder-derived assets can both reduce manifest over-generation for sparse categories and add decoder-driven coverage for currently missing non-ship/slot domains.
- `decoder-cachelist-comparison`: Allow the comparison loop to consume the full decoder output asset set so coverage iteration does not require syncing repo-tracked bootstrap assets on every run.

## Impact

- `main-decoder/src/`: new or expanded extractors for subset metadata, audio resources, and UI resources; pipeline output updates.
- `main-decoder/README.md` and CLI workflow: updated decoder output/sync flow for the new coverage assets.
- `crates/emukc_bootstrap/assets/`: additional decoder-produced JSON assets beyond `resource_manifest.json`.
- `crates/emukc_bootstrap/src/make_list/`: resolver/generator changes to consume new assets and produce broader decoder-driven cache lists.
- `examples/decoder_cachelist_compare.rs`: updated to compare against decoder output assets directly, without changing unrelated bootstrap flows.
