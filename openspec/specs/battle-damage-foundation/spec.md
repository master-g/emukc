## ADDED Requirements

### Requirement: Phase damage reports effective damage after sinking protection

All battle phase output arrays (hougeki `api_damage`, torpedo `api_fydam`/`api_eydam`/`api_fdam`/`api_edam`, kouku `api_fdam`/`api_edam`, OASW `api_damage`, night battle `api_damage`) SHALL contain the effective damage value returned by `apply_damage()` — the actual HP subtracted after sinking protection — NOT the raw pre-protection damage.

When sinking protection triggers for a protected ship (flagship or non-taiha-at-entry during sortie), the reported damage SHALL equal the proportional damage that was actually applied, which is less than the raw calculated damage.

This ensures the client's sequential HP animation (initial HP minus per-phase cumulative damage) matches the server's actual HP state at every phase boundary.

#### Scenario: Flagship survives lethal damage via sinking protection
- **WHEN** a flagship with 50 HP takes 80 raw damage during shelling
- **THEN** sinking protection converts the damage to proportional (e.g., 25)
- **THEN** `api_damage` for that attack reports 25 (NOT 80)

#### Scenario: Non-taiha friendly ship survives via sinking protection in torpedo phase
- **WHEN** a non-taiha friendly ship at sortie entry takes lethal damage in opening torpedo
- **THEN** sinking protection reduces the damage to guarantee survival
- **THEN** `api_fydam`/`api_eydam` report the reduced effective damage

#### Scenario: Kouku airstrike against protected friendly ship
- **WHEN** an airstrike would deal lethal damage to a protected friendly ship
- **THEN** `api_fdam` for that ship position reports the effective post-protection damage

#### Scenario: No sinking protection needed (damage below lethal)
- **WHEN** raw damage is less than the target's current HP
- **THEN** effective damage equals raw damage
- **THEN** reported damage equals raw damage (no change in behavior)

#### Scenario: Enemy ship takes damage (no protection)
- **WHEN** an enemy ship takes damage
- **THEN** effective damage equals raw damage clamped to current HP
- **THEN** reported damage equals the clamped value
