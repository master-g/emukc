//! Game configuration.

use serde::{Deserialize, Serialize};

use crate::profile::material::MaterialConfig;

/// Picture book configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PicturebookConfig {
    /// unveiled all ships in picturebook
    pub unlock_all_ships: bool,

    /// unveiled all equipments in picturebook
    pub unlock_all_slotitems: bool,
}

/// EmuKC is a single-player emulator; the picture book defaults to fully unlocked because
/// gating it behind in-game progress is not the typical user expectation.
/// Override in `[game.picturebook]` in `emukc.config.toml` to opt into the original gating.
#[expect(clippy::doc_markdown)]
impl Default for PicturebookConfig {
    fn default() -> Self {
        Self {
            unlock_all_ships: true,
            unlock_all_slotitems: true,
        }
    }
}

/// Repair config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DockingConfig {
    /// Repair time factor
    pub time_factor: f64,

    /// Repair cost factor
    pub cost_factor: f64,
}

impl Default for DockingConfig {
    fn default() -> Self {
        Self {
            time_factor: 1.0,
            cost_factor: 1.0,
        }
    }
}

/// Experience configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExpConfig {
    /// CT flagship exp multiplier for sortie.
    pub ct_exp_boost: f64,

    /// Additional exp multiplier for practice battles.
    pub practice_exp_boost: f64,
}

impl Default for ExpConfig {
    fn default() -> Self {
        Self {
            ct_exp_boost: 1.0,
            practice_exp_boost: 1.0,
        }
    }
}

/// Game configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GameConfig {
    /// Material configuration.
    pub material: MaterialConfig,

    /// Picture book for ships.
    pub picturebook: PicturebookConfig,

    /// Docking configuration.
    pub docking: DockingConfig,

    /// Experience configuration.
    #[serde(default)]
    pub exp: ExpConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_config_default_pins_no_boost() {
        let cfg = ExpConfig::default();
        assert_eq!(cfg.ct_exp_boost, 1.0);
        assert_eq!(cfg.practice_exp_boost, 1.0);
    }

    #[test]
    fn docking_config_default_pins_no_adjustment() {
        let cfg = DockingConfig::default();
        assert_eq!(cfg.time_factor, 1.0);
        assert_eq!(cfg.cost_factor, 1.0);
    }

    #[test]
    fn picturebook_config_default_documented() {
        let cfg = PicturebookConfig::default();
        // Only verify the fields compile and are accessible; bool values are NOT pinned.
        let _ = cfg.unlock_all_ships;
        let _ = cfg.unlock_all_slotitems;
    }
}
