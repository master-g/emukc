## Context

EmuKC currently returns ALL regular maps via `api_get_member/mapinfo` regardless of player progress. The real KanColle server only returns maps the player has unlocked (apilist.txt: "未出現海域は存在しない"). There is no prerequisite data, no unlock tracking, and no unlock notification on map clear.

Current implementation chain:
- `MapDefinition` (`crates/emukc_model/src/codex/map/types.rs`): No prerequisite fields
- `map_record` entity (`crates/emukc_db/src/entity/profile/map_record.rs`): No `unlocked` field
- `MapRecord` model (`crates/emukc_model/src/profile/map_record.rs`): No unlock state
- `DEFAULT_MAP_RECORDS`: Lists all maps 11-73, all with `cleared: false`
- `MapOps::get_map_infos()` (`crates/emukc_gameplay/src/game/map.rs:90`): Returns all maps
- `build_map_infos()` (`crates/emukc_gameplay/src/game/map.rs:327`): No filtering by unlock
- `ensure_map_records_impl()` (`crates/emukc_gameplay/src/game/map.rs:225`): Creates records for ALL maps
- `SortieBattleResultResponse` (`crates/emukc_gameplay/src/game/sortie_result.rs:64`): No `api_next_map_ids`
- `api_req_map/start` handler (`src/bin/net/router/kcsapi/api_req_map/start.rs`): No unlock check

## Goals / Non-Goals

**Goals:**
- Define map prerequisites in codex (static data, loaded at startup)
- Track per-player unlock state in the database
- Filter `api_get_member/mapinfo` to unlocked maps only
- Gate `api_req_map/start` against unlock status
- Return `api_next_map_ids` in `battleresult` when clearing a map unlocks new maps
- New accounts start with only map 1-1 unlocked

**Non-Goals:**
- Event map unlock/gimmick systems
- EO map (1-5, 1-6, etc.) special prerequisite rules — use same prerequisite mechanism but can be added incrementally
- `api_gauge_type`/`api_gauge_num`/`api_defeat_count` field correctness
- Combined fleet or LBAS unlock gating

## Decisions

### D1: Prerequisite data location — Codex static config

**Decision**: Define prerequisites as a static map in `MapCatalog` (within the codex), populated from a hardcoded table in `emukc_model`.

**Alternatives considered**:
- **Bootstrap asset file** (JSON/TOML): Adds another asset to manage, requires bootstrap step for a simple lookup table
- **Database table**: Overkill for static data that never changes per-player
- **`DEFAULT_MAP_RECORDS` extension**: Mixing prerequisite data with default state is messy

**Rationale**: Prerequisites are game-configuration data (identical for all players), not per-player state. Codex is the single source of truth for game configuration. A `HashMap<i64, i64>` (map_id → prerequisite_map_id) inside `MapCatalog` is sufficient. For maps with no prerequisite (only 1-1), the lookup returns `None`.

**Prerequisite rules** (regular maps):
- 1-1: no prerequisite (always unlocked)
- Same area sequential: N-M requires N-(M-1) cleared (e.g., 1-2 requires 1-1 cleared)
- Cross-area: clearing area boss unlocks next area's first map (1-4 → 2-1, 2-4 → 3-1, etc.)
- EO maps: will use same mechanism but with specific prerequisite IDs (future work)

### D2: Unlock state storage — `unlocked` column on `map_record`

**Decision**: Add a boolean `unlocked` column to the existing `map_record` SeaORM entity.

**Alternatives considered**:
- **Separate `map_unlock` table**: Normalized but adds join complexity for what is a simple flag
- **Inferred at query time** (compute from prerequisites + cleared state): Correct but requires recursive queries; slower and harder to paginate

**Rationale**: Unlock state changes rarely (only on map clear), so a denormalized boolean is efficient. The column is set during:
1. Account init: only 1-1 gets `unlocked = true`
2. Map clear: `check_and_unlock_dependencies()` cascades unlocks
3. Migration: existing accounts get `unlocked = true` for all cleared maps, then forward-chained for prerequisite-satisfied maps

### D3: Unlock cascade trigger — in `sortie_result` map clear path

**Decision**: After setting `cleared = true` on a map record, call `check_and_unlock_dependencies()` which looks up the prerequisite table and unlocks any maps whose prerequisite is now satisfied.

**Rationale**: Map clear is the only trigger for regular map unlocks. The function reads the prerequisite table (codex), iterates maps that require the just-cleared map, checks if they're already unlocked, and if not, sets `unlocked = true` and collects their IDs for the `api_next_map_ids` response.

### D4: `ensure_map_records_impl` change — create all records, but locked

**Decision**: Keep creating records for all maps (needed for defeat_count tracking), but set `unlocked = false` for locked maps.

**Rationale**: The real game tracks defeat counts and gauge state even for maps the player hasn't reached yet (the data exists server-side). We need records to exist; we just filter them from the API response.

## Risks / Trade-offs

- **Migration complexity** → Existing accounts have all maps cleared/in-progress. Migration must: (1) set `unlocked = true` for all cleared maps, (2) forward-chain from cleared maps to unlock their dependents. Simple iterative approach is sufficient since the chain depth is bounded (max ~7 areas).
- **Prerequisite table accuracy** → The hardcoded prerequisite table must match real game rules. Can be verified against kcwiki/map wiki sources. Mistakes only affect new accounts; existing accounts keep their unlocks.
- **EO maps out of scope** → EO maps (1-5, 1-6, 2-5, etc.) have special prerequisites that differ from the simple sequential rule. For now, they'll be unlocked by default (or share the same prerequisite mechanism with correct IDs added later).

## Migration Plan

1. Add `unlocked` column to `map_record` table (SeaORM migration)
2. Existing rows: `unlocked = true` (all maps unlocked) — safe default for existing accounts
3. New profiles: only 1-1 gets `unlocked = true` via `ensure_map_records_impl` change
4. No rollback needed — the column is additive

## Open Questions

- Should EO maps be unlocked by default in the initial implementation, or should we define their prerequisites now?
- The real game's `api_req_map/start` error response format for locked maps — need to verify what error code/message the client expects (likely just `api_result = -1` with an error message).
