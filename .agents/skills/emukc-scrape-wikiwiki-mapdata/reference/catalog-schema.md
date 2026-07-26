# WikiwikiMapCatalog JSON Schema

This document describes the JSON structure that the `emukc-scrape-wikiwiki-mapdata`
skill must output. The canonical type definitions are in
`crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs` (Rust structs with
`#[derive(Serialize, Deserialize)]`).

## Top-level: WikiwikiMapCatalog

```json
{
  "maps": {
    "12": { "...WikiwikiMapDefinition }
  }
}
```

- `maps`: object keyed by map_id (string). Map ID = `maparea_id * 10 + mapinfo_no`.
  Example: map 1-2 → map_id `12`.

## WikiwikiMapDefinition

```json
{
  "map_id": 12,
  "maparea_id": 1,
  "mapinfo_no": 2,
  "name": "南西諸島沖",
  "source_url": "https://wikiwiki.jp/kancolle/鎮守府海域/1-2",
  "default_variant": "",
  "variants": {
    "": { "...WikiwikiMapVariantDefinition }
  },
  "overlays": {}
}
```

| Field | Type | Description |
|---|---|---|
| `map_id` | i64 | `maparea_id * 10 + mapinfo_no` (e.g., 1-2 → 12) |
| `maparea_id` | i64 | Map area (1-7 for normal, higher for events) |
| `mapinfo_no` | i64 | Map number within the area |
| `name` | string | Japanese map name from the wikiwiki page title |
| `source_url` | string | Full wikiwiki.jp URL |
| `default_variant` | string | Default variant key (empty string for normal maps) |
| `variants` | object | Variant definitions keyed by variant_key |
| `overlays` | object | Optional overlay data (usually empty for normal maps) |

## WikiwikiMapVariantDefinition

```json
{
  "variant_key": "",
  "nodes": [ { "...WikiwikiNodeDefinition" } ],
  "routing_rules": [ { "...RouteRule" } ],
  "enemy_fleets": [ { "...WikiwikiEnemyFleetDefinition" } ],
  "ship_drops": {},
  "required_defeat_count": null,
  "clear_to_variant_key": null,
  "parse_warnings": []
}
```

| Field | Type | Description |
|---|---|---|
| `variant_key` | string | Variant identifier (empty for normal maps) |
| `nodes` | array | All map nodes with BFS-assigned cell numbers |
| `routing_rules` | array | Route rules between cells |
| `enemy_fleets` | array | Enemy fleet definitions per battle node |
| `ship_drops` | object | Ship drops keyed by cell_no (often empty) |
| `required_defeat_count` | i64? | Boss kill requirement (null if none) |
| `clear_to_variant_key` | string? | Next variant after clearing (null if none) |
| `parse_warnings` | array | Non-fatal warnings (use for ambiguous data) |

## WikiwikiNodeDefinition

```json
{
  "label": "A",
  "cell_no": 1,
  "is_boss": false,
  "is_battle": true
}
```

| Field | Type | Description |
|---|---|---|
| `label` | string | Node label from wikiwiki: `Start`, `A`, `B`, `C`, ... |
| `cell_no` | i64 | BFS-assigned cell number (Start = 0) |
| `is_boss` | bool | True if this is the boss node |
| `is_battle` | bool | True if enemy encounters exist at this node |

### Cell number assignment (BFS)

1. Start node = cell 0
2. BFS traverse the routing graph from Start
3. Each newly visited node gets the next sequential number
4. If the graph is disconnected, remaining nodes get sequential numbers after BFS

## RouteRule

```json
{
  "from_cell_no": 0,
  "to_cell_no": 1,
  "priority": 0,
  "weight": 6000,
  "probability_pct": 60.0,
  "predicate": { "FleetSize": { "op": "Eq", "value": 4 } },
  "raw_text": "艦隊サイズ4隻"
}
```

| Field | Type | Description |
|---|---|---|
| `from_cell_no` | i64 | Source cell (must match a node's cell_no) |
| `to_cell_no` | i64 | Target cell (must match a node's cell_no) |
| `priority` | i64 | Priority (lower = higher priority, 0 = first checked) |
| `weight` | i64? | Probability weight (0-10000, where 10000 = 100%) |
| `probability_pct` | f64? | Probability percentage (0-100) |
| `predicate` | RoutePredicate | Routing condition |
| `raw_text` | string | Original Japanese condition text |

### Weight/Probability calculation

If wikiwiki says `約60%`:
- `probability_pct`: 60.0
- `weight`: 6000 (probability_pct * 100)

If no percentage is given (deterministic route):
- `probability_pct`: null
- `weight`: null

### Priority assignment

Assign sequential priorities starting from 0 within each `from_cell_no` group.
Lower priority = checked first. Routes without conditions (`Always`) get the
highest priority number (checked last as default).

## RoutePredicate

RoutePredicate is a tagged union (Rust enum). In JSON, use `{ "VariantName": { ...fields } }`.

### Variants

#### Always
```json
{ "Always": null }
```
No condition — route is always taken.

#### FleetSize
```json
{ "FleetSize": { "op": "Eq", "value": 4 } }
```
- `op`: `"Eq"`, `"Gte"`, or `"Lte"`
- `value`: fleet ship count

#### ShipTypeCount
```json
{ "ShipTypeCount": { "ship_types": [2], "op": "Gte", "value": 2 } }
```
Count of specific ship types in fleet.

#### FlagshipShipType
```json
{ "FlagshipShipType": { "ship_types": [2, 3] } }
```
Flagship must be one of the listed ship types.

#### ContainsShipType
```json
{ "ContainsShipType": { "ship_types": [2] } }
```
Fleet must contain at least one ship of the listed types.

#### Speed
```json
{ "Speed": { "class": "Fast" } }
```
- `class`: `"Slow"`, `"Fast"`, `"FastPlus"`, `"Fastest"`

#### LoS (Line of Sight)
```json
{ "LoS": { "formula": null, "op": "Gte", "value": 30 } }
```

#### DrumCanisterCount
```json
{ "DrumCanisterCount": { "op": "Gte", "value": 3 } }
```

#### And / Or / Not (combinators)
```json
{ "And": [ { "FleetSize": { "op": "Lte", "value": 4 } }, { "Speed": { "class": "Fast" } } ] }
{ "Or": [ { "...": "..." }, { "...": "..." } ] }
{ "Not": { "FleetSize": { "op": "Gte", "value": 5 } } }
```

#### Unknown (fallback)
```json
{ "Unknown": { "raw_text": "複雑な条件" } }
```
Use when the condition cannot be confidently parsed.

## WikiwikiEnemyFleetDefinition

```json
{
  "node_label": "A",
  "cell_no": 1,
  "battle_kind": 1,
  "formations": [1],
  "compositions": [
    {
      "comp_id": "A-1",
      "weight": 100,
      "ship_ids": [1, 2, 3],
      "formation": 1,
      "raw_ship_names": ["駆逐イ級", "駆逐イ級", "軽巡ホ級"]
    }
  ]
}
```

| Field | Type | Description |
|---|---|---|
| `node_label` | string | Node label (A, B, etc.) |
| `cell_no` | i64 | Cell number matching the node |
| `battle_kind` | i64 | Battle type (0 = normal, 1 = normal battle, 5 = boss) |
| `formations` | array | Possible formations (1=単縦, 2=複縦, 3=輪形, 4=梯形, 5=単横) |
| `compositions` | array | Enemy fleet compositions |

### EnemyComposition

| Field | Type | Description |
|---|---|---|
| `comp_id` | string | Unique ID for this composition (e.g., `"A-1"`, `"Boss-1"`) |
| `weight` | i64 | Spawn weight (100 = guaranteed if only composition) |
| `ship_ids` | array | Enemy ship IDs from start2.json (use 0 if unknown) |
| `formation` | i64? | Override formation (null = use `formations` field) |
| `raw_ship_names` | array | Japanese ship names from wikiwiki (for manual verification) |

### Ship ID resolution

Ship names on wikiwiki use pattern: `<ship class>` + `<grade>` (e.g., `駆逐イ級`,
`軽巡ホ級`, `戦艦ル級`). Common enemy ship IDs:
- Patrol boat (PT): `駆逐イ級` etc.
- Destroyer (DD): `駆逐ロ級`, `駆逐ハ級`
- Light cruiser (CL): `軽巡ホ級`, `軽巡ヘ級`
- Heavy cruiser (CA): `重巡リ級`
- Battleship (BB): `戦艦ル級`, `戦艦タ級`
- Carrier (CV): `空母ヲ級`

If you cannot resolve a ship name to an ID, use ship_id `0` and put the raw name
in `raw_ship_names`. The Rust pipeline has a resolver that can fill in gaps.

## ShipDropDefinition

```json
{
  "12": [
    {
      "ship_id": 0,
      "raw_ship_name": "浦波",
      "tags": []
    }
  ]
}
```

Keyed by cell_no in the `ship_drops` map. Often empty for many maps.

## Overlay Types (for event maps)

For event/operation maps with multiple phases, overlay data may be extracted.
See `WikiwikiMapOverlayCatalog`, `WikiwikiMapOverlayDefinition`, and
`WikiwikiLabelOverlay` in the Rust types. For normal maps, the `overlays` field
is empty.
