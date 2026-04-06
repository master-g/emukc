# main-decoder

Offline decoder for KanColle's current `main.js`, implemented with **Bun + TypeScript**.

## Inputs

By default the CLI reads:

- `../z/cache/gadget_html5/js/kcs_const.js`
- `../z/cache/kcs2/js/main.js`

## Commands

```bash
bun run decode
bun run check
bun run test
bun run build
```

## Decode outputs

Running `bun run decode` writes artifacts into `./out/`:

- `version.txt`
- `decoder-runtime.js`
- `main.bundle.js`
- `main.decoded.js`
- `summary.json`
- `modules/module-graph.json`
- `modules/hotspot-delta-report.json`
- `battle/battle_protocol_fields.json`
- `battle/battle_resource_rules.json`
- `battle/battle_module_index.json`
- `battle/battle_slot_resource_triggers.json`
- `modules/*.js`

By default `bun run decode` only updates `./out/`.

Use `--sync-battle-assets` when you explicitly want to sync the current battle knowledge assets into:

- `../crates/emukc_bootstrap/assets/battle_protocol_fields.json`
- `../crates/emukc_bootstrap/assets/battle_resource_rules.json`
- `../crates/emukc_bootstrap/assets/battle_module_index.json`
- `../crates/emukc_bootstrap/assets/battle_slot_resource_triggers.json`

The module graph now includes:

- per-module `moduleKind` classification: `game`, `helper`, or `vendor`
- per-module `cleanupTier` classification: `none`, `named-game`, or `priority-body`
- aggregate module-kind counts in `summary.json`
- battle knowledge summaries in `summary.json`, including extracted battle module counts, protocol field counts, and resource-rule counts
- a `topObfuscatedGameModules` list to prioritize readability work on gameplay code first
- shell readability metrics in `summary.json` / `module-graph.json`, including namespace/class shell coverage, structural transform counts, and top structurally transformed modules
- named game hotspot scoring in `summary.json` / `module-graph.json` to prioritize body-level cleanup on the best readability-ROI gameplay modules
- rule-driven named-game local recovery for mechanically recoverable aliases such as `self`, `scene`, `aShip`, `damage`, `rawProgress`, and `listItem`, without relying on a hard-coded hotspot whitelist
- rule-driven priority-body normalization for parse-safe sequence-heavy setup/teardown code, including split `return a, b, value` returns and split comma-chained expression statements
- conservative legacy-only `if (a, b, cond)` test splitting for the small set of modules where it has already proven parse-safe
- per-module `hotspotCleanup.appliedRules` metadata so downstream analysis can tell which cleanup rules actually fired
- a dedicated `modules/hotspot-delta-report.json` artifact with before/after hotspot rankings plus per-module rename/body-normalization deltas for the automatically selected cleanup cohort
- battle protocol field catalogs extracted from battle-related modules such as `RawDayBattleData`, `RawNightBattleData`, `RaigekiData`, and `RaigekiOpeningData`
- battle resource-rule catalogs extracted from modules such as `ShipBanner`, `CutinResourcesPreloadTask`, and battle texture-provider canvases
- a battle module index that tags battle-related decoded modules and ties them to extracted protocol fields and resource rules
- slot-resource trigger catalogs that tie shelling slot ids such as `api_hougeki*.api_si_list[*][*]` to cutin/text resource consumers such as `slot/btxt_flat`
- canonical names for common TypeScript helper aliases such as `__extends`, `__createBinding`, `__setModuleDefault`, `__importStar`, and `__importDefault`
- export-driven shell normalization so namespace wrappers like `SuffixUtil` and class shells like `ShipChoiceView` expose readable exported identities inside the module body
- parse-safe shell AST cleanup, including canonical namespace IIFEs and expanded class-shell `return ... , Ctor` sequences

## CLI options

```bash
bun run decode -- --main ../z/cache/kcs2/js/main.js --const ../z/cache/gadget_html5/js/kcs_const.js --out ./out --max-passes 8
bun run decode -- --sync-battle-assets
```
