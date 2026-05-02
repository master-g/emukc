use crate::types::BattleType;

/// Identifies a single phase within a battle flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePhaseKind {
    Kouku,
    OpeningAsw,
    OpeningTorpedo,
    Shelling1,
    Shelling2,
    ClosingTorpedo,
}

/// Defines the ordered sequence of phases for a battle type.
pub(crate) struct BattleFlow {
    pub phases: &'static [BattlePhaseKind],
}

impl BattleFlow {
    /// Select the flow for a given battle type.
    pub fn for_battle_type(battle_type: BattleType) -> &'static BattleFlow {
        match battle_type {
            BattleType::Normal => &SURFACE_DAY,
            BattleType::AirBattle => &AIR_BATTLE,
            BattleType::LdAirBattle => &LD_AIR_BATTLE,
            BattleType::LdShooting => &LD_SHOOTING,
        }
    }
}

/// Normal surface battle: full phase sequence.
pub(crate) static SURFACE_DAY: BattleFlow = BattleFlow {
    phases: &[
        BattlePhaseKind::Kouku,
        BattlePhaseKind::OpeningAsw,
        BattlePhaseKind::OpeningTorpedo,
        BattlePhaseKind::Shelling1,
        BattlePhaseKind::Shelling2,
        BattlePhaseKind::ClosingTorpedo,
    ],
};

/// Air battle: kouku + OASW only.
pub(crate) static AIR_BATTLE: BattleFlow = BattleFlow {
    phases: &[BattlePhaseKind::Kouku, BattlePhaseKind::OpeningAsw],
};

/// Land-based air battle: kouku only.
pub(crate) static LD_AIR_BATTLE: BattleFlow = BattleFlow {
    phases: &[BattlePhaseKind::Kouku],
};

/// Land-based shooting: shelling only (no torpedo, no kouku).
pub(crate) static LD_SHOOTING: BattleFlow = BattleFlow {
    phases: &[BattlePhaseKind::Shelling1, BattlePhaseKind::Shelling2],
};
