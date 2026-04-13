## Why

`api_get_member/mapinfo` returns all regular maps regardless of player progress. The real game only returns unlocked maps (apilist.txt: "未出現海域は存在しない"). New accounts can sortie any map immediately, breaking the intended progression. Additionally, `api_req_sortie/battleresult` does not return `api_next_map_ids` when a map is cleared, so the client never receives unlock notifications.

## What Changes

- Define per-map prerequisites in codex (e.g., "1-1 cleared → unlock 1-2", "1-4 cleared → unlock 2-1")
- Add `unlocked` field to `map_record` entity and model to track per-player unlock state
- Filter `api_get_member/mapinfo` to only return unlocked maps (matching real game behavior)
- Gate `api_req_map/start` against unlock status, reject sorties to locked maps
- Add `api_next_map_ids` to `sortie_battle_result` response when map clear unlocks new maps
- Initialize new accounts with only map 1-1 unlocked; infer unlock state for existing accounts from cleared records

## Capabilities

### New Capabilities
- `map-unlock`: Map unlock prerequisite definitions, per-player unlock state tracking, unlock gating on mapinfo/sortie, and post-clear unlock notification via `api_next_map_ids`

### Modified Capabilities
- `sortie`: Map start handler must validate unlock status before allowing sortie; battle result must emit `api_next_map_ids` on map clear

## Impact

- **Codex** (`emukc_model::codex::map`): New prerequisite data in `MapDefinition` or a dedicated map prerequisite catalog
- **Database** (`emukc_db::entity::profile::map_record`): New `unlocked` column (bool, default false for new accounts)
- **Model** (`emukc_model::profile::map_record`): `MapRecord` struct gains `unlocked` field; `DEFAULT_MAP_RECORDS` updated (only 1-1 unlocked)
- **Gameplay** (`emukc_gameplay::game::map`): `MapOps::get_map_infos()` filters by unlock state; new `MapOps::unlock_map()` / `MapOps::check_and_unlock_dependencies()` methods
- **Gameplay** (`emukc_gameplay::game::sortie_result`): Battle result computes and returns `api_next_map_ids`
- **API handler** (`api_get_member/mapinfo`): Response filtered (no code changes needed if gameplay layer handles it)
- **API handler** (`api_req_map/start`): Reject sortie to locked map with error response
- **API response model** (`KcApiBattleResult`): Add optional `api_next_map_ids` field
- **Migration**: New column on `map_record` table; existing rows set `unlocked = true` for all cleared maps, plus infer unlocks from prerequisite chains

## Non-goals

- Event map unlock progression (event maps have separate unlock/gimmick systems — out of scope)
- EO map (1-5, 1-6, 2-5, ...) specific prerequisite rules (can be added later in a follow-up change)
- `api_gauge_type` / `api_gauge_num` / `api_defeat_count` / `api_required_defeat_count` field correctness (separate concern from unlock gating)
- Map HP gauge damage tracking improvements
- Combined fleet or LBAS unlock gating
