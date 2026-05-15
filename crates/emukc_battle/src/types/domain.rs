//! Domain enums, value objects, and parameter structs.
//! No Serialize derivations — these are pure computation types.

/// Controls which phases execute in a day battle simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleType {
    /// Normal day battle: kouku -> OASW -> opening torpedo -> shelling x 2 -> closing torpedo.
    Normal,
    /// Air battle only (航空戦): kouku + OASW, no shelling / torpedo.
    AirBattle,
    /// Long-distance air raid (長距離空襲): kouku only, no OASW / shelling / torpedo.
    LdAirBattle,
    /// Long-distance shelling (長距離砲撃): shelling only, no kouku / torpedo.
    LdShooting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngagementType {
    SameCourse,
    HeadOn,
    TAdvantage,
    TDisadvantage,
}

impl EngagementType {
    pub const fn api_id(self) -> i64 {
        match self {
            Self::SameCourse => 1,
            Self::HeadOn => 2,
            Self::TAdvantage => 3,
            Self::TDisadvantage => 4,
        }
    }

    pub const fn modifier(self) -> f64 {
        match self {
            Self::SameCourse => 1.0,
            Self::HeadOn => 0.8,
            Self::TAdvantage => 1.2,
            Self::TDisadvantage => 0.6,
        }
    }

    /// Parse from `KanColle` API engagement ID (1–4).
    pub const fn from_api_id(api_id: i64) -> Option<Self> {
        match api_id {
            1 => Some(Self::SameCourse),
            2 => Some(Self::HeadOn),
            3 => Some(Self::TAdvantage),
            4 => Some(Self::TDisadvantage),
            _ => None,
        }
    }
}

/// Air superiority state after the kouku (aerial combat) phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirState {
    Supremacy,
    Superiority,
    Parity,
    Denial,
    Incapability,
}

impl AirState {
    pub(crate) fn from_power(friendly: i64, enemy: i64) -> Self {
        if enemy == 0 && friendly == 0 {
            return Self::Parity;
        }
        if enemy == 0 {
            return Self::Supremacy;
        }
        if friendly >= 3 * enemy {
            Self::Supremacy
        } else if 2 * friendly >= 3 * enemy {
            Self::Superiority
        } else if 3 * friendly <= enemy {
            Self::Incapability
        } else if 3 * friendly <= 2 * enemy {
            Self::Denial
        } else {
            Self::Parity
        }
    }

    pub(crate) fn api_disp_seiku(self) -> i64 {
        match self {
            Self::Supremacy => 1,
            Self::Superiority => 2,
            Self::Parity => 0,
            Self::Denial => 3,
            Self::Incapability => 4,
        }
    }

    /// Parse from `KanColle` API `api_disp_seiku` value.
    pub fn from_api_disp_seiku(value: i64) -> Option<Self> {
        match value {
            1 => Some(Self::Supremacy),
            2 => Some(Self::Superiority),
            0 => Some(Self::Parity),
            3 => Some(Self::Denial),
            4 => Some(Self::Incapability),
            _ => None,
        }
    }

    pub(crate) fn stage1_friendly_loss_ratio(self) -> (f64, f64) {
        match self {
            Self::Supremacy => (0.0, 0.04),
            Self::Superiority => (0.02, 0.08),
            Self::Parity => (0.04, 0.12),
            Self::Denial => (0.08, 0.18),
            Self::Incapability => (0.20, 0.36),
        }
    }

    pub(crate) fn stage1_enemy_loss_ratio(self) -> (f64, f64) {
        match self {
            Self::Supremacy => (0.20, 0.36),
            Self::Superiority => (0.08, 0.18),
            Self::Parity => (0.04, 0.12),
            Self::Denial => (0.02, 0.08),
            Self::Incapability => (0.0, 0.04),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePhase {
    OpeningTorpedo,
    DayShelling,
    ClosingTorpedo,
    NightShelling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetClass {
    SurfaceShip,
    Installation,
    PtBoat,
    Submarine,
}

impl TargetClass {
    pub(crate) const fn is_submarine(self) -> bool {
        matches!(self, Self::Submarine)
    }

    pub(crate) const fn is_surface_like(self) -> bool {
        !self.is_submarine()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttackCapability {
    CannotAttack,
    SurfaceOnly,
    BothPreferSubmarine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TorpedoAttackerSide {
    Friendly,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TorpedoHit {
    pub(crate) attacker_index: usize,
    pub(crate) defender_index: usize,
    pub(crate) damage: i64,
}

/// Parameters for a shelling side simulation.
pub(crate) struct ShellingParams {
    pub attacker_is_enemy: bool,
    pub formation_id: i64,
    pub engagement: EngagementType,
    pub phase: BattlePhase,
}

/// Mutable output buffers for an airstrike phase.
pub(crate) struct AirstrikeOutput<'a> {
    pub damage: &'a mut [i64],
    pub bak_targets: &'a mut [i64],
    pub rai_targets: &'a mut [i64],
    pub bak_flags: &'a mut [i64],
    pub rai_flags: &'a mut [i64],
}

/// Parameters for night battle shelling simulation.
///
/// Night battle does not use formation or engagement modifiers
/// per `KanColle` mechanics.
pub(crate) struct NightBattleParams<'a> {
    #[expect(dead_code)]
    pub friendly_formation_id: i64,
    #[expect(dead_code)]
    pub enemy_formation_id: i64,
    #[expect(dead_code)]
    pub engagement: EngagementType,
    pub air_state: Option<&'a AirState>,
}
