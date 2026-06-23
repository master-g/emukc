---
name: emukc-api-development
description: Implement or maintain KanColle server APIs in the emukc project following its layered architecture (database → model → gameplay → API handler). Use this skill when the user asks to implement new KCS APIs, add game features, fix API bugs, refactor API structure, or says "implement according to the plan" for emukc. Also trigger when working with preset systems, fleet management, equipment development, or any KanColle game mechanics in the emukc codebase.
---

# EmuKC API Development

This skill guides you through implementing or maintaining APIs in the emukc project, a KanColle (Kantai Collection) server emulator written in Rust.

## Architecture Overview

EmuKC follows a strict 4-layer architecture. Always implement changes in this order:

1. **Database Layer** (`crates/emukc_db/src/entity/`)
   - SeaORM entity models
   - Database schema definitions
   - Relations between entities

2. **Model Layer** (`crates/emukc_model/src/`)
   - Business domain types
   - Conversion logic between DB models and API types
   - KC API response structures

3. **Gameplay Layer** (`crates/emukc_gameplay/src/game/`)
   - Core game logic implementation
   - Trait definitions for game operations
   - Transaction management

4. **API Handler Layer** (`src/bin/net/router/kcsapi/`)
   - HTTP request handlers
   - Parameter parsing
   - Response formatting

## Implementation Pattern

When implementing a new feature, follow this systematic approach:

### 1. Database Layer

**Create entity model:**
```rust
// crates/emukc_db/src/entity/profile/feature_name.rs
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "table_name")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub profile_id: i64,
    // ... other fields
}
```

**Update parent module:**
- Add `pub mod feature_name;` to the parent `mod.rs`
- Add table creation in `bootstrap()` function if needed
- Update `wipe()` function to clean up records

### 2. Model Layer

**Create domain types:**
```rust
// crates/emukc_model/src/profile/feature_name.rs
use serde::{Deserialize, Serialize};
use crate::kc2::KcApiFeatureName;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FeatureElement {
    pub field1: i64,
    pub field2: String,
}

impl From<FeatureElement> for KcApiFeatureElement {
    fn from(value: FeatureElement) -> Self {
        Self {
            api_field1: value.field1,
            api_field2: value.field2,
        }
    }
}
```

**Add KC API types:**
```rust
// crates/emukc_model/src/kc2/api/mod.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KcApiFeatureElement {
    pub api_field1: i64,
    pub api_field2: String,
}
```

**Export in prelude if needed:**
```rust
// crates/emukc_model/src/lib.rs
pub mod prelude {
    pub use crate::{
        // ... existing exports
        profile::feature_name::FeatureElement,
    };
}
```

### 3. Gameplay Layer

**Implement core logic:**
```rust
// crates/emukc_gameplay/src/game/feature/mod.rs
pub(crate) async fn get_feature_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<Vec<Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let items = Entity::find()
        .filter(Column::ProfileId.eq(profile_id))
        .all(c)
        .await?;
    Ok(items)
}
```

**Define trait methods:**
```rust
#[async_trait]
pub trait FeatureOps {
    async fn get_feature(&self, profile_id: i64) -> Result<Feature, GameplayError>;
    async fn update_feature(&self, profile_id: i64, data: &FeatureData) -> Result<(), GameplayError>;
}
```

**Implement trait:**
```rust
#[async_trait]
impl<T: HasContext + ?Sized> FeatureOps for T {
    async fn get_feature(&self, profile_id: i64) -> Result<Feature, GameplayError> {
        let db = self.db();
        let tx = db.begin().await?;

        let result = get_feature_impl(&tx, profile_id).await?;

        tx.commit().await?;
        Ok(result.into())
    }
}
```

### 4. API Handler Layer

**Create handler:**
```rust
// src/bin/net/router/kcsapi/api_get_member/feature.rs
use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
};
use emukc::prelude::FeatureOps;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    api_param1: i64,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;

    let result = state.get_feature(pid).await?;
    let resp: KcApiFeature = result.into();

    Ok(KcApiResponse::success(&resp))
}
```

**Register route:**
```rust
// src/bin/net/router/kcsapi/api_get_member/mod.rs
mod feature;

pub(super) fn router() -> Router {
    Router::new()
        // ... existing routes
        .route("/feature", post(feature::handler))
}
```

## Common Patterns

### Pattern: Reference Existing Code

When implementing a new feature, always find a similar existing feature and follow its pattern:

- For preset systems → look at `preset_deck.rs` or `preset_slot.rs`
- For fleet operations → look at `fleet/mod.rs`
- For equipment → look at `slot_item/mod.rs`
- For construction → look at `kdock/mod.rs`

Read the reference implementation carefully and adapt it to your needs.

### Pattern: Handle Imports Correctly

**Common import issues:**

1. **Trait not in scope** - Add trait import to handler:
   ```rust
   use emukc::prelude::FeatureOps;
   ```

2. **Type not exported** - Add to model prelude:
   ```rust
   // crates/emukc_model/src/lib.rs
   pub mod prelude {
       pub use crate::profile::feature::FeatureElement;
   }
   ```

3. **Module not declared** - Add to parent mod.rs:
   ```rust
   pub mod feature;
   ```

### Pattern: Error Handling

Use appropriate error variants from `GameplayError`:

```rust
// Entry not found
Err(GameplayError::EntryNotFound(format!("feature for profile {profile_id}")))

// Insufficient resources
Err(GameplayError::Insufficient("not enough fuel".to_string()))

// Capacity exceeded
Err(GameplayError::CapacityExceeded(current_count))
```

### Pattern: Database Transactions

For operations that modify data, always use transactions:

```rust
let db = self.db();
let tx = db.begin().await?;

// ... perform operations on &tx

tx.commit().await?;
```

### Pattern: Active Model Updates

When updating existing records:

```rust
let record = Entity::find()
    .filter(Column::ProfileId.eq(profile_id))
    .one(c)
    .await?
    .ok_or_else(|| GameplayError::EntryNotFound(...))?;

let mut am = record.into_active_model();
am.field = ActiveValue::Set(new_value);

let m = match am.id {
    ActiveValue::NotSet => am.insert(c).await?,
    _ => am.update(c).await?,
};
```

## Compilation Error Resolution

When you encounter compilation errors, follow this systematic approach:

1. **Read the error carefully** - Understand what the compiler is telling you
2. **Check imports** - Missing trait imports are the most common issue
3. **Verify exports** - Ensure types are exported in preludes
4. **Check module declarations** - Ensure all new modules are declared
5. **Fix one error at a time** - Start with the first error, rebuild, repeat

Common error patterns:

- `no method named 'X' found` → Trait not imported in handler
- `cannot find type 'X'` → Type not exported in prelude
- `use of undeclared module` → Module not added to parent mod.rs
- `no method named 'try_into_model'` → Use `insert()` or `update()` instead of `save()`

## API List Maintenance

After implementing APIs, update the API list:

```markdown
# apilist.md
api_get_member/feature
api_req_category/feature_action
```

Keep the list alphabetically organized within each section.

## Testing Approach

After implementation:

1. **Build verification**: `cargo build` must succeed with no errors
2. **Type checking**: Ensure all types are correctly defined
3. **Manual testing**: Start server and test endpoints (TODO: automated tests)

## Key Principles

1. **Follow existing patterns** - The codebase has established conventions; don't invent new ones
2. **Maintain layer separation** - Never skip layers or mix concerns
3. **Use transactions** - Wrap multi-step operations in database transactions
4. **Handle errors properly** - Use appropriate error types and provide context
5. **Keep it minimal** - Write only the code needed to solve the problem
6. **Reference similar code** - Find and study similar existing implementations

## Workflow Summary

For any new API implementation:

1. Identify a similar existing feature to use as reference
2. Create database entity and update bootstrap/wipe
3. Create model types and KC API types
4. Implement gameplay logic with trait methods
5. Create API handlers and register routes
6. Update API list
7. Build and fix any compilation errors systematically
8. Verify the implementation works

Remember: The architecture is strict but logical. Each layer has a clear responsibility. Follow the pattern, reference existing code, and the implementation will be clean and maintainable.
