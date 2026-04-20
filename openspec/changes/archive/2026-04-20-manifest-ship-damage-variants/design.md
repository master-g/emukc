## Context

The Manifest strategy ship path generator (`crates/emukc_bootstrap/src/make_list/manifest/generate.rs`) currently produces one path per ship per target type. The Default strategy (`source/kcs2/resources/ship.rs`) generates multiple variants: `_dmg`, `_g_dmg`, `_g` counterparts for many base categories. The coverage analysis shows 13,987 ship paths missing because the Manifest generator does not produce these variants.

The `resource_manifest.json` has a `damagedSource` field per ship entry with values like `"false"`, `"true"`, `"damaged"`, `"_0x..."`, `"this._ship_damaged"`, etc. The current resolver only handles `"false"` → no damage, `"true"` → damage. All other values return `None` (meaning "generate both"), but the generator ignores this signal and only emits the base category path.

## Goals / Non-Goals

**Goals:**
- Generate damage variant paths (`_dmg`, `_g_dmg`, `_g`) for ship categories that have them
- Resolve `damagedSource` correctly: `"false"` → base only, everything else → base + all applicable damage variants
- Increase Manifest strategy ship coverage from ~46% to ~90%+

**Non-Goals:**
- Changing Default or Greedy strategy behavior
- Handling `kcs/` legacy, `kcs2/img/`, or other uncovered resource types
- Modifying `resource_manifest.json` extraction (main-decoder side)

## Decisions

### 1. Static variant mapping table

**Decision**: Define a `const` mapping from base ship categories to their damage variants in `generate.rs`.

```rust
const SHIP_DAMAGE_VARIANTS: &[(&str, &[&str])] = &[
    ("banner",      &["banner_dmg", "banner_g_dmg", "banner_g"]),
    ("banner2",     &["banner2_dmg", "banner2_g_dmg", "banner2_g"]),
    ("banner3",     &["banner3_g_dmg", "banner3_g"]),
    ("card",        &["card_dmg"]),
    ("full",        &["full_dmg"]),
    ("character_full", &["character_full_dmg"]),
    ("character_up",   &["character_up_dmg"]),
    ("remodel",     &["remodel_dmg"]),
    ("supply_character", &["supply_character_dmg"]),
];
```

**Rationale**: The variant patterns are fixed by the game client. A static table is simpler and more maintainable than inferring variants from naming conventions. Matches the Default strategy's explicit enumeration.

**Alternative**: Parse variant names from the manifest entries. Rejected — the manifest doesn't enumerate variants, it only has a single `targetType` per entry.

### 2. `damagedSource` resolution semantics

**Decision**:
- `"false"` → generate only base path (no damage variants)
- `"true"` → generate only damage variant paths (for the `full`/`full_dmg` pattern where the manifest entry itself targets a specific damaged state)
- Everything else (`"damaged"`, `"_0x..."`, `"this._ship_damaged"`, etc.) → generate base path AND all damage variants

**Rationale**: The `damagedSource` field indicates whether the caller passes a damaged flag. When it's a static `"false"`, the code path never loads damage art. When it's a static `"true"`, the code path only loads damage art. When it's a variable expression, the runtime evaluates it — both normal and damaged paths may be loaded.

### 3. No changes to resolver API

**Decision**: Keep `resolve_damaged()` returning `Option<bool>`. Add variant expansion logic in `generate_ship_paths()` only.

**Rationale**: The resolver's job is interpreting the expression. The generator's job is producing paths. Mixing variant expansion into the resolver would break separation of concerns.

## Risks / Trade-offs

**[Over-generation]** → The Manifest strategy will generate paths for damage variants that don't exist on CDN for some ships. This is acceptable — the Default strategy already generates these paths and relies on the cache populate step to handle 404s. The Manifest strategy is meant to be comprehensive, not minimal.

**[Future game updates adding new variant patterns]** → New damage variant naming conventions would require updating the static table. Low risk — KanColle's variant naming has been stable for years.

**[Manifest entries with `damagedSource = "true"` generating wrong paths]** → When `damagedSource` is explicitly `"true"`, the manifest entry's `targetType` may already be the damaged variant (e.g., targetType `"full"` with damagedSource `"true"` means the code loads `full_dmg`). Need to handle this case by not double-appending `_dmg`.
