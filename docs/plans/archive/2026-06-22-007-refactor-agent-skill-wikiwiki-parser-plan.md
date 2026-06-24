---
title: "refactor: Replace Programmatic Wikiwiki HTML Parser with Agent Skill"
type: refactor
date: 2026-06-22
origin: brainstorm session (inline) — replacing 7,389 lines of regex/HTML parsing with LLM-based extraction
status: completed
---

# refactor: Replace Programmatic Wikiwiki HTML Parser with Agent Skill

## Summary

The wikiwiki HTML parser (`crates/emukc_bootstrap/src/parser/wikiwiki_map/`, 11 files,
7,389 lines) uses 20 regex patterns and 79 hardcoded Japanese text literals to parse
natural-language route conditions, enemy tables, and drop tables from wikiwiki.jp HTML
pages. This is exactly the kind of semantic text understanding where LLMs excel and
programmatic parsers are brittle — every wikiwiki page rephrasing breaks a regex.

This plan replaces the programmatic parser with a project-level agent skill
(`emukc-scrape-wikiwiki-mapdata`) that reads downloaded HTML pages and outputs
structured JSON (`WikiwikiMapCatalog` format). The agent works in **label space**
(A, B, Start) — cell number assignment stays in the retained Rust conversion layer
(`into_map_catalog()`), which is deterministic topology work.

**Delete ~6,800 lines** of HTML parsing + semantic extraction code.
**Keep ~500 lines** of type definitions + label→cell_no topology conversion.
**Create** one agent skill with reference documentation.

## Problem Frame

`route_condition.rs` alone is 2,102 lines trying to match Japanese phrases like
`"次の条件のいずれかを満たし"` via regex. It has 11 explicit `Unknown`/`SourceUnknown`
fallback paths admitting it cannot handle all cases. This is an NLP problem being solved
with pattern matching.

The agent skill approach:

- **Agent does what agents do well**: read HTML, understand tables, interpret natural
  language route conditions, map ship names to IDs.
- **Rust code does what Rust does well**: deterministic label→cell_no topology
  assignment, JSON deserialization validation, catalog assembly.
- **No runtime dependency on the agent**: the agent output is committed as a static JSON
  asset (`wikiwiki_map_catalog.json`). The agent runs only when updating map data.

### Data flow (before → after)

```text
Before:
  HTML → 7,389行 Rust 解析器 → WikiwikiMapCatalog → into_map_catalog() → MapCatalog

After:
  HTML → agent skill → WikiwikiMapCatalog.json → into_map_catalog() → MapCatalog
```

## Scope Boundaries

### In scope

- Creating the `emukc-scrape-wikiwiki-mapdata` project-level agent skill
- Deleting HTML parsing + semantic extraction code (~6,800 lines)
- Keeping `WikiwikiMapCatalog` type definitions and `into_map_catalog()` conversion
- Updating CLI workflow and module exports
- Updating the `normalize` CLI command to consume agent-produced JSON

### Non-goals

- Changing kcdata pipeline (topology, cells, routes) — untouched
- Changing assemble.rs merge logic — untouched
- Changing runtime code (gameplay, battle, sortie) — untouched
- Changing `wikiwiki_map_download.rs` (HTML download) — untouched
- Changing `wikiwiki_map_asset.rs` (JSON asset loading) — untouched
- Re-parsing all 131 maps in this plan — the skill is validated on representative maps;
  full re-parse is a follow-up operational task

---

## Key Technical Decisions

### KTD1. Agent output format: `WikiwikiMapCatalog` (label space)

The agent outputs JSON matching the existing `WikiwikiMapCatalog` struct from
`crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs`. Crucially, this format works
in **label space** — the agent writes node labels like `"A"`, `"B"`, `"Start"` and does
not assign cell numbers. Cell number assignment (`build_nodes()` + `into_map_catalog()`)
is deterministic topology work that stays in Rust.

This means the agent never needs to understand `cell_no`, `api_no`, or the route→cell
model in terms of KanColle's API internals. But the agent **does** assign sequential
cell numbers within each variant (start=0, then BFS order 1, 2, 3...). This is trivial
topology numbering — "A is reachable first, so A=1; B comes after A, so B=2" — not
API-level cell numbering. The agent's skill reference will explain this numbering
convention with a worked example.

The conversion layer (`into_map_catalog()`) reads the agent's cell_no assignments
and uses them to build `MapVariantDefinition.cells`, routing rules, and
`boss_cell_no`. It performs no topology renumbering — it trusts the agent's BFS
ordering, same as it currently trusts `build_nodes()` output.

- What nodes exist (by label: A, B, C...)
- Which node is the boss
- Which nodes have battles
- Route rules (from label → to label, with conditions)
- Enemy fleet compositions (by node label)
- Ship drops (by node label)

### KTD2. Type definitions stay in Rust, not in the skill

The `WikiwikiMapCatalog` type and its sub-types remain defined in Rust
(`types.rs`). The skill carries a human-readable JSON schema reference document
showing the expected structure, but the canonical type definition is the Rust
struct with `#[derive(Serialize, Deserialize)]`. This ensures compile-time
validation when the agent JSON is loaded.

### KTD3. Parser module reorganization

After deleting parser internals, the module at `parser/wikiwiki_map/` will contain only:

- `mod.rs` — `WikiwikiMapCatalog` struct + `into_map_catalog()` / `into_map_catalog_with_overlay()`
- `types.rs` — type definitions (kept as-is)

The `mod.rs` no longer exports `parse()`, `parse_debug()`, or any HTML-related functions.
Internal types used only by the deleted parser (`ShipTypeResolver`, `ShipResolver`,
`RouteSelector`, `RouteTableSection`, etc.) are removed. The module stays at its current
path to minimize import churn — `parser::wikiwiki_map` is semantically "the wikiwiki
catalog types", not "the wikiwiki HTML parser".

### KTD4. Skill location: project-level under `.claude/skills/`

The project already has `.claude/skills/` with two skills (`commit`, `emukc-api-development`).
The new skill follows the same convention: `.claude/skills/emukc-scrape-wikiwiki-mapdata/`.

---

## Implementation Units

### U1. Create the agent skill definition

- **Goal:** A project-level skill that an agent can invoke to parse wikiwiki HTML pages
  into structured `WikiwikiMapCatalog` JSON.
- **Files:**
  - `.claude/skills/emukc-scrape-wikiwiki-mapdata/SKILL.md` — skill definition with
    instructions, trigger conditions, input/output specification
  - `.claude/skills/emukc-scrape-wikiwiki-mapdata/reference/catalog-schema.md` —
    human-readable JSON schema showing the `WikiwikiMapCatalog` structure with
    field-by-field documentation and a complete annotated example
  - `.claude/skills/emukc-scrape-wikiwiki-mapdata/reference/map-example.json` —
    a real example output for map 1-2 (the multi-route boss map we know well),
    showing nodes, routing rules, enemy fleets, and drops
- **Approach:**
  - SKILL.md describes: what the skill does, when to trigger it (explicit invocation only),
    what inputs it reads (HTML files from `.data/temp/wikiwiki_map/pages/`), what output it
    produces (`WikiwikiMapCatalog` JSON), and what manifest data it needs (ship IDs, ship
    type IDs from `start2.json`)
  - The schema reference explains each field of `WikiwikiMapCatalog`,
    `WikiwikiMapDefinition`, `WikiwikiMapVariantDefinition`, `WikiwikiNodeDefinition`,
    `WikiwikiEnemyFleetDefinition`, `RouteRule`, `RoutePredicate`, `EnemyComposition`,
    and `ShipDropDefinition`
  - The reference includes examples of common wikiwiki route condition patterns and how
    they map to `RoutePredicate` variants (e.g., `"艦隊サイズ4隻以上"` →
    `FleetSize { op: Gte, value: 4 }`)
  - The map 1-2 example is derived from the existing catalog JSON, not hand-written
- **Patterns to follow:** `.claude/skills/emukc-api-development/SKILL.md` for project
  skill structure; existing skill definitions in the Pi ecosystem for SKILL.md format
- **Test scenarios:**
  - Manual: invoke the skill on map 1-1 HTML, verify the JSON output matches expected
    structure (4 nodes, 3 enemy fleets, 1 route rule)
  - Manual: invoke on map 1-2 HTML, verify multi-route boss detection (cells 5+6
    share label E, boss flag on E)
  - The skill itself has no automated tests — it is validated by U2
- **Verification:** the skill produces valid `WikiwikiMapCatalog` JSON that
  deserializes successfully via `serde_json::from_str`

### U2. Agent output validation harness

- **Goal:** A lightweight validation that the agent-produced JSON deserializes into the
  Rust `WikiwikiMapCatalog` type and passes structural sanity checks, before any parser
  code is deleted.
- **Dependencies:** U1.
- **Files:**
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs` — add a `pub fn
    validate_catalog_json(raw: &str) -> Result<WikiwikiMapCatalog, serde_json::Error>`
    function that deserializes and checks non-empty maps
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs` — add a test that loads
    the example JSON from U1 and validates it
- **Approach:**
  - The validation function is the seam between agent output and Rust types.
    It exists before parser deletion so we can test agent output independently.
  - Run the skill (U1) on a few maps (1-1, 1-2, 2-1), save outputs, validate each
    via the harness
  - This is a characterization step — prove the agent can produce valid output
    before removing the fallback
- **Test scenarios:**
  - Deserialize the map 1-2 example → succeeds, 1 map, 1 variant, ≥5 nodes
  - Deserialize empty JSON `{}` → succeeds but 0 maps (warning, not error)
  - Deserialize malformed JSON → `serde_json::Error`
  - Deserialize valid JSON with wrong field types → `serde_json::Error`
- **Verification:** `cargo test -p emukc_bootstrap wikiwiki` green; manual validation
  of skill output on ≥3 representative maps passes

### U3. Delete parser internals, keep types + conversion

- **Goal:** Remove all HTML parsing and semantic extraction code (~6,800 lines),
  keeping only the `WikiwikiMapCatalog` type definitions and `into_map_catalog()`
  conversion logic.
- **Dependencies:** U2 (agent output must be validated first as a fallback).
- **Files to delete:**
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/html.rs` (277 lines)
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/enemy.rs` (131 lines)
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/drop.rs` (172 lines)
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/resolver.rs` (414 lines)
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/mod.rs` (98 lines)
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/route_table.rs` (140 lines)
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/route_predicate.rs` (709 lines)
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/route_condition.rs` (2,102 lines)
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs` (2,548 lines) — replace
    with the U2 validation tests
- **Files to modify:**
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs` — remove `parse()`,
    `parse_debug()`, `parse_map_page()`, all `static` regex/selectors, all helper
    functions (`build_nodes`, `parse_map_name`, `compact_enemy_composition`,
    `ordered_route_targets`, `rewrite_route_predicate_labels`, etc.). Keep
    `WikiwikiMapCatalog` struct, `into_map_catalog()`,
    `into_map_catalog_with_overlay()`, `to_debug_json()`. Remove module
    declarations for deleted submodules.
  - `crates/emukc_bootstrap/src/parser/wikiwiki_map/types.rs` — remove
    `ShipTypeResolver`, `ShipResolver`, `RouteSelector`, `RouteTableSection`,
    `CompiledRouteClause`, `RouteConditionLine`, `DropCellEvent`, `RouteClauseAst`
    (internal parser types no longer needed). Keep
    `WikiwikiMapCatalog`, `WikiwikiMapDefinition`,
    `WikiwikiMapVariantDefinition`, `WikiwikiNodeDefinition`,
    `WikiwikiEnemyFleetDefinition`, `EnemyNodeRows`, `RouteRuleDraft`,
    `ShipDropDraft`, and all overlay types.
  - `crates/emukc_bootstrap/src/lib.rs` — remove exports for deleted functions
    (`parse_wikiwiki_map`, `parse_wikiwiki_map_debug`). Keep exports for types
    and any new validation function from U2.
- **Approach:**
  - **Key dependency analysis:** `build_nodes()` (which assigns cell_nos via BFS)
    runs in `parse_map_page()` during the **parsing phase**, not in
    `into_map_catalog_with_overlay()` during the **conversion phase**. The
    conversion function only reads the pre-assigned `variant.nodes[i].cell_no`
    values. Since the agent replaces the parsing phase, the agent also replaces
    `build_nodes()` — the agent assigns cell_nos itself (sequential BFS from
    start node, start=0, first node=1, etc.). `build_nodes()` is therefore
    **deleted, not preserved**.
  - `into_map_catalog_with_overlay()` does depend on two helpers in `mod.rs`:
    `ordered_route_targets()` (line ~489) and
    `rewrite_route_predicate_labels()` (line ~500). These are pure functions
    operating on runtime types (`RouteRule`, `RoutePredicate`) with no HTML/regex
    dependency — they stay in `mod.rs`.
  - `compact_enemy_composition()` (line ~484) and `probability_to_weight()`
    (line ~564) are also used by the conversion and stay.
  - After deletion, `mod.rs` should be ~300-400 lines: the catalog struct,
    `into_map_catalog()`, `into_map_catalog_with_overlay()`, and the four
    standalone helpers above. No functions from the deleted `route/` submodule
    are needed — they all operate on parser-intermediate types
    (`RouteRuleDraft`, `EnemyNodeRows`) that no longer exist in the conversion
    path.
  - The `tests.rs` file is replaced with a new, much smaller test module
    (~100 lines) testing `into_map_catalog()` on synthetic catalog data.
- **Patterns to follow:** The existing `into_map_catalog_with_overlay()` is the
  authoritative reference for what the conversion needs. Tracing its call chain
  confirms it only depends on `mod.rs`-local helpers, not on any function in the
  `route/` or other deleted submodules.
- **Test scenarios:**
  - `into_map_catalog()` on a hand-built `WikiwikiMapCatalog` with map 1-2 topology
    → produces correct cell_nos, boss_cell_no, routing_rules
  - `into_map_catalog_with_overlay()` extracts overlay correctly
  - `validate_catalog_json()` from U2 still works
  - `cargo build -p emukc_bootstrap` compiles with zero warnings
- **Verification:** `cargo build --workspace` clean; `cargo clippy --workspace --tests
  -- -D warnings` clean; `cargo test -p emukc_bootstrap` green

### U4. Update CLI workflow and module exports

- **Goal:** The `wikiwiki-map` CLI commands reflect the new workflow: `sync` downloads
  HTML (unchanged), but `normalize` no longer calls the Rust parser — it loads
  agent-produced JSON and runs `into_map_catalog()`.
- **Dependencies:** U3.
- **Files:**
  - `src/bin/cli/wikiwiki_map.rs` — update `normalize` command: instead of calling
    `parse_wikiwiki_map()`, it loads a JSON file (agent output) and deserializes it
    as `WikiwikiMapCatalog`, then calls `into_map_catalog()`. Add a new subcommand or
    flag for the agent workflow. Update `debug` command similarly.
  - `crates/emukc_bootstrap/src/lib.rs` — remove `parse_wikiwiki_map` and
    `parse_wikiwiki_map_debug` from public exports (they no longer exist). Ensure
    `WikiwikiMapCatalog` and `into_map_catalog` are still exported.
- **Approach:**
  - The `normalize` command gains a `--from-agent-json <PATH>` flag that points to
    the agent-produced catalog JSON. The old path (parsing HTML directly) is removed.
  - Document the new workflow in the command's help text: "Run the
    `emukc-scrape-wikiwiki-mapdata` skill first, then pass its output here."
  - The `debug` command similarly loads agent JSON instead of parsing HTML.
  - The `sync` command (HTML download) is unchanged.
  - The `build-overlays` command is unchanged (it works on the assembled catalog).
- **Test scenarios:**
  - `normalize --from-agent-json <valid.json>` produces correct `MapCatalog` output
  - `normalize --from-agent-json <malformed.json>` returns error
  - `normalize` without `--from-agent-json` prints usage with workflow instructions
- **Verification:** `cargo run -- wikiwiki-map normalize --help` shows updated usage;
  `cargo build --workspace` clean

### U5. Update docs and verification sweep

- **Goal:** All documentation reflects the new workflow, and the full workspace passes
  all gates.
- **Dependencies:** U1, U3, U4.
- **Files:**
  - `crates/emukc_bootstrap/src/lib.rs` — verify exports are clean
  - `docs/solutions/architecture-patterns/map-data-authority.md` — update to reflect
    agent skill as the wikiwiki parsing mechanism (was: Rust parser)
  - `CLAUDE.md` — if the wikiwiki workflow is documented, update it
  - `crates/README.md` — update the description that mentions wikiwiki parser
- **Approach:**
  - Run the full gate suite: `cargo fmt`, `cargo clippy -D warnings`, `cargo test`
  - Verify the 5 kcdata route→cell tests still pass (proves kcdata pipeline untouched)
  - Verify `sortie_battle` integration tests still pass (15/15)
  - Verify `cargo run -- serve` still loads codex successfully
- **Test scenarios:**
  - All workspace tests green (excluding known pre-existing `make_list` failure)
  - Server startup + codex load succeeds
  - kcdata parser tests (12) green
  - assemble.rs tests green (merge logic intact)
- **Verification:** `cargo clippy --workspace --tests -- -D warnings` clean;
  `cargo test -p emukc_bootstrap` green; `cargo test -p emukc_gameplay --test sortie_battle`
  15/15 green

---

## Risks & Dependencies

- **Risk: agent output quality.** The agent may produce subtly incorrect route predicates
  or miss enemy compositions. Mitigated by U2 validation harness and the fact that map data
  is semi-static (updates are rare). The existing catalog JSON serves as a baseline for
  regression comparison.

- **Risk: `into_map_catalog()` dependency chain (resolved).** The conversion function
  `into_map_catalog_with_overlay()` was analyzed: it depends only on four `mod.rs`-local
  helpers (`ordered_route_targets`, `rewrite_route_predicate_labels`,
  `compact_enemy_composition`, `probability_to_weight`) — none of which are in the
  deletion set. `build_nodes()` runs during parsing (replaced by the agent), not
  during conversion. No helpers need to be extracted from the deletion set.

- **Risk: overlay extraction.** `into_map_catalog_with_overlay()` produces
  `WikiwikiMapOverlayCatalog` used by `assemble.rs`'s overlay merge path. The overlay
  extraction must continue working after parser deletion. The overlay types in
  `types.rs` are preserved.

- **Dependency:** `wikiwiki_map_download.rs` must continue to work — it downloads HTML
  pages that the agent skill consumes. It is not modified by this plan.

- **Dependency:** The `assets/wikiwiki_map_catalog.json` file remains the runtime asset.
  It is regenerated by the new workflow (agent skill → normalize CLI) but its format does
  not change.
