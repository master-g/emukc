---
name: emukc-scrape-wikiwiki-mapdata
description: Parse wikiwiki.jp KanColle map pages (cached HTML) into structured WikiwikiMapCatalog JSON for the emukc project. Use this skill when the user asks to update wikiwiki map data, re-parse map HTML, or regenerate the wikiwiki_map_catalog.json asset. Trigger explicitly — this skill does not auto-activate.
---

# EmuKC Wikiwiki Map Data Scraper

This skill reads cached wikiwiki.jp HTML pages and outputs structured
`WikiwikiMapCatalog` JSON that the emukc Rust pipeline consumes.

## What This Skill Does

1. Reads HTML files from `.data/temp/wikiwiki_map/pages/` (downloaded by `cargo run -- wikiwiki-map sync`)
2. Extracts map topology, routing rules, enemy fleets, and ship drops from wikiwiki's HTML tables
3. Outputs a single `WikiwikiMapCatalog` JSON file matching the Rust type at `crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs`

The output JSON is consumed by `cargo run -- wikiwiki-map normalize --from-agent-json <path>`
which runs the Rust conversion layer (`into_map_catalog()`) to produce the final
`MapCatalog` used at runtime.

## Prerequisites

- HTML pages must already be downloaded: `cargo run -- wikiwiki-map sync`
- The manifest file `start2.json` (ship names→IDs, ship type mappings) should be available for reference

## Input

- **HTML files**: `.data/temp/wikiwiki_map/pages/<map-name>` (e.g., `1-1`, `1-2`, `2-3`)
- **Filename format**: `<maparea_id>-<mapinfo_no>` (e.g., `1-2` = map area 1, map 2)
- **HTML source**: wikiwiki.jp map pages like `https://wikiwiki.jp/kancolle/鎮守府海域/1-2`

## Output

A single JSON file with the `WikiwikiMapCatalog` structure. See
`reference/catalog-schema.md` for the full schema and `reference/map-example.json`
for a worked example.

The output file should be saved to `.data/temp/wikiwiki_map/wikiwiki_catalog.json`.

## Extraction Instructions

### Step 1: Parse map metadata

From the HTML page, extract:

- **Map name**: The Japanese map name (e.g., `南西諸島沖` for 1-2)
- **Map ID**: Derive from filename — `1-2` → `maparea_id: 1, mapinfo_no: 2, map_id: 12`
- **Source URL**: The wikiwiki page URL

### Step 2: Parse route table (分岐表)

wikiwiki pages contain a route/branching table with columns:

- **分岐点** (Branch point): source node label (e.g., `A`)
- **ルート** (Route): target node label(s) (e.g., `B/C`)
- **移動条件** (Movement condition): natural language routing rule (e.g., `艦隊サイズ4隻以下`)

**Important:** Some pages have multiple tables. The route table has `分岐点` in the
header row. Skip any `ルート分岐表記` table — that is a notation guide explaining
ship type abbreviations, not actual routing rules.

Routes can have multiple branch points in one table (e.g., Start branches to A/C,
then A branches to D/E, etc.). Extract every row.

**Implicit routes:** If a route table only shows branching at node A but the
topology implies Start → A, do NOT add an explicit routing rule for Start → A.
The conversion layer infers start targets from topology. Only output routing
rules that appear in the wikiwiki table.

**Non-battle nodes:** Some nodes have `戦闘なし` (no battle) or `アイテム獲得`
(item acquisition). These nodes have `is_battle: false` and `is_boss: false`.

Extract each row as a routing rule with:

- `from_label`: source node label
- `to_label`: target node label
- `predicate`: parsed RoutePredicate (see below)
- `probability_pct`: if the route has a percentage (e.g., `約60%`)
- `raw_text`: the original Japanese condition text

### Step 3: Parse enemy table (敵影表)

wikiwiki pages contain enemy encounter tables with:

- Node label (e.g., `A`, `B`, `C：ボス`)
- Enemy fleet compositions (ship names)
- Formation (陣形: 単縦, 複縦, 輪形, 梯形, 陣形)

**Pattern aliasing:** Some patterns say `パターンNと同じ` or `パターンNと同編成`
(same as pattern N). Copy the referenced pattern's ship composition into the
aliasing pattern — do not leave it empty.

**Non-battle nodes:** Nodes marked `戦闘なし` (no battle), `戦闘回避` (battle
avoided), or `アイテム獲得` (item acquisition) should NOT appear in
`enemy_fleets`. They are nodes with `is_battle: false`.

For each battle node, extract:

- `node_label`: the node label (A, B, etc.)
- `is_boss`: true if this is the boss node (marked as `ボス`)
- `compositions`: list of enemy compositions, each with ship names

**Enemy ship IDs:** Generic enemy ships (駆逐イ級, 軽巡ホ級, etc.) have IDs in
the 1501–1599 range. If you cannot resolve the exact ID, use `ship_id: 0` and
put the raw name in `raw_ship_names`. The conversion pipeline clears
`raw_ship_names` at runtime; they are for human verification only.

### Step 4: Parse drop table (ドロップ表)

If the page has a drop/reward table, extract ship drops per node.

### Step 5: Assign cell numbers (BFS)

Assign sequential cell numbers starting from `Start = 0`:

1. Start node = cell 0
2. Perform BFS from Start following the routing graph
3. Each node gets the next sequential number in BFS visit order
4. The Start node itself is cell 0

Example for a linear map Start → A → B(boss):

- Start = 0, A = 1, B = 2

Example for map 1-2:

```
Start → A, Start → B
A → D, A → C
C → D
D → E(boss)
```

BFS: Start=0, A=1, B=2, D=3, E=4, C=5

### Step 6: Build output JSON

Assemble the `WikiwikiMapCatalog` JSON following the schema. See
`reference/catalog-schema.md` for field-by-field documentation.

## RoutePredicate Mapping

wikiwiki route conditions are written in Japanese natural language. Map them to
`RoutePredicate` variants:

| Japanese pattern | RoutePredicate variant |
|---|---|
| (no condition / always) | `Always` |
| `艦隊サイズN隻` / `N隻編成` | `FleetSize { op: Eq, value: N }` |
| `艦隊サイズN隻以上` | `FleetSize { op: Gte, value: N }` |
| `艦隊サイズN隻以下` | `FleetSize { op: Lte, value: N }` |
| `駆逐艦N隻以上` | `ShipTypeCount { ship_types: [2], op: Gte, value: N }` |
| `旗艦が駆逐艦` | `FlagshipShipType { ship_types: [2] }` |
| `高速統一` | `Speed { class: Fast }` |
| `低速艦含まない` | `Speed { class: Fast }` (negative form) |
| 索敵値N以上 | `LoS { formula: null, op: Gte, value: N }` |
| ドラム缶N個以上 | `DrumCanisterCount { op: Gte, value: N }` |
| Conditions with `かつ` (AND) | `And [pred1, pred2]` |
| Conditions with `または` (OR) | `Or [pred1, pred2]` |
| `補給艦を含む` | `ContainsShipType { ship_types: [22] }` |
| `水母を含む` | `ContainsShipType { ship_types: [16] }` |
| `空母系を含む` | `ContainsShipType { ship_types: [7, 11, 18] }` |
| `海防艦N隻以上` | `ShipTypeCount { ship_types: [1], op: Gte, value: N }` |
| `(駆逐+海防)N隻以上` | `ShipTypeCount { ship_types: [2, 1], op: Gte, value: N }` |
| `パターンNと同じ` | (alias — copy the referenced pattern's composition) |
| Conditions with `以外` (NOT) | `Not(pred)` |

Ship type IDs (common):

- DD (駆逐) = 2, CL (軽巡) = 3, CT (練巡) = 21, CVL (軽空母) = 7
- CA (重巡) = 5, CAV (航巡) = 6, BB (戦艦) = 8, BBV (航戦) = 10
- CV (空母) = 11, SS (潜水) = 13, SSV (潜水母艦) = 14, AV (水母) = 16
- AS (潜水母艦) = 20, AP (補給) = 22, AV (揚陸) = 17, LST = 18

When you cannot confidently parse a condition, use:

```json
{ "Unknown": { "raw_text": "<original Japanese text>" } }
```

Do not guess — an `Unknown` predicate is valid and will be reviewed by a human.

## Quality Checks

Before outputting the JSON:

1. Every node in the routing graph has a cell_no (BFS-assigned)
2. Boss node has `is_boss: true`
3. All routing rule `from_cell_no`/`to_cell_no` reference valid node cell_nos
4. Enemy fleet `cell_no` matches a battle node's cell_no
5. Ship names that could not be resolved to ship_id use `raw_ship_names` field

## Limitations

- This skill does NOT download HTML — use `cargo run -- wikiwiki-map sync` first
- This skill does NOT run the Rust conversion — use `cargo run -- wikiwiki-map normalize --from-agent-json <path>` after
- Event/operation maps (gauge type, TP bars, multiple phases) may have multiple variants — extract each variant separately
- Some older wikiwiki pages may use different table layouts — adapt accordingly
