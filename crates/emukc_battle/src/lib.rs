//! Battle simulation engine for KanColle.
//!
//! Pure computation crate — takes `Codex` (read-only) and battle inputs,
//! produces battle simulation results. No database, HTTP, or side effects.

mod config;
mod damage;
mod outcome;
pub mod random;
pub mod simulation;
mod state;
mod targeting;
mod types;

#[cfg(test)]
mod test_utils;

// Public API — types
pub use types::{
    AirState, BattleContext, BattleHougeki, BattleKouku, BattleKoukuStage1, BattleKoukuStage2,
    BattleKoukuStage3, BattleNightHougeki, BattleOpeningAttack, BattleOutcome, BattlePacket,
    BattleRaigeki, BattleRuntimeShip, BattleShipInput, BattleSimulation, BattleType,
    EngagementType, NightBattlePacket, NightBattleSimulation,
};

// Public API — RNG
pub use random::BattleRng;

// Public API — utilities
pub use damage::apply_cap;
pub use outcome::{calculate_mvp, calculate_win_rank};
pub use targeting::any_alive;

// Entry functions
pub use simulation::{simulate_day, simulate_night};
