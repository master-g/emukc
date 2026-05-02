## ADDED Requirements

### Requirement: SortieRepository trait for explicit state dependency
The system SHALL define a synchronous `SortieRepository` trait with methods for active sortie lifecycle management (`get_active`, `insert_active`, `remove_active`) and pending battle/result management (`get_pending_battle`, `insert_pending_battle`, `take_pending_battle`, `get_pending_result`, `insert_pending_result`, `take_pending_result`). The trait SHALL NOT use `#[async_trait]` — all methods are synchronous.

#### Scenario: Production uses GlobalSortieStore
- **WHEN** the server starts in production mode
- **THEN** `HasContext::sortie_store()` SHALL return a reference to the global `SortieStore` instance implementing `SortieRepository`

#### Scenario: Tests use isolated store
- **WHEN** a gameplay integration test creates a `TestSortieStore` implementing `SortieRepository`
- **THEN** sortie operations SHALL read/write only that isolated store
- **THEN** concurrent tests SHALL NOT observe each other's sortie state

#### Scenario: HasContext requires explicit sortie_store
- **WHEN** a type implements `HasContext`
- **THEN** it SHALL provide an explicit `fn sortie_store(&self) -> &dyn SortieRepository` implementation
- **THEN** there SHALL be no default implementation relying on a global static

### Requirement: SortieRepository operations are atomic within the store
Each operation on `SortieRepository` SHALL be internally Mutex-protected (or equivalent) such that concurrent access from multiple Tokio tasks does not cause data races. The repository SHALL NOT require external locking by callers.

#### Scenario: Concurrent sortie starts from different profiles
- **WHEN** two Tokio tasks simultaneously call `insert_active` for different `profile_id` values
- **THEN** both operations SHALL succeed without deadlock or data corruption

#### Scenario: take_pending_battle removes and returns
- **WHEN** `take_pending_battle(profile_id)` is called
- **THEN** a subsequent call to `get_pending_battle(profile_id)` SHALL return `None`
- **THEN** the returned session SHALL be the one previously inserted
