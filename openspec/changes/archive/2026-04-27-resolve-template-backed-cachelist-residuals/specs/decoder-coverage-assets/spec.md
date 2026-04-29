## MODIFIED Requirements

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
