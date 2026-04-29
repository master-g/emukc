## ADDED Requirements

### Requirement: Decoder emits template-backed resource family metadata
The decoder SHALL emit structured metadata for resource families whose path shape is observable in decoded `main.js` as a deterministic template but whose member set depends on runtime bootstrap inputs. Each template-backed family MUST record a stable family key, resource domain, path template, required input bindings, coverage mode, and decoded-module provenance.

#### Scenario: Deterministic template is observed
- **WHEN** decoded modules expose a deterministic resource path formula for map, gauge-adjacent map, furniture, BGM, sound bucket, titlecall, useitem, area, or world-select resources
- **THEN** the decoder output MUST represent that formula as a template-backed family with stable family identity and path-template metadata
- **THEN** the decoder output MUST record the decoded module provenance that supports the template

#### Scenario: Template needs runtime membership input
- **WHEN** the decoder can prove a path template but cannot enumerate the full member set from decoded `main.js` alone
- **THEN** the decoder output MUST declare the required runtime input binding for that template-backed family
- **THEN** the decoder output MUST NOT synthesize the missing member set by copying Rust fallback constants, CDN-derived lists, or generated cache-list output

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
