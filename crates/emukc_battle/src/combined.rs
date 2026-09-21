//! Combined fleet (連合艦隊) formation multipliers and attack-power corrections.
//!
//! Every number here comes from `docs/battle/combined-fleet-reference.md`, which
//! transcribes the community-verified tables on wikiwiki.jp. Accuracy and
//! evasion corrections are deliberately absent: upstream marks every one of
//! those cells unverified (`?`), so they are not modelled at all.

/// Which combined fleet the player organised, as stored in
/// `profile.combined_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombinedType {
    /// 空母機動部隊 — `combined_type == 1`.
    CarrierTaskForce,
    /// 水上打撃部隊 — `combined_type == 2`.
    SurfaceTaskForce,
    /// 輸送護衛部隊 — `combined_type == 3`.
    TransportEscort,
}

impl CombinedType {
    /// Parse from the stored `combined_type`. `0` (disbanded) and anything out
    /// of range yield `None`.
    pub const fn from_api_id(api_id: i64) -> Option<Self> {
        match api_id {
            1 => Some(Self::CarrierTaskForce),
            2 => Some(Self::SurfaceTaskForce),
            3 => Some(Self::TransportEscort),
            _ => None,
        }
    }

    /// The value stored in `profile.combined_type`.
    pub const fn api_id(self) -> i64 {
        match self {
            Self::CarrierTaskForce => 1,
            Self::SurfaceTaskForce => 2,
            Self::TransportEscort => 3,
        }
    }
}

/// Which of the two decks a ship belongs to.
///
/// The packet index space is continuous across both decks: deck 1 occupies
/// 0..=5 and deck 2 occupies 6..=11. The escort offset is always 6, even when
/// deck 1 holds fewer than six ships — see `_getNum` in the decoded client,
/// which dispatches on `index >= 6` and then reads `combined[index - 6]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombinedFleetRole {
    /// 第1艦隊 / 本隊 — packet indices 0..=5.
    Main,
    /// 第2艦隊 / 護衛艦隊 — packet indices 6..=11.
    Escort,
}

/// Fixed packet index offset of the escort deck.
pub const ESCORT_INDEX_OFFSET: usize = 6;

impl CombinedFleetRole {
    /// Offset added to this deck's local index to get the packet index.
    pub const fn index_offset(self) -> usize {
        match self {
            Self::Main => 0,
            Self::Escort => ESCORT_INDEX_OFFSET,
        }
    }
}

/// Translate a friendly ship's index in the simulation's contiguous vector into
/// the client's packet index space.
///
/// The simulation packs deck 2 directly behind deck 1, so deck 2 starts at
/// `escort_start` — deck 1's actual ship count. The client always reads deck 2
/// from index 6, so a deck 1 shorter than six ships leaves a gap the packet has
/// to carry.
pub(crate) const fn packet_index(index: usize, escort_start: usize) -> usize {
    if index < escort_start {
        index
    } else {
        ESCORT_INDEX_OFFSET + index - escort_start
    }
}

/// Attack class, for picking a formation multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombinedAttackClass {
    /// 砲撃戦.
    Shelling,
    /// 雷撃戦.
    Torpedo,
    /// 対潜攻撃.
    Asw,
    /// 対空 — also the aerial-phase correction row.
    AntiAir,
}

/// Formation multiplier for the four 警戒航行序列 (formation ids 11..=14).
///
/// Returns `None` for any other formation id — night-start cells use the six
/// normal formations instead, and those are not this table's business.
///
/// The three combined types share one table; only deck 1's lack of a torpedo
/// phase distinguishes them, and that is enforced by the phase order, not here.
pub fn combined_formation_modifier(formation_id: i64, class: CombinedAttackClass) -> Option<f64> {
    let row = match formation_id {
        // 第一警戒航行序列 (対潜警戒)
        11 => [0.8, 0.7, 1.3, 1.1],
        // 第二警戒航行序列 (前方警戒)
        12 => [1.0, 0.9, 1.1, 1.0],
        // 第三警戒航行序列 (輪形陣)
        13 => [0.7, 0.6, 1.0, 1.5],
        // 第四警戒航行序列 (戦闘隊形)
        14 => [1.1, 1.0, 0.7, 1.0],
        _ => return None,
    };

    Some(match class {
        CombinedAttackClass::Shelling => row[0],
        CombinedAttackClass::Torpedo => row[1],
        CombinedAttackClass::Asw => row[2],
        CombinedAttackClass::AntiAir => row[3],
    })
}

/// Minimum deck 2 size each combined formation requires.
///
/// Eligibility depends on deck 2 only — deck 1 may hold any number of ships.
/// Escort withdrawal can shrink deck 2 past a threshold mid-sortie.
pub const fn combined_formation_min_escort_size(formation_id: i64) -> Option<usize> {
    match formation_id {
        11 | 12 => Some(0),
        13 => Some(5),
        14 => Some(4),
        _ => None,
    }
}

/// Additive combined-fleet correction for 味方連合艦隊 vs 敵通常艦隊.
///
/// The term sits inside basic attack power next to the improvement bonus:
/// `... + improvement + correction + 5`.
///
/// `attacker_is_friendly` picks the 自軍 / 敵軍 row; `role` is the deck the
/// *friendly* combined fleet is acting as during this phase (the enemy is a
/// single fleet, so it has no deck of its own — its correction still varies by
/// which friendly deck it is trading fire with).
///
/// Daytime ASW and night battle take no correction at all, so they have no
/// class here; callers must not route them through this function.
pub const fn combined_correction_vs_single(
    ty: CombinedType,
    class: CombinedAttackClass,
    role: CombinedFleetRole,
    attacker_is_friendly: bool,
) -> i64 {
    match class {
        // Torpedo is -5 for both sides in every case.
        CombinedAttackClass::Torpedo => -5,
        // Aerial is 0 for both sides in every case.
        CombinedAttackClass::AntiAir | CombinedAttackClass::Asw => 0,
        CombinedAttackClass::Shelling => {
            // [deck 1 自軍, deck 1 敵軍, deck 2 自軍, deck 2 敵軍]
            let row = match ty {
                CombinedType::CarrierTaskForce => [2, 10, 10, 5],
                CombinedType::SurfaceTaskForce => [10, 5, -5, -5],
                CombinedType::TransportEscort => [-5, 10, 10, 5],
            };
            let column = match (role, attacker_is_friendly) {
                (CombinedFleetRole::Main, true) => 0,
                (CombinedFleetRole::Main, false) => 1,
                (CombinedFleetRole::Escort, true) => 2,
                (CombinedFleetRole::Escort, false) => 3,
            };
            row[column]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combined_type_round_trips_through_api_id() {
        for ty in [
            CombinedType::CarrierTaskForce,
            CombinedType::SurfaceTaskForce,
            CombinedType::TransportEscort,
        ] {
            assert_eq!(CombinedType::from_api_id(ty.api_id()), Some(ty));
        }
        assert_eq!(CombinedType::from_api_id(0), None, "0 means disbanded");
        assert_eq!(CombinedType::from_api_id(4), None);
        assert_eq!(CombinedType::from_api_id(-1), None);
    }

    /// The escort deck always starts at packet index 6, however few ships deck
    /// 1 holds. Getting this wrong silently misattributes every hit.
    #[test]
    fn escort_offset_is_fixed_at_six() {
        assert_eq!(CombinedFleetRole::Main.index_offset(), 0);
        assert_eq!(CombinedFleetRole::Escort.index_offset(), 6);
    }

    /// A short deck 1 does not pull deck 2 down with it: the gap between deck
    /// 1's last ship and index 6 stays empty on the wire.
    #[test]
    fn packet_index_leaves_a_gap_when_deck_one_is_short() {
        // Full deck 1: the two spaces coincide.
        for i in 0..12 {
            assert_eq!(packet_index(i, 6), i);
        }
        // Deck 1 of four: deck 2 still starts at 6.
        assert_eq!(packet_index(0, 4), 0);
        assert_eq!(packet_index(3, 4), 3);
        assert_eq!(packet_index(4, 4), 6, "deck 2 flagship jumps the gap");
        assert_eq!(packet_index(9, 4), 11);
    }

    /// Every cell of the formation table, transcribed from the reference.
    #[test]
    fn formation_modifier_table_matches_reference() {
        use CombinedAttackClass::{AntiAir, Asw, Shelling, Torpedo};

        let expected = [
            // (formation, shelling, torpedo, asw, anti-air)
            (11, 0.8, 0.7, 1.3, 1.1),
            (12, 1.0, 0.9, 1.1, 1.0),
            (13, 0.7, 0.6, 1.0, 1.5),
            (14, 1.1, 1.0, 0.7, 1.0),
        ];

        for (id, shelling, torpedo, asw, anti_air) in expected {
            assert_eq!(combined_formation_modifier(id, Shelling), Some(shelling), "shelling {id}");
            assert_eq!(combined_formation_modifier(id, Torpedo), Some(torpedo), "torpedo {id}");
            assert_eq!(combined_formation_modifier(id, Asw), Some(asw), "asw {id}");
            assert_eq!(combined_formation_modifier(id, AntiAir), Some(anti_air), "anti-air {id}");
        }
    }

    /// Normal formations (1..=6) must not resolve here — combined battles use
    /// 11..=14, and a night-start cell falls back to the normal table.
    #[test]
    fn formation_modifier_rejects_non_combined_formations() {
        for id in [0, 1, 2, 3, 4, 5, 6, 10, 15] {
            assert_eq!(
                combined_formation_modifier(id, CombinedAttackClass::Shelling),
                None,
                "formation {id} is not a 警戒航行序列"
            );
        }
    }

    #[test]
    fn formation_eligibility_depends_on_escort_size() {
        assert_eq!(combined_formation_min_escort_size(11), Some(0));
        assert_eq!(combined_formation_min_escort_size(12), Some(0));
        assert_eq!(combined_formation_min_escort_size(13), Some(5));
        assert_eq!(combined_formation_min_escort_size(14), Some(4));
        assert_eq!(combined_formation_min_escort_size(1), None);
    }

    /// Every shelling cell of the 味方連合・敵通常 table.
    #[test]
    fn shelling_correction_table_matches_reference() {
        use CombinedAttackClass::Shelling;
        use CombinedFleetRole::{Escort, Main};
        use CombinedType::{CarrierTaskForce, SurfaceTaskForce, TransportEscort};

        let expected = [
            // (type, role, friendly?, correction)
            (CarrierTaskForce, Main, true, 2),
            (CarrierTaskForce, Main, false, 10),
            (CarrierTaskForce, Escort, true, 10),
            (CarrierTaskForce, Escort, false, 5),
            (SurfaceTaskForce, Main, true, 10),
            (SurfaceTaskForce, Main, false, 5),
            (SurfaceTaskForce, Escort, true, -5),
            (SurfaceTaskForce, Escort, false, -5),
            (TransportEscort, Main, true, -5),
            (TransportEscort, Main, false, 10),
            (TransportEscort, Escort, true, 10),
            (TransportEscort, Escort, false, 5),
        ];

        for (ty, role, friendly, want) in expected {
            assert_eq!(
                combined_correction_vs_single(ty, Shelling, role, friendly),
                want,
                "{ty:?} {role:?} friendly={friendly}"
            );
        }
    }

    /// Torpedo is a flat -5 and aerial a flat 0 — neither varies by type, deck
    /// or side.
    #[test]
    fn torpedo_and_aerial_corrections_are_flat() {
        use CombinedFleetRole::{Escort, Main};
        use CombinedType::{CarrierTaskForce, SurfaceTaskForce, TransportEscort};

        for ty in [CarrierTaskForce, SurfaceTaskForce, TransportEscort] {
            for role in [Main, Escort] {
                for friendly in [true, false] {
                    assert_eq!(
                        combined_correction_vs_single(
                            ty,
                            CombinedAttackClass::Torpedo,
                            role,
                            friendly
                        ),
                        -5,
                        "torpedo is always -5"
                    );
                    assert_eq!(
                        combined_correction_vs_single(
                            ty,
                            CombinedAttackClass::AntiAir,
                            role,
                            friendly
                        ),
                        0,
                        "aerial is always 0"
                    );
                }
            }
        }
    }
}
