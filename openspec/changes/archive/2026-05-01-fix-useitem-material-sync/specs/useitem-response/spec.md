## ADDED Requirements

### Requirement: Useitem response derives special resources from material table
The `api_get_member/useitem` and `api_get_member/require_info` endpoints SHALL return bucket, torch, devmat, and screw counts sourced from the `material` table, not the `use_item` table. Other use items SHALL continue to be sourced from the `use_item` table.

#### Scenario: Useitem includes correct bucket count after expedition reward
- **WHEN** an expedition grants bucket+2 via `add_material_impl` (which only updates the material table)
- **AND** the client calls `api_get_member/useitem`
- **THEN** the response entry with `api_id=1` (Bucket) SHALL have `api_count` equal to the current `material.bucket` value

#### Scenario: Useitem includes correct torch count after instant construction
- **WHEN** a ship is constructed consuming torch via `deduct_material_impl` (which only updates the material table)
- **AND** the client calls `api_get_member/useitem`
- **THEN** the response entry with `api_id=2` (Torch) SHALL have `api_count` equal to the current `material.torch` value

#### Scenario: Useitem includes correct devmat count
- **WHEN** the client calls `api_get_member/useitem`
- **THEN** the response entry with `api_id=3` (DevMat) SHALL have `api_count` equal to the current `material.devmat` value

#### Scenario: Useitem includes correct screw count
- **WHEN** the client calls `api_get_member/useitem`
- **THEN** the response entry with `api_id=4` (Screw) SHALL have `api_count` equal to the current `material.screw` value

#### Scenario: Non-material use items remain unchanged
- **WHEN** the client calls `api_get_member/useitem`
- **THEN** response entries with `api_id` values other than 1, 2, 3, 4 SHALL be sourced from the `use_item` table as before

#### Scenario: Missing use_item records for special resources
- **WHEN** the `use_item` table has no record for bucket/torch/devmat/screw for a profile
- **AND** the client calls `api_get_member/useitem`
- **THEN** the response SHALL still include entries for these 4 items with counts from the material table

#### Scenario: Require info includes correct special resource counts
- **WHEN** the client calls `api_get_member/require_info`
- **THEN** the `api_useitem` field SHALL include bucket/torch/devmat/screw entries with counts from the material table, consistent with the useitem endpoint

#### Scenario: Consume use item exchange grants correct material count
- **WHEN** a medal is consumed for bucket reward via `consume_use_item_impl`
- **AND** the client calls `api_get_member/useitem`
- **THEN** the response entry with `api_id=1` (Bucket) SHALL have `api_count` equal to the current `material.bucket` value (reflecting the added buckets)
