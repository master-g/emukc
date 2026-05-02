## ADDED Requirements

### Requirement: PracticeRepository trait for practice session storage

The system SHALL define a synchronous `PracticeRepository` trait in `crates/emukc_gameplay/src/game/battle/repository.rs` exposing the methods `get_pending_practice(profile_id)`, `insert_pending_practice(profile_id, session)`, and `take_pending_practice(profile_id)`. The trait SHALL NOT use `#[async_trait]`; all methods are synchronous. The trait SHALL replace the process-global `PENDING_PRACTICE_BATTLES` static `Mutex<HashMap<…>>` as the storage seam for pending practice battle sessions.

#### Scenario: Production uses GlobalPracticeStore

- **WHEN** the server starts in production mode
- **THEN** `HasContext::practice_store()` SHALL return a reference to a `GlobalPracticeStore` instance implementing `PracticeRepository`
- **THEN** `GlobalPracticeStore` SHALL encapsulate any internal mutex; callers SHALL NOT lock anything externally

#### Scenario: Tests use isolated practice store

- **WHEN** a gameplay integration test creates a `TestPracticeStore` implementing `PracticeRepository`
- **THEN** practice battle operations in that test SHALL read/write only that isolated store
- **THEN** concurrent tests SHALL NOT observe each other's practice session state

#### Scenario: HasContext requires explicit practice_store

- **WHEN** a type implements `HasContext`
- **THEN** it SHALL provide an explicit `fn practice_store(&self) -> &dyn PracticeRepository` implementation
- **THEN** there SHALL be no default implementation that falls back to a global static

### Requirement: PracticeRepository operations are atomic within the store

Each operation on `PracticeRepository` SHALL be internally Mutex-protected (or equivalent) such that concurrent access from multiple Tokio tasks does not cause data races. The repository SHALL NOT require external locking by callers.

#### Scenario: Concurrent practice starts from different profiles

- **WHEN** two Tokio tasks simultaneously call `insert_pending_practice` for different `profile_id` values
- **THEN** both operations SHALL succeed without deadlock or data corruption

#### Scenario: take_pending_practice removes and returns

- **WHEN** `take_pending_practice(profile_id)` is called after a prior `insert_pending_practice(profile_id, session)`
- **THEN** the call SHALL return `Some(session)` matching the inserted value
- **THEN** a subsequent call to `get_pending_practice(profile_id)` SHALL return `None`

### Requirement: PENDING_PRACTICE_BATTLES static removed

The global `PENDING_PRACTICE_BATTLES` static `Mutex<HashMap<…>>` previously declared in the practice battle module SHALL be removed from public scope. Any remaining storage required by `GlobalPracticeStore` SHALL be a private field of that struct, not a process-global symbol.

#### Scenario: No public reference to PENDING_PRACTICE_BATTLES remains

- **WHEN** `cargo doc --workspace --no-deps` is generated
- **THEN** no public symbol named `PENDING_PRACTICE_BATTLES` SHALL be present in the documentation
- **THEN** `grep -r "PENDING_PRACTICE_BATTLES" crates/` SHALL return zero matches outside `GlobalPracticeStore`'s private internals

### Requirement: Practice night battle surfaces engagement decode failures

When the stored formation tuple in a `PracticeBattleSession` cannot be decoded into a valid `EngagementType`, the practice night battle entry point SHALL log the corruption at error level via `tracing::error!` and SHALL return `None` instead of silently coercing the value to `EngagementType::SameCourse`.

#### Scenario: Corrupt stored engagement value

- **WHEN** `run_night_battle` is invoked for a profile whose stored `session.formation[2]` is not a valid engagement id
- **THEN** the function SHALL log `tracing::error!` containing the profile id and the offending value
- **THEN** the function SHALL return `None`
- **THEN** the function SHALL NOT mutate the stored session

#### Scenario: Valid stored engagement value

- **WHEN** `run_night_battle` is invoked for a profile whose stored `session.formation[2]` decodes to a valid engagement id
- **THEN** the function SHALL proceed with that engagement
- **THEN** no error is logged
