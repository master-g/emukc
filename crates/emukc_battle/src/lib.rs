//! Battle simulation engine for `KanColle`.
//!
//! Pure computation crate — takes `Codex` (read-only) and battle inputs,
//! produces battle simulation results. No database, HTTP, or side effects.

/// Internal battle configuration.
mod config;
mod damage;
/// Event types for the owned-pass architecture.
pub mod event;
/// Internal battle documentation.
mod outcome;
/// Random number generation trait and implementations for battle simulation.
pub mod random;
pub mod simulation;
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
    BattleKoukuStage3, BattleNightHougeki, BattleOpeningAttack, BattleOutcome, BattlePacket,
    BattleRaigeki, BattleRuntimeShip, BattleShipInput, BattleSimulation, BattleType,
    EngagementType, NightBattleInput, NightBattlePacket, NightBattleSimulation,
};

// Public API — RNG
pub use random::BattleRng;

// Public API — utilities
pub use damage::apply_cap;
pub use outcome::{calculate_mvp, calculate_win_rank};
pub use targeting::any_alive;

// Entry functions
pub use simulation::{simulate_day, simulate_night};

// Public API — transcript renderer
pub use transcript::{render_day_battle, render_night_battle};
