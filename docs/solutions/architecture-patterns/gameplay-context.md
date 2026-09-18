---
title: "Gameplay context: inherent methods on a concrete Ctx, not a single-implementation trait layer"
date: 2026-09-18
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Adding a new gameplay operation to an existing or new domain"
  - "Deciding where the database connection, codex or runtime stores come from"
  - "Wiring a new owning context (server state, test harness, CLI tool)"
tags: [gameplay, ctx, context, inherent-methods, transactions, connectiontrait, deref]
related_components: [emukc_db, emukc_model]
---

# Gameplay context: inherent methods on a concrete Ctx, not a single-implementation trait layer

## Context

Gameplay used to expose 24 `XxxOps` traits (`MaterialOps`, `ShipOps`, `QuestOps`,
…), two composition traits (`GameOps`, `Gameplay`) and a `HasContext` accessor
trait. Every one of the 144 operations was written twice — once as a trait
signature, once in a `impl<T: HasContext + ?Sized> XxxOps for T` blanket impl —
and each trait carried `#[async_trait]`, so every call paid a `Box::pin`
allocation for polymorphism the repository never used: there was no `dyn Ops`
anywhere, and each trait had exactly one implementation.

The trait layer's only real product was call syntax: roughly 500 call sites
outside the crate write `state.get_materials(pid)` rather than
`get_materials(&state, pid)`.

## Guidance

### The context is a concrete type

`gameplay::Ctx` holds everything an operation needs:

```rust
pub struct Ctx {
    pub db: Arc<DbConn>,
    pub codex: Arc<Codex>,
    pub sortie_store: Arc<SortieStore>,
    pub practice_store: Arc<PracticeStore>,
}
```

Gameplay operations SHALL be inherent `pub async fn`s in an `impl Ctx` block in
their domain module. One signature, one body, no `#[async_trait]`: `Send` is
inferred by the compiler.

### Owning contexts embed and `Deref`

Types that own a context — the server `State`, the integration `TestContext`,
the `battle sim` `SimContext` — SHALL hold a `Ctx` field and implement
`Deref<Target = Ctx>`. Method resolution then reaches the inherent methods by
autoderef, which is what keeps `state.foo(..)` call sites unchanged. `Ctx` also
keeps `db()`, `codex()`, `sortie_store()` and `practice_store()` accessors for
the same reason.

A context MUST NOT be a tuple or any other foreign type: inherent methods and
`Deref` both require a local type.

### Runtime stores are per-context

`Ctx::new(db, codex)` builds a fresh `SortieStore` and `PracticeStore`. There is
no process-global fallback, so tests running in parallel cannot collide on
profile ids through a shared store.

### `_impl` functions stay generic over the connection

The `_impl` convention is unchanged and is NOT an artifact of the trait layer:

```rust
pub(crate) async fn add_material_impl<C>(c: &C, ..) -> Result<..>
where
    C: ConnectionTrait,
```

The inherent method owns the transaction (`db.begin()` … `tx.commit()`); the
`_impl` function takes `&C` so it can run either on a `DbConn` or inside a
`DatabaseTransaction` opened by another domain. Cross-domain writes SHALL go
through `_impl` functions, never by calling another inherent method, because a
nested inherent call would open a second transaction.

## Rationale

- A trait with one implementation is two signatures to keep in sync and nothing
  else; deleting the layer removed ~2600 lines with no behavior change.
- `#[async_trait]` boxes every future. Inherent `async fn` does not.
- rustdoc and editor "go to definition" land on the body instead of on a
  signature that forwards to a blanket impl.
- A generic `fn foo<C: HasContext>(ctx: &C)` cannot be given a new operation
  without touching the trait; `&Ctx` can.

## Anti-patterns

- Re-introducing a trait to "allow mocking". Tests build a real `Ctx` over an
  in-memory database (`new_mem_db()`); that is cheaper and catches SQL errors.
- Calling one inherent method from another inside a transaction. Use the
  `_impl` function.
- Reading `db`/`codex` from a global. They live on the context.

## Applies when

- Adding a game API (see `CLAUDE.md`, *Adding a New Game API*).
- Adding a new owning context.

## See also

- `docs/plans/2026-09-18-001-refactor-eliminate-boilerplate-plan.md` — the plan
  that removed the trait layer (units U6-U8).
