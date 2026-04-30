## MODIFIED Requirements

### Requirement: fetch_from_remote fails immediately on HTTP 404
When `fetch_from_remote` in `emukc_cache::kache` receives an HTTP 404 response from any CDN node, it SHALL return `FailedOnAllCdn` immediately without attempting remaining CDN nodes. Non-404 errors (connection failures, timeouts, 5xx) continue to fall through to the next CDN node as before.

#### Scenario: First CDN returns 404
- **WHEN** `fetch_from_remote` requests a resource and the first CDN node returns HTTP 404
- **THEN** no other CDN nodes are tried, and `FailedOnAllCdn` is returned

#### Scenario: Connection timeout on first CDN, success on second
- **WHEN** `fetch_from_remote` requests a resource, the first CDN times out, and the second CDN returns 200
- **THEN** the resource is downloaded from the second CDN (existing fallback preserved)
