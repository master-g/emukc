## ADDED Requirements

### Requirement: Resource manifest extractor scans all modules
The extractor SHALL traverse all decoded modules (not just battle-tagged modules) to discover resource loading patterns.

#### Scenario: Non-battle module contains resource loading
- **WHEN** a decoded module calls `resources.getShip(id, damaged, type)` and the module is not tagged as battle-related
- **THEN** the extractor SHALL include the resource entry in the manifest output

#### Scenario: Battle module contains resource loading
- **WHEN** a decoded module calls `resources.getShip(id, damaged, type)` and the module is tagged as battle-related
- **THEN** the extractor SHALL include the resource entry in the manifest output (no battle-only filtering)

### Requirement: Ship resource pattern extraction
The extractor SHALL match `resources.getShip(id, damaged, type)` and `ShipLoader.add(id, damaged, type)` call expressions across all modules, extracting id source expression, damaged source expression, and target type string literal.

#### Scenario: Standard getShip call
- **WHEN** a module contains `resources.getShip(vo.ship.api_id, false, "full")`
- **THEN** the extractor SHALL produce an entry with kind "ship", source "resources.getShip", shipMstIdSource "vo.ship.api_id", damagedSource "false", targetType "full"

#### Scenario: ShipLoader.add call
- **WHEN** a module contains `ShipLoader.add(shipId, isDamaged, "banner")`
- **THEN** the extractor SHALL produce an entry with kind "ship", source "ShipLoader.add", shipMstIdSource "shipId", damagedSource "isDamaged", targetType "banner"

#### Scenario: Invalid getShip call (spread arguments)
- **WHEN** a module contains `resources.getShip(...args)`
- **THEN** the extractor SHALL skip this call (no entry produced)

### Requirement: Slotitem resource pattern extraction
The extractor SHALL match `resources.getSlotitem(id, type)` and `SlotLoader.add(id, type)` call expressions across all modules, extracting id source expression and target type string literal.

#### Scenario: Standard getSlotitem call
- **WHEN** a module contains `resources.getSlotitem(eq.api_id, "card")`
- **THEN** the extractor SHALL produce an entry with kind "slotitem", source "resources.getSlotitem", slotMstIdSources ["eq.api_id"], targetType "card"

#### Scenario: SlotLoader.add call
- **WHEN** a module contains `SlotLoader.add(itemId, "item_on")`
- **THEN** the extractor SHALL produce an entry with kind "slotitem", source "SlotLoader.add", slotMstIdSources ["itemId"], targetType "item_on"

### Requirement: Texture provider pattern extraction
The extractor SHALL match `getTexture(provider, ...ids)` call expressions across all modules, extracting provider name and numeric texture ID arguments.

#### Scenario: getTexture with multiple IDs
- **WHEN** a module contains `textures.getTexture("COMMON_MISC", 1, 2, 5)`
- **THEN** the extractor SHALL produce an entry with kind "texture-provider", provider "COMMON_MISC", textureIds [1, 2, 5]

#### Scenario: Multiple getTexture calls with same provider
- **WHEN** a module contains two calls `textures.getTexture("FOO", 1)` and `textures.getTexture("FOO", 2)`
- **THEN** the extractor SHALL merge them into one entry with textureIds [1, 2]

### Requirement: Explicit path extraction
The extractor SHALL scan all module source code for `kcs2/resources/[A-Za-z0-9_./-]+` patterns and collect unique explicit resource paths.

#### Scenario: Hardcoded resource path in source
- **WHEN** a module source contains the string `kcs2/resources/battle/banner/001_abc.png`
- **THEN** the extractor SHALL include this path in an explicit-path entry

#### Scenario: Duplicate paths across modules
- **WHEN** two modules both contain the same explicit path `kcs2/resources/ui/common/bg.png`
- **THEN** the path SHALL appear once in the manifest

### Requirement: Output format
The extractor SHALL output a JSON file containing a version field, timestamp, and an entries array. Each entry SHALL include kind, source module ID, and source module name.

#### Scenario: Valid output structure
- **WHEN** the extractor completes successfully
- **THEN** the output JSON SHALL contain "version" (integer 1), "generatedAt" (ISO 8601 string), and "entries" (array of resource entries)

#### Scenario: Entry includes module provenance
- **WHEN** a resource entry is produced from module "abc123" named "BattleRenderer"
- **THEN** the entry SHALL contain moduleId "abc123" and moduleName "BattleRenderer"

### Requirement: CLI flag integration
The decode pipeline SHALL accept a `--sync-resource-manifest` flag that triggers resource manifest extraction and writes output to `crates/emukc_bootstrap/assets/resource_manifest.json`.

#### Scenario: Flag provided
- **WHEN** user runs `bun run decode -- --sync-resource-manifest`
- **THEN** the pipeline SHALL extract the resource manifest and write it to the assets directory

#### Scenario: Flag not provided
- **WHEN** user runs `bun run decode` without `--sync-resource-manifest`
- **THEN** the pipeline SHALL NOT run the resource manifest extractor

### Requirement: Deduplication
The extractor SHALL deduplicate entries before output. Ship entries by (targetType, source). Slotitem entries by (targetType, source). Texture-provider entries by (provider). Explicit-path entries by individual path.

#### Scenario: Same pattern in multiple modules
- **WHEN** two modules both call `resources.getShip(id, false, "full")`
- **THEN** the manifest SHALL contain one ship entry with both moduleIds in its provenance

### Requirement: No modification to existing extractors
The resource manifest extractor SHALL NOT modify the behavior, output, or interface of the existing `battle-knowledge.ts` extractor.

#### Scenario: Battle knowledge output unchanged
- **WHEN** both `--sync-battle-assets` and `--sync-resource-manifest` flags are provided
- **THEN** battle knowledge assets SHALL be identical to running with `--sync-battle-assets` alone

### Requirement: fetch_from_remote fails immediately on HTTP 404
When `fetch_from_remote` in `emukc_cache::kache` receives an HTTP 404 response from any CDN node, it SHALL return `FailedOnAllCdn` immediately without attempting remaining CDN nodes. Non-404 errors (connection failures, timeouts, 5xx) continue to fall through to the next CDN node as before.

#### Scenario: First CDN returns 404
- **WHEN** `fetch_from_remote` requests a resource and the first CDN node returns HTTP 404
- **THEN** no other CDN nodes are tried, and `FailedOnAllCdn` is returned

#### Scenario: Connection timeout on first CDN, success on second
- **WHEN** `fetch_from_remote` requests a resource, the first CDN times out, and the second CDN returns 200
- **THEN** the resource is downloaded from the second CDN (existing fallback preserved)
