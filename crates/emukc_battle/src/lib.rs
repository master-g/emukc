//! Battle simulation engine for `KanColle`.
//!
//! Pure computation crate — takes `Codex` (read-only) and battle inputs,
//! produces battle simulation results. No database, HTTP, or side effects.

/// Combined fleet (連合艦隊) formation multipliers and attack corrections.
pub mod combined;
/// Rewrites a combined-fleet packet into the client's ship index space.
mod combined_packet;
/// Internal battle configuration.
mod config;
mod damage;
/// Debug overlay: overrides simulation results for god mode / one hit kill.
mod debug_overlay;
mod execution;
/// Internal battle documentation.
mod outcome;
/// Random number generation trait and implementations for battle simulation.
pub mod random;
mod simulation;
mod state;
mod targeting;
/// Deterministic text renderer for battle simulations.
pub mod transcript;
#[expect(missing_docs)]
mod types;

#[cfg(test)]
mod test_utils;

// Public API — types
pub use types::{
    AirState, BattleContext, BattleHougeki, BattleKouku, BattleKoukuStage1, BattleKoukuStage2,
    BattleKoukuStage3, BattleKoukuStage3Combined, BattleNightHougeki, BattleOpeningAttack,
    BattleOutcome, BattlePacket, BattleRaigeki, BattleRuntimeShip, BattleShipInput,
    BattleSimulation, BattleType, CombinedSetup, EngagementType, NightBattleInput,
    NightBattlePacket, NightBattleSimulation, SiListId,
};

// Public API — combined fleet tables
pub use combined::{
    CombinedAttackClass, CombinedFleetRole, CombinedType, ESCORT_INDEX_OFFSET,
    combined_correction_vs_single, combined_formation_min_escort_size, combined_formation_modifier,
};

// Public API — RNG
pub use random::BattleRng;

// Public API — utilities
pub use damage::apply_cap;
// Public API — complete battle execution
pub use execution::{SpMidnightSimulation, execute_day, execute_night, execute_sp_midnight};
pub use outcome::{calculate_mvp, calculate_win_rank};
pub use targeting::any_alive;

// Public API — transcript renderer
pub use transcript::{render_day_battle, render_night_battle};
