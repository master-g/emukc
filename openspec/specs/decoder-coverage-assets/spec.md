# decoder-coverage-assets Specification

## Purpose
TBD - created by archiving change expand-decoder-cachelist-coverage. Update Purpose after archive.
## Requirements
### Requirement: Sparse ship and slot subsets are emitted with explicit completeness metadata
The decoder SHALL emit a structured coverage asset bundle for sparse ship and slot categories whose membership is directly observable in decoded `main.js`. Each extracted subset MUST include the category key, resource domain, observed IDs, provenance, and an explicit coverage mode that distinguishes complete observation from partial or unresolved observation. Categories with decoder-observable literal evidence MUST no longer remain unresolved merely because the current extractor fails to connect that evidence to the emitted subset.

#### Scenario: Observable sparse ship subset is extracted
- **WHEN** decoded modules contain an explicit enumerable subset for ship resources such as `special`, `sp_remodel/*`, `card_round`, or `reward_*`
- **THEN** the emitted coverage asset MUST include that subset under a stable category key
- **THEN** the asset MUST record the observed ship IDs and provenance identifying which decoder modules produced the subset

#### Scenario: Runtime-driven subset remains unresolved
- **WHEN** a ship or slot category is only referenced through runtime-driven identifiers and the decoder cannot enumerate a stable subset directly from decoded `main.js`
- **THEN** the emitted coverage asset MUST mark that category as `partial` or `unresolved`
- **THEN** the decoder MUST NOT synthesize completeness by copying Rust constants or CDN-derived IDs into the asset

### Requirement: Audio coverage asset captures cache-list audio domains
The decoder SHALL emit an audio coverage asset that records directly observable cache-list audio domains needed by bootstrap generation, including sound-effect IDs, categorized BGM IDs, and voice-related ranges or explicit file stems. The emitted asset MUST materially cover the currently dominant fallback families before those families are considered decoder-authored by the Rules path.

#### Scenario: Sound effect and BGM data are visible in decoded modules
- **WHEN** decoded modules expose numeric sound-effect IDs or categorized BGM IDs through explicit paths, call arguments, inline tables, or other decoder-observable runtime structures
- **THEN** the audio coverage asset MUST store those IDs under stable domain keys usable by bootstrap cache-list generation
- **THEN** the asset MUST NOT leave categorized `port`, `battle`, or `fanfare` BGM coverage empty when decoder-observable evidence for those families exists in the decoded script

#### Scenario: Voice domain is represented by ranges or file stems
- **WHEN** decoded modules expose titlecall ranges, tutorial voice stems, or other voice file groupings without listing every file individually
- **THEN** the audio coverage asset MUST preserve the observable range or stem form instead of flattening it into invented IDs
- **THEN** the emitted asset MUST make the reachable titlecall and explicit voice-file families available strongly enough to reduce the current fallback-owned titlecall residuals

### Requirement: UI coverage asset captures explicit cache-list file groups
The decoder SHALL emit a UI coverage asset that records explicit file groups for non-ship/slot UI domains required by cache-list generation, including map, furniture, useitem, area, and world-select resources. For decoder-observable migration-critical UI families, the asset MUST emit concrete members instead of leaving those groups empty, and MUST preserve partial or unresolved coverage modes when the decoded script does not prove full membership.

#### Scenario: Map and furniture files are observable
- **WHEN** decoded modules contain explicit map or furniture resource paths, file groups, deterministic literal references, or decoder-observable construction patterns
- **THEN** the UI coverage asset MUST store those paths or file groups in a structure that preserves the original domain grouping
- **THEN** decoder-observable map and furniture members MUST be available to downstream cache-list generation as concrete asset members

#### Scenario: Useitem card and underline resources are distinct
- **WHEN** decoded modules expose `useitem/card` and `useitem/card_` style resources through direct paths, literal ID sets, or deterministic construction patterns
- **THEN** the UI coverage asset MUST represent those groups separately so bootstrap generation can preserve the current cache-list layout
- **THEN** the emitted asset MUST NOT leave both groups empty when decoder-observable IDs exist

#### Scenario: Area resources are observable
- **WHEN** decoded modules expose `area/sally`, `area/airunit`, or `area/airunit_extend_confirm` resources through direct paths, literal ID sets, or deterministic construction patterns
- **THEN** the UI coverage asset MUST preserve those concrete members under stable area domain keys
- **THEN** the emitted asset MUST keep unresolved area groups partial or unresolved instead of claiming complete coverage without decoder evidence

#### Scenario: World-select resources are observable
- **WHEN** decoded modules expose `worldselect` resources through direct paths, literal filenames, or deterministic construction patterns
- **THEN** the UI coverage asset MUST preserve those concrete files under a stable world-select domain key
- **THEN** the emitted asset MUST make decoder-observable world-select files available for Rules-path cache-list generation

### Requirement: Coverage assets are emitted as decoder outputs and syncable bootstrap assets
The decoder pipeline SHALL write coverage assets into the normal decoder output tree and SHALL support syncing those same assets into `crates/emukc_bootstrap/assets/` for repo-tracked workflows.

#### Scenario: Decoder run without bootstrap sync
- **WHEN** the decoder runs with coverage extraction enabled but without an asset sync flag
- **THEN** the coverage assets MUST be written under the decoder output directory
- **THEN** the pipeline MUST NOT modify repo-tracked bootstrap assets

#### Scenario: Decoder run with bootstrap sync
- **WHEN** the decoder runs with the asset sync workflow enabled
- **THEN** the same coverage assets MUST be written to the decoder output directory and synced into `crates/emukc_bootstrap/assets/`

### Requirement: Cache rules asset emits canonical ship semantic scope
The decoder SHALL emit ship semantic rule data in `cache_rules.json` for target families whose generation behavior cannot be represented correctly by raw manifest entries alone.

#### Scenario: Ship target family needs semantic disambiguation
- **WHEN** decoded `main.js` usage distinguishes between base, damaged-only, or group-scoped ship targets inside the same family
- **THEN** `cache_rules.json` MUST encode the canonical semantic behavior for that family
- **THEN** the emitted rule MUST preserve enough selector scope information for downstream generation to distinguish friendly, abyssal, graph-driven, or sparse-subset behavior

#### Scenario: Decoder cannot prove full ship semantic scope
- **WHEN** the decoder cannot derive complete semantic scope for a ship target family directly from decoded `main.js`
- **THEN** `cache_rules.json` MUST mark that semantic rule as partial or unresolved
- **THEN** the decoder MUST NOT synthesize complete ship semantic scope by copying Rust-authored fallback tables

### Requirement: Cache rules asset emits slot normalization semantics
The decoder SHALL emit slot semantic rule data in `cache_rules.json` for normalization-driven or alias slot target families whose behavior depends on runtime selector mapping rather than universal slotitem membership.

#### Scenario: Slot alias family is runtime-normalized
- **WHEN** decoded `main.js` usage shows a slot target family such as `item_up2` or `item_on2` is produced through runtime normalization or alias behavior
- **THEN** `cache_rules.json` MUST encode the normalization and selector constraints needed to reproduce that family precisely
- **THEN** the emitted rule MUST be consumable without treating the family as a universal slotitem category

#### Scenario: Slot normalization cannot be resolved completely
- **WHEN** the decoder cannot fully derive a slot alias family's selector or normalization behavior
- **THEN** `cache_rules.json` MUST mark that family as partial or unresolved
- **THEN** the decoder MUST NOT claim complete slot semantic precision for that family

### Requirement: Decoder semantic rules remain decoder-authored artifacts
The decoder SHALL derive ship and slot semantic rule outputs from decoded `main.js` evidence and SHALL NOT use Rust-authored path rule constants as the source of truth for semantic meaning.

#### Scenario: Semantic rule can be derived from decoded runtime evidence
- **WHEN** decoded modules expose enough call structure, grouping behavior, or normalization behavior to infer a semantic rule
- **THEN** the decoder MUST emit that rule from decoder-observed evidence with provenance
- **THEN** the rule MUST be stable when regenerated from the same decoded artifact set

#### Scenario: Semantic rule cannot be derived from decoded runtime evidence
- **WHEN** decoded modules do not expose enough evidence to derive a semantic rule safely
- **THEN** the decoder MUST leave that semantic rule partial or unresolved
- **THEN** the decoder MUST NOT backfill semantic truth by parsing Rust fallback constants

### Requirement: Decoder distinguishes explicit audio assets from algorithmic sound-rule families
The decoder SHALL keep explicit audio asset extraction distinct from algorithmic `kcs/sound/*` rule extraction so decoder output makes clear which audio domains are direct asset groups and which are rule-driven sound families.

#### Scenario: Explicit audio asset is directly referenced
- **WHEN** decoded modules expose direct `se`, `bgm`, titlecall, tutorial voice, or explicit voice file references
- **THEN** the decoder MUST continue to emit those as explicit audio coverage assets
- **THEN** the decoder MUST NOT require an algorithmic sound rule just to represent a direct explicit asset path

#### Scenario: Algorithmic sound family is inferred
- **WHEN** decoded modules expose sound behavior that is better modeled as a semantic or algorithmic `kcs/sound/*` rule
- **THEN** the decoder MUST emit that family as a sound rule rather than flattening it into the explicit audio asset lists
- **THEN** the output MUST make the distinction between explicit audio coverage and rule-driven sound generation observable

### Requirement: Decoder emits sound-rule metadata needed for `kcs/sound` migration
The decoder SHALL emit the metadata needed to drive decoder-authored `kcs/sound/*` generation for covered sound families.

#### Scenario: Covered sound bucket or formula family is observed
- **WHEN** decoded modules expose a covered `kcs/sound/*` family
- **THEN** the decoder output MUST preserve the bucket identity, reachable voice IDs, and any semantic grouping needed for downstream cache-list generation
- **THEN** the emitted metadata MUST be stable enough to regenerate the same sound-rule bundle from the same decoded script version

#### Scenario: Sound-rule family is not fully derivable
- **WHEN** decoded modules expose only partial evidence for a sound-rule family
- **THEN** the decoder MUST mark that family partial or unresolved
- **THEN** the decoder MUST NOT silently claim complete algorithmic sound coverage for it

### Requirement: Decoder emits template-backed resource family metadata
The decoder SHALL emit structured metadata for resource families whose path shape is observable in decoded `main.js` as a deterministic template but whose member set depends on runtime bootstrap inputs. Each template-backed family MUST record a stable family key, resource domain, path template, required input bindings, coverage mode, decoded-module provenance, and enough completeness information for downstream generation to distinguish complete ownership from residual fallback territory.

#### Scenario: Deterministic template is observed
- **WHEN** decoded modules expose a deterministic resource path formula for map, gauge-adjacent map, furniture, BGM, sound bucket, titlecall, useitem, area, or world-select resources
- **THEN** the decoder output MUST represent that formula as a template-backed family with stable family identity and path-template metadata
- **THEN** the decoder output MUST record the decoded module provenance that supports the template

#### Scenario: Template needs runtime membership input
- **WHEN** the decoder can prove a path template but cannot enumerate the full member set from decoded `main.js` alone
- **THEN** the decoder output MUST declare the required runtime input binding for that template-backed family
- **THEN** the decoder output MUST NOT synthesize the missing member set by copying Rust fallback constants, CDN-derived lists, or generated cache-list output

#### Scenario: Migration-critical template family needs blocker metadata
- **WHEN** a migration-critical template family such as `map.base`, `gauge.map`, `bgm.category`, or `sound.kc9998` is emitted as partial or unresolved
- **THEN** the decoder output MUST preserve the path-template evidence that was observed
- **THEN** the decoder output MUST expose the missing descriptor, family-boundary, or runtime-input reason that prevents complete decoder ownership

### Requirement: Template-backed coverage separates path authority from member completeness
The decoder SHALL distinguish path-template authority from member-set completeness for template-backed resource families. A family MAY be decoder-authoritative for path shape while remaining partial or unresolved for membership until its required runtime inputs are available to downstream generation.

#### Scenario: Template shape is complete but membership is runtime-bound
- **WHEN** decoded modules fully prove the path construction formula and the family depends on declared runtime inputs for member enumeration
- **THEN** the decoder output MUST mark the template shape as complete and the required membership inputs explicitly
- **THEN** downstream generation MUST be able to decide ownership from the descriptor and input availability instead of treating the family as an opaque fallback list

#### Scenario: Template evidence is incomplete
- **WHEN** decoded modules do not prove enough of the path formula, family boundary, or input binding to generate the family safely
- **THEN** the decoder output MUST mark that template-backed family as partial or unresolved
- **THEN** the decoder output MUST preserve provenance for the partial evidence without claiming complete decoder coverage

#### Scenario: Runtime input can prove complete ownership
- **WHEN** a template-backed family has complete path evidence and all declared runtime inputs are available to bootstrap generation
- **THEN** the decoder output MUST provide enough descriptor data for downstream generation to expand the family without consulting legacy fallback for the same family
- **THEN** any remaining fallback requirement MUST be represented as an explicit partial or unresolved coverage mode rather than implicit broad fallback ownership

