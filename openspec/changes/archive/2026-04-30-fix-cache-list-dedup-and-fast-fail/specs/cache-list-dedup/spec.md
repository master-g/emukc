## ADDED Requirements

### Requirement: CacheList deduplicates by path and version
`CacheList` SHALL treat two items with the same `path` and `version` as identical. Adding an item with a path+version combination already present in the list SHALL be a no-op (the existing entry is kept).

#### Scenario: Adding duplicate path with same version
- **WHEN** `CacheList::add("kcs2/resources/ship/full/0003_9511_mhqqhhvvpzxg.png", Some("1"))` is called twice
- **THEN** the list contains exactly one entry for that path+version

#### Scenario: Same path with different version
- **WHEN** `CacheList::add("kcs2/resources/ship/full/0003_9511_mhqqhhvvpzxg.png", Some("1"))` is called, then `CacheList::add("kcs2/resources/ship/full/0003_9511_mhqqhhvvpzxg.png", Some("2"))` is called
- **THEN** the list contains two entries for that path (one per version)

#### Scenario: Same path with None version vs Some version
- **WHEN** `CacheList::add("kcs2/resources/ship/full/0003_9511_mhqqhhvvpzxg.png", None)` is called twice
- **THEN** the list contains exactly one entry for that path

### Requirement: CacheListItem identity excludes sequential id
`CacheListItem` equality and ordering SHALL be based on `(path, version)` only. A sequential `id` field SHALL NOT exist on the struct.

#### Scenario: Generated cache list has no _id field
- **WHEN** a `CacheListItem` is serialized to JSON
- **THEN** the output contains `path` and optionally `version`, but no `_id` field

### Requirement: fetch_from_remote fails immediately on HTTP 404
When `fetch_from_remote` receives an HTTP 404 response from any CDN node, it SHALL return `FailedOnAllCdn` immediately without attempting remaining CDN nodes.

#### Scenario: First CDN returns 404
- **WHEN** `fetch_from_remote` requests a resource and the first CDN node returns HTTP 404
- **THEN** no other CDN nodes are tried, and `FailedOnAllCdn` is returned

#### Scenario: CDN returns non-404 error
- **WHEN** `fetch_from_remote` requests a resource and a CDN node returns a connection error or HTTP 5xx
- **THEN** the next CDN node is tried (existing fallback behavior preserved)

#### Scenario: CDN returns 200
- **WHEN** `fetch_from_remote` requests a resource and a CDN node returns HTTP 200
- **THEN** the resource is downloaded and returned (existing success behavior preserved)

## MODIFIED Requirements

### Requirement: resource-manifest fetch preserves CDN fallback for non-404 errors
The existing CDN fallback behavior (trying multiple CDN nodes) SHALL be preserved for connection errors, timeouts, and server errors (5xx). Only HTTP 404 triggers immediate failure.

#### Scenario: Connection timeout on first CDN, success on second
- **WHEN** `fetch_from_remote` requests a resource, the first CDN times out, and the second CDN returns 200
- **THEN** the resource is downloaded from the second CDN
