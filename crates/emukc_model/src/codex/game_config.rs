//! Game configuration.

use serde::{Deserialize, Serialize};

use crate::profile::material::MaterialConfig;

/// Picture book configuration
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PicturebookConfig {
    /// unveiled all ships in picturebook
    pub unlock_all_ships: bool,

    /// unveiled all equipments in picturebook
    pub unlock_all_slotitems: bool,
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
    #[serde(default = "default_ct_exp_boost")]
    pub ct_exp_boost: f64,

    /// Additional exp multiplier for practice battles.
    #[serde(default = "default_f64_1")]
    pub practice_exp_boost: f64,
}

impl Default for ExpConfig {
    fn default() -> Self {
        Self {
            ct_exp_boost: default_ct_exp_boost(),
            practice_exp_boost: default_f64_1(),
        }
    }
}

fn default_ct_exp_boost() -> f64 {
    1.0
}

fn default_f64_1() -> f64 {
    1.0
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
