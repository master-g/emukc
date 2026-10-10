//! 対空カットイン: a ship whose equipment forms one shoots down more of the
//! aircraft that come to strike her fleet.
//!
//! The kinds, their figures and the order they are tried in are
//! `kancolle-replay`'s (`kcsim.js` `AACIDATA`, `getAACI`; `kcships.js`
//! `getAACItype`). Only the kinds formed by equipment and ship class are here;
//! the ones that name a ship are not.

use emukc_model::{
    codex::Codex,
    kc2::{KcShipType, start2::ApiMstSlotitem},
};

use crate::random::BattleRng;
use crate::targeting::{ship_mst, ship_type};
use crate::types::{BattleAirFire, BattleRuntimeShip};

/// One kind of cut-in.
struct Kind {
    /// `api_kind`.
    id: i64,
    /// Aircraft it shoots down from every slot, whatever else lands.
    fixed: i64,
    /// Times in a hundred it happens.
    rate: i64,
    /// What it multiplies the fleet's fixed shot by.
    modifier: f64,
    /// Its place in the order the kinds are preferred, lowest first.
    priority: usize,
    /// The equipment it shows, one letter a piece (see [`shown`]).
    equipment: &'static str,
}

const KINDS: [Kind; 11] = [
    Kind {
        id: 1,
        fixed: 7,
        rate: 65,
        modifier: 1.7,
        priority: 12,
        equipment: "HHR",
    },
    Kind {
        id: 2,
        fixed: 6,
        rate: 58,
        modifier: 1.7,
        priority: 17,
        equipment: "HR",
    },
    Kind {
        id: 3,
        fixed: 4,
        rate: 50,
        modifier: 1.6,
        priority: 32,
        equipment: "HH",
    },
    Kind {
        id: 4,
        fixed: 6,
        rate: 52,
        modifier: 1.5,
        priority: 16,
        equipment: "MSAR",
    },
    Kind {
        id: 5,
        fixed: 4,
        rate: 55,
        modifier: 1.5,
        priority: 33,
        equipment: "BBR",
    },
    Kind {
        id: 6,
        fixed: 4,
        rate: 40,
        modifier: 1.45,
        priority: 34,
        equipment: "MSA",
    },
    Kind {
        id: 7,
        fixed: 3,
        rate: 45,
        modifier: 1.35,
        priority: 42,
        equipment: "HAR",
    },
    Kind {
        id: 8,
        fixed: 4,
        rate: 50,
        modifier: 1.4,
        priority: 39,
        equipment: "BR",
    },
    Kind {
        id: 9,
        fixed: 2,
        rate: 40,
        modifier: 1.3,
        priority: 52,
        equipment: "HA",
    },
    Kind {
        id: 12,
        fixed: 3,
        rate: 45,
        modifier: 1.25,
        priority: 46,
        equipment: "CGR",
    },
    Kind {
        id: 13,
        fixed: 4,
        rate: 35,
        modifier: 1.35,
        priority: 40,
        equipment: "BCR",
    },
];

fn kind(id: i64) -> &'static Kind {
    KINDS.iter().find(|kind| kind.id == id).expect("only ids from KINDS are asked for")
}

/// 秋月型.
const AKIZUKI_CLASS: i64 = 54;
/// The icon every 高角砲 carries, main gun or secondary.
const HIGH_ANGLE_ICON: i64 = 16;

/// What a piece of equipment is to a cut-in.
#[derive(Default, Clone, Copy)]
struct Part {
    mst_id: i64,
    /// 高角砲, with or without a director of its own.
    high_angle: bool,
    /// 高角砲 with its own 高射装置: one whose 対空 is 8 or more.
    built_in_director: bool,
    /// 対空機銃.
    machine_gun: bool,
    /// 対空機銃 of 対空 9 or more (集中配備).
    concentrated: bool,
    /// 高射装置.
    director: bool,
    /// Any radar.
    radar: bool,
    /// A radar of 対空 2 or more.
    air_radar: bool,
    /// 三式弾.
    type3_shell: bool,
    /// 大口径主砲.
    large_gun: bool,
}

fn part(mst: &ApiMstSlotitem) -> Part {
    let type3 = mst.api_type[2];
    let high_angle = mst.api_type[3] == HIGH_ANGLE_ICON;
    let radar = matches!(type3, 12 | 13 | 93);
    Part {
        mst_id: mst.api_id,
        high_angle,
        built_in_director: high_angle && mst.api_tyku >= 8,
        machine_gun: type3 == 21,
        concentrated: type3 == 21 && mst.api_tyku >= 9,
        director: type3 == 36,
        radar,
        air_radar: radar && mst.api_tyku >= 2,
        type3_shell: type3 == 18,
        large_gun: matches!(type3, 3 | 38),
    }
}

fn parts(codex: &Codex, ship: &BattleRuntimeShip) -> Vec<Part> {
    ship.slot_items
        .iter()
        .filter_map(|item| codex.find::<ApiMstSlotitem>(&item.api_slotitem_id).ok())
        .map(part)
        .collect()
}

/// The kinds `ship` can fire, in the order the source lists them
/// (`getAACItype`): that order decides which is rolled first.
fn kinds_of(codex: &Codex, ship: &BattleRuntimeShip, parts: &[Part]) -> Vec<i64> {
    let count = |is: fn(&Part) -> bool| parts.iter().filter(|part| is(part)).count();
    let high_angle = count(|part| part.high_angle);
    let built_in = count(|part| part.built_in_director);
    let machine_guns = count(|part| part.machine_gun);
    let concentrated = count(|part| part.concentrated) > 0;
    let director = count(|part| part.director) > 0;
    let radar = count(|part| part.radar) > 0;
    let air_radar = count(|part| part.air_radar) > 0;
    let akizuki = ship_mst(codex, ship).is_some_and(|mst| mst.api_ctype == AKIZUKI_CLASS);
    let battleship =
        matches!(ship_type(codex, ship), Some(KcShipType::FBB | KcShipType::BB | KcShipType::BBV));

    let mut kinds = Vec::new();
    if akizuki {
        if high_angle >= 2 && radar {
            kinds.push(1);
        }
        if built_in >= 1 && radar {
            kinds.push(2);
        }
        if high_angle >= 2 {
            kinds.push(3);
        }
    }
    let big_guns = battleship
        && count(|part| part.large_gun) > 0
        && count(|part| part.type3_shell) > 0
        && director;
    if big_guns && air_radar {
        kinds.push(4);
    }
    if !akizuki && built_in >= 2 && air_radar {
        kinds.push(5);
    }
    if big_guns {
        kinds.push(6);
    }
    if !akizuki && built_in >= 1 && air_radar {
        kinds.push(8);
    }
    if !akizuki && high_angle >= 1 && director && air_radar {
        kinds.push(7);
    }
    if high_angle >= 1 && director {
        kinds.push(9);
    }
    if concentrated && machine_guns >= 2 && air_radar {
        kinds.push(12);
    }
    if concentrated && built_in >= 1 && air_radar {
        kinds.push(13);
    }
    kinds
}

/// The equipment a cut-in shows: for each letter of the kind, the first piece
/// not yet shown that answers to it.
fn shown(kind: &Kind, mut parts: Vec<Part>) -> Vec<i64> {
    let mut ids = Vec::new();
    for letter in kind.equipment.chars() {
        let found = parts.iter().position(|part| match letter {
            'B' => part.built_in_director,
            'H' => part.high_angle,
            'C' => part.concentrated,
            'G' => part.machine_gun,
            'R' => part.air_radar,
            'A' => part.director,
            'M' => part.large_gun,
            'S' => part.type3_shell,
            _ => false,
        });
        if let Some(at) = found {
            ids.push(parts.remove(at).mst_id);
        }
    }
    ids
}

/// A cut-in that fired.
pub(crate) struct AirFire {
    /// Aircraft it takes from every slot.
    pub fixed: i64,
    /// What it multiplies the fixed shot by.
    pub modifier: f64,
    /// What the client is told.
    pub packet: BattleAirFire,
}

/// Roll the fleet's cut-in: every ship afloat tries each of her kinds, and a
/// kind is only rolled when it would replace the one already standing
/// (`getAACI`). A fleet with nothing that forms a cut-in draws nothing.
pub(crate) fn roll_air_fire(
    codex: &Codex,
    rng: &mut impl BattleRng,
    defenders: &[BattleRuntimeShip],
) -> Option<AirFire> {
    let mut best: Option<(&Kind, usize, Vec<Part>)> = None;
    for (index, ship) in defenders.iter().enumerate().filter(|(_, ship)| ship.is_alive()) {
        let parts = parts(codex, ship);
        for id in kinds_of(codex, ship, &parts) {
            let kind = kind(id);
            let wins = best.as_ref().is_none_or(|(standing, ..)| kind.priority < standing.priority);
            if wins && rng.roll_range(0, 100) < kind.rate {
                best = Some((kind, index, parts.clone()));
            }
        }
    }
    best.map(|(kind, index, parts)| AirFire {
        fixed: kind.fixed,
        modifier: kind.modifier,
        packet: BattleAirFire {
            api_idx: index as i64,
            api_kind: kind.id,
            api_use_items: shown(kind, parts),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::SeededRng;
    use crate::test_utils::*;

    /// 10cm連装高角砲+高射装置: 対空 10.
    const HIGH_ANGLE_DIRECTOR: i64 = 122;
    /// 10cm連装高角砲: 対空 7.
    const HIGH_ANGLE: i64 = 3;
    /// 13号対空電探改.
    const AIR_RADAR: i64 = 106;
    /// 22号対水上電探: 対空 0.
    const SURFACE_RADAR: i64 = 28;
    /// 94式高射装置.
    const DIRECTOR: i64 = 121;
    /// 25mm三連装機銃 集中配備.
    const CONCENTRATED: i64 = 131;
    /// 25mm連装機銃.
    const MACHINE_GUN: i64 = 39;
    /// 46cm三連装砲 and 三式弾.
    const LARGE_GUN: i64 = 9;
    const TYPE3_SHELL: i64 = 35;
    /// 秋月 and 大和.
    const AKIZUKI: i64 = 421;
    const YAMATO: i64 = 131;

    /// A die that always shows the same face.
    struct Fixed(i64);

    impl BattleRng for Fixed {
        fn random_f64_range(&mut self, min: f64, _max: f64) -> f64 {
            min
        }

        fn roll_range_impl(&mut self, _min: i64, _max: i64) -> i64 {
            self.0
        }
    }

    fn ship(codex: &Codex, mst_id: i64, equipment: &[i64]) -> BattleRuntimeShip {
        let mut ship = sample_ship(codex, mst_id, 50);
        ship.slot_items = equipment.iter().copied().map(slotitem_with_mst_id).collect();
        BattleRuntimeShip::from(ship)
    }

    fn kinds(codex: &Codex, mst_id: i64, equipment: &[i64]) -> Vec<i64> {
        let ship = ship(codex, mst_id, equipment);
        kinds_of(codex, &ship, &parts(codex, &ship))
    }

    #[test]
    fn the_table_follows_the_source() {
        // (id, fixed, rate, modifier, priority) as `AACIDATA` and `orderKnown` give them.
        let source = [
            (1, 7, 65, 1.7, 12),
            (2, 6, 58, 1.7, 17),
            (3, 4, 50, 1.6, 32),
            (4, 6, 52, 1.5, 16),
            (5, 4, 55, 1.5, 33),
            (6, 4, 40, 1.45, 34),
            (7, 3, 45, 1.35, 42),
            (8, 4, 50, 1.4, 39),
            (9, 2, 40, 1.3, 52),
            (12, 3, 45, 1.25, 46),
            (13, 4, 35, 1.35, 40),
        ];
        let table: Vec<_> = KINDS
            .iter()
            .map(|kind| (kind.id, kind.fixed, kind.rate, kind.modifier, kind.priority))
            .collect();
        assert_eq!(table, source);
    }

    #[test]
    fn equipment_and_class_decide_the_kinds() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let destroyer = first_ship_mst_by_type(&codex, KcShipType::DD);
        let twin = [HIGH_ANGLE_DIRECTOR, HIGH_ANGLE_DIRECTOR, AIR_RADAR];
        assert_eq!(kinds(&codex, AKIZUKI, &twin), [1, 2, 3]);
        assert_eq!(kinds(&codex, destroyer, &twin), [5, 8]);
        // 秋月型 takes any radar; the others need one that sees aircraft.
        assert_eq!(kinds(&codex, AKIZUKI, &[HIGH_ANGLE, HIGH_ANGLE, SURFACE_RADAR]), [1, 3]);
        assert_eq!(kinds(&codex, destroyer, &[HIGH_ANGLE_DIRECTOR, SURFACE_RADAR]), [0; 0]);
        // A plain 高角砲 needs a 高射装置 beside it.
        assert_eq!(kinds(&codex, destroyer, &[HIGH_ANGLE, DIRECTOR, AIR_RADAR]), [7, 9]);
        assert_eq!(kinds(&codex, destroyer, &[HIGH_ANGLE, DIRECTOR]), [9]);
        assert_eq!(kinds(&codex, destroyer, &[CONCENTRATED, MACHINE_GUN, AIR_RADAR]), [12]);
        assert_eq!(
            kinds(&codex, destroyer, &[CONCENTRATED, HIGH_ANGLE_DIRECTOR, AIR_RADAR]),
            [8, 13]
        );
        // 大口径主砲, 三式弾 and 高射装置 on a battleship, with and without the radar.
        assert_eq!(kinds(&codex, YAMATO, &[LARGE_GUN, TYPE3_SHELL, DIRECTOR, AIR_RADAR]), [4, 6]);
        assert_eq!(kinds(&codex, YAMATO, &[LARGE_GUN, TYPE3_SHELL, DIRECTOR]), [6]);
        assert_eq!(kinds(&codex, destroyer, &[LARGE_GUN, TYPE3_SHELL, DIRECTOR]), [0; 0]);
        assert_eq!(kinds(&codex, destroyer, &[]), [0; 0]);
    }

    #[test]
    fn the_preferred_kind_stands_and_names_its_equipment() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let destroyer = first_ship_mst_by_type(&codex, KcShipType::DD);
        let twin = [HIGH_ANGLE_DIRECTOR, HIGH_ANGLE_DIRECTOR, AIR_RADAR];
        let fleet = [ship(&codex, destroyer, &twin), ship(&codex, AKIZUKI, &twin)];

        // Every roll succeeds: 秋月's kind 1 is preferred to the destroyer's 5.
        let fire = roll_air_fire(&codex, &mut Fixed(0), &fleet).unwrap();
        assert_eq!((fire.fixed, fire.modifier), (7, 1.7));
        assert_eq!(fire.packet.api_idx, 1);
        assert_eq!(fire.packet.api_kind, 1);
        assert_eq!(fire.packet.api_use_items, twin);

        // 57 fails kinds 5 (55) and 8 (50) and lands 秋月's 1 (65).
        assert_eq!(roll_air_fire(&codex, &mut Fixed(57), &fleet).unwrap().packet.api_kind, 1);
        // 60 lands only kind 1; 99 lands nothing.
        assert!(roll_air_fire(&codex, &mut Fixed(99), &fleet).is_none());
        // The destroyer alone, every roll landing: 5 stands and 8 is not preferred to it.
        let fire = roll_air_fire(&codex, &mut Fixed(0), &fleet[..1]).unwrap();
        assert_eq!((fire.packet.api_idx, fire.packet.api_kind), (0, 5));
    }

    #[test]
    fn a_fleet_without_the_equipment_draws_nothing() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let destroyer = first_ship_mst_by_type(&codex, KcShipType::DD);
        let fleet =
            [ship(&codex, destroyer, &[HIGH_ANGLE, MACHINE_GUN]), ship(&codex, YAMATO, &[])];
        let mut rng = SeededRng::new(3);
        let mut untouched = SeededRng::new(3);
        assert!(roll_air_fire(&codex, &mut rng, &fleet).is_none());
        assert_eq!(rng.roll_range(0, 1000), untouched.roll_range(0, 1000));
    }
}
