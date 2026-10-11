//! 対空カットイン: a ship whose equipment forms one shoots down more of the
//! aircraft that come to strike her fleet.
//!
//! The kinds, their figures and the order they are tried in are
//! `kancolle-replay`'s (`kcsim.js` `AACIDATA`, `getAACI`; `kcships.js`
//! `getAACItype`), the kinds that name a ship among them.

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

const KINDS: [Kind; 53] = [
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
        id: 10,
        fixed: 8,
        rate: 60,
        modifier: 1.65,
        priority: 6,
        equipment: "HCR",
    },
    Kind {
        id: 11,
        fixed: 6,
        rate: 55,
        modifier: 1.5,
        priority: 9,
        equipment: "HC",
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
    Kind {
        id: 14,
        fixed: 4,
        rate: 63,
        modifier: 1.45,
        priority: 31,
        equipment: "HGR",
    },
    Kind {
        id: 15,
        fixed: 3,
        rate: 55,
        modifier: 1.3,
        priority: 41,
        equipment: "HG",
    },
    Kind {
        id: 16,
        fixed: 4,
        rate: 62,
        modifier: 1.4,
        priority: 30,
        equipment: "HGR",
    },
    Kind {
        id: 17,
        fixed: 2,
        rate: 55,
        modifier: 1.25,
        priority: 49,
        equipment: "HG",
    },
    Kind {
        id: 18,
        fixed: 2,
        rate: 60,
        modifier: 1.2,
        priority: 50,
        equipment: "C",
    },
    Kind {
        id: 19,
        fixed: 5,
        rate: 55,
        modifier: 1.45,
        priority: 26,
        equipment: "HC",
    },
    Kind {
        id: 20,
        fixed: 3,
        rate: 65,
        modifier: 1.25,
        priority: 43,
        equipment: "C",
    },
    Kind {
        id: 21,
        fixed: 5,
        rate: 60,
        modifier: 1.45,
        priority: 27,
        equipment: "HR",
    },
    Kind {
        id: 22,
        fixed: 2,
        rate: 60,
        modifier: 1.2,
        priority: 51,
        equipment: "C",
    },
    Kind {
        id: 23,
        fixed: 1,
        rate: 80,
        modifier: 1.05,
        priority: 53,
        equipment: "G",
    },
    Kind {
        id: 24,
        fixed: 3,
        rate: 55,
        modifier: 1.25,
        priority: 44,
        equipment: "HG",
    },
    Kind {
        id: 25,
        fixed: 7,
        rate: 60,
        modifier: 1.55,
        priority: 10,
        equipment: "GRS",
    },
    Kind {
        id: 26,
        fixed: 6,
        rate: 60,
        modifier: 1.4,
        priority: 15,
        equipment: "HR",
    },
    Kind {
        id: 27,
        fixed: 6,
        rate: 55,
        modifier: 1.55,
        priority: 20,
        equipment: "BGR",
    },
    Kind {
        id: 28,
        fixed: 4,
        rate: 55,
        modifier: 1.4,
        priority: 35,
        equipment: "GR",
    },
    Kind {
        id: 29,
        fixed: 5,
        rate: 60,
        modifier: 1.55,
        priority: 28,
        equipment: "HR",
    },
    Kind {
        id: 30,
        fixed: 3,
        rate: 45,
        modifier: 1.3,
        priority: 38,
        equipment: "HHH",
    },
    Kind {
        id: 31,
        fixed: 2,
        rate: 50,
        modifier: 1.25,
        priority: 47,
        equipment: "HH",
    },
    Kind {
        id: 32,
        fixed: 3,
        rate: 50,
        modifier: 1.2,
        priority: 45,
        equipment: "CM",
    },
    Kind {
        id: 33,
        fixed: 3,
        rate: 42,
        modifier: 1.35,
        priority: 37,
        equipment: "HG",
    },
    Kind {
        id: 34,
        fixed: 7,
        rate: 60,
        modifier: 1.6,
        priority: 13,
        equipment: "BB",
    },
    Kind {
        id: 35,
        fixed: 6,
        rate: 55,
        modifier: 1.55,
        priority: 18,
        equipment: "BH",
    },
    Kind {
        id: 36,
        fixed: 6,
        rate: 55,
        modifier: 1.55,
        priority: 19,
        equipment: "HHR",
    },
    Kind {
        id: 37,
        fixed: 4,
        rate: 40,
        modifier: 1.45,
        priority: 36,
        equipment: "HH",
    },
    Kind {
        id: 38,
        fixed: 10,
        rate: 62,
        modifier: 1.85,
        priority: 1,
        equipment: "BB",
    },
    Kind {
        id: 39,
        fixed: 10,
        rate: 57,
        modifier: 1.7,
        priority: 2,
        equipment: "BB",
    },
    Kind {
        id: 40,
        fixed: 10,
        rate: 56,
        modifier: 1.7,
        priority: 3,
        equipment: "BBR",
    },
    Kind {
        id: 41,
        fixed: 9,
        rate: 55,
        modifier: 1.65,
        priority: 5,
        equipment: "BB",
    },
    Kind {
        id: 42,
        fixed: 10,
        rate: 65,
        modifier: 1.65,
        priority: 4,
        equipment: "HHRG",
    },
    Kind {
        id: 43,
        fixed: 8,
        rate: 58,
        modifier: 1.6,
        priority: 7,
        equipment: "HHR",
    },
    Kind {
        id: 44,
        fixed: 6,
        rate: 55,
        modifier: 1.6,
        priority: 14,
        equipment: "HRG",
    },
    Kind {
        id: 45,
        fixed: 6,
        rate: 50,
        modifier: 1.55,
        priority: 21,
        equipment: "HR",
    },
    Kind {
        id: 46,
        fixed: 8,
        rate: 60,
        modifier: 1.55,
        priority: 8,
        equipment: "MCR",
    },
    Kind {
        id: 47,
        fixed: 2,
        rate: 70,
        modifier: 1.3,
        priority: 48,
        equipment: "MG",
    },
    Kind {
        id: 48,
        fixed: 8,
        rate: 65,
        modifier: 1.75,
        priority: 11,
        equipment: "HHR",
    },
    Kind {
        id: 49,
        fixed: 6,
        rate: 50,
        modifier: 1.5,
        priority: 23,
        equipment: "HHR",
    },
    Kind {
        id: 50,
        fixed: 7,
        rate: 50,
        modifier: 1.5,
        priority: 22,
        equipment: "HHAR",
    },
    Kind {
        id: 51,
        fixed: 5,
        rate: 50,
        modifier: 1.4,
        priority: 24,
        equipment: "HCR",
    },
    Kind {
        id: 52,
        fixed: 5,
        rate: 50,
        modifier: 1.4,
        priority: 25,
        equipment: "HHA",
    },
    Kind {
        id: 53,
        fixed: 4,
        rate: 60,
        modifier: 1.6,
        priority: 29,
        equipment: "BR",
    },
];

fn kind(id: i64) -> &'static Kind {
    KINDS.iter().find(|kind| kind.id == id).expect("only ids from KINDS are asked for")
}

/// 秋月型.
const AKIZUKI_CLASS: i64 = 54;
/// Fletcher級.
const FLETCHER_CLASS: i64 = 91;
/// Atlanta級.
const ATLANTA_CLASS: i64 = 99;
/// 白露型.
const SHIRATSUYU_CLASS: i64 = 23;
/// Queen Elizabeth, Ark Royal, J, Nelson, Town and Illustrious classes.
const ROYAL_NAVY_CLASSES: [i64; 6] = [67, 78, 82, 88, 108, 112];
/// The icon every 高角砲 carries, main gun or secondary.
const HIGH_ANGLE_ICON: i64 = 16;

/// What a piece of equipment is to a cut-in.
#[derive(Default, Clone, Copy)]
struct Part {
    mst_id: i64,
    /// 対空.
    tyku: i64,
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
        tyku: mst.api_tyku,
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
/// (`getAACItype`): that order decides which is rolled first. The ships and
/// equipment it names are named here by the same ids.
fn kinds_of(codex: &Codex, ship: &BattleRuntimeShip, parts: &[Part]) -> Vec<i64> {
    let count = |is: &dyn Fn(&Part) -> bool| parts.iter().filter(|part| is(part)).count();
    let of = |ids: &[i64]| count(&|part| ids.contains(&part.mst_id));
    let high_angle = count(&|part| part.high_angle);
    let built_in = count(&|part| part.built_in_director);
    let machine_guns = count(&|part| part.machine_gun);
    let concentrated = count(&|part| part.concentrated);
    let plain_machine_gun = machine_guns > concentrated;
    let director = count(&|part| part.director) > 0;
    let radar = count(&|part| part.radar) > 0;
    let air_radar = count(&|part| part.air_radar) > 0;
    // A radar of 対空 4 or more.
    let strong_radar = count(&|part| part.radar && part.tyku >= 4) > 0;
    let type3_shell = count(&|part| part.type3_shell) > 0;
    let mst = ship_mst(codex, ship);
    let id = mst.map_or(0, |mst| mst.api_id);
    let class = mst.map_or(0, |mst| mst.api_ctype);
    let is = |ids: &[i64]| ids.contains(&id);
    let akizuki = class == AKIZUKI_CLASS;
    let battleship =
        matches!(ship_type(codex, ship), Some(KcShipType::FBB | KcShipType::BB | KcShipType::BBV));
    // 5inch単装砲 Mk.30 and its 改, the same with GFCS Mk.37, and the director alone.
    let mk30 = of(&[313, 284]);
    let mk30_gfcs = of(&[308]);
    let gfcs = of(&[307]) > 0;

    let mut kinds = Vec::new();
    // 秋月型改 and 改二, 吹雪改三護: two 10cm連装高角砲改+高射装置改.
    if is(&[330, 346, 357, 537, 538, 963, 968, 1040]) && of(&[533]) >= 2 && strong_radar {
        kinds.push(48);
    }
    // 涼波改二 and her like, 北上改三 and the three 吹雪.
    if is(&[745, 981, 982, 983, 986, 987, 1033, 1034, 1071, 426, 1035, 1040])
        && built_in >= 2
        && strong_radar
    {
        kinds.push(49);
    }
    if akizuki || is(&[745, 981, 982, 983, 986, 987, 1033, 1034, 426, 1035, 1040]) {
        // 10cm連装高角砲改, with or without its director, and 94式高射装置.
        let guns = of(&[533, 553]);
        let type94 = of(&[121]) > 0;
        if (!akizuki || is(&[330, 346, 357, 537, 538, 963, 968]))
            && guns >= 2
            && strong_radar
            && type94
        {
            kinds.push(50);
        }
        if !akizuki
            && guns >= 1
            && strong_radar
            && count(&|part| part.machine_gun && part.tyku >= 5) > 0
        {
            kinds.push(51);
        }
        if of(&[553]) >= 2 && type94 {
            kinds.push(52);
        }
    }
    // 飛龍改三, 北上改三.
    if is(&[1031, 1071])
        && count(&|part| part.built_in_director && part.tyku >= 9) > 0
        && strong_radar
    {
        kinds.push(53);
    }
    // 吹雪改三.
    if id == 1035 {
        if high_angle >= 1 && machine_guns >= 1 {
            kinds.push(15);
        }
        if high_angle >= 1 && machine_guns >= 1 && air_radar {
            kinds.push(16);
        }
        if high_angle >= 1 && air_radar {
            kinds.push(21);
        }
        if high_angle >= 1 && plain_machine_gun {
            kinds.push(24);
        }
    }
    // 吹雪改三護.
    if id == 1040 {
        if built_in >= 1 && radar {
            kinds.push(2);
        }
        if high_angle >= 1 && air_radar {
            kinds.push(21);
        }
        if mk30_gfcs >= 2 {
            kinds.push(34);
        }
        if mk30 >= 1 && mk30_gfcs >= 1 {
            kinds.push(35);
        }
        if mk30 >= 2 && gfcs {
            kinds.push(36);
        }
    }
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
    // 摩耶改二, 飛龍改三, 北上改三.
    if is(&[428, 1031, 1071]) && concentrated >= 1 && high_angle >= 1 {
        if air_radar {
            kinds.push(10);
        }
        if is(&[428, 1031]) {
            kinds.push(11);
        }
    }
    // 五十鈴改二, 北上改三.
    if is(&[141, 1071]) && high_angle >= 1 && machine_guns >= 1 {
        if air_radar {
            kinds.push(14);
        }
        if id == 141 {
            kinds.push(15);
        }
    }
    // 霞改二乙, 夕張改二, 稲木改二.
    if is(&[470, 622]) && high_angle >= 1 && machine_guns >= 1 && air_radar {
        kinds.push(16);
    }
    if is(&[470, 979]) && high_angle >= 1 && machine_guns >= 1 {
        kinds.push(17);
    }
    // 鬼怒改二, with a 高角砲 that has no director of its own.
    if id == 487 && concentrated >= 1 && high_angle > built_in {
        kinds.push(19);
    }
    // 由良改二, 北上改三.
    if is(&[488, 1071]) && high_angle >= 1 && air_radar {
        kinds.push(21);
    }
    // 伊勢型改 and 改二, with 12cm30連装噴進砲改二.
    if is(&[82, 88, 553, 554]) && of(&[274]) >= 1 && air_radar && type3_shell {
        kinds.push(25);
    }
    if class == FLETCHER_CLASS {
        if mk30_gfcs >= 2 {
            kinds.push(34);
        }
        if mk30 >= 1 && mk30_gfcs >= 1 {
            kinds.push(35);
        }
        if of(&[313]) >= 2 {
            kinds.push(37);
        }
        if mk30 >= 2 && gfcs {
            kinds.push(36);
        }
    }
    if class == ATLANTA_CLASS {
        // 5inch連装両用砲(集中配備), with and without GFCS Mk.37.
        let with_gfcs = of(&[363]);
        let dual = of(&[363, 362]);
        if with_gfcs >= 2 {
            kinds.push(38);
        }
        if with_gfcs >= 1 && of(&[362]) >= 1 {
            kinds.push(39);
        }
        if gfcs && dual >= 2 {
            kinds.push(40);
        }
        if dual >= 2 {
            kinds.push(41);
        }
    }
    // 大和型改二: 10cm連装高角砲群 集中配備, 15m二重測距儀 and a 機銃 of 対空 6 or more.
    if is(&[546, 911, 916]) {
        let groups = of(&[464]);
        let rangefinder = of(&[142, 460]) > 0;
        let strong_gun = count(&|part| part.machine_gun && part.tyku >= 6) > 0;
        if groups >= 2 && rangefinder && strong_gun {
            kinds.push(42);
        }
        if groups >= 2 && rangefinder {
            kinds.push(43);
        }
        if groups >= 1 && rangefinder && strong_gun {
            kinds.push(44);
        }
        if groups >= 1 && rangefinder {
            kinds.push(45);
        }
    }
    // 榛名改二乙, with 35.6cm連装砲改三(ダズル迷彩仕様) or 改四.
    if id == 593 && of(&[502, 503]) >= 1 && concentrated >= 1 && air_radar {
        kinds.push(46);
    }
    // 白露型 of 対空 70 or more at their best, with 12.7cm連装砲C型改三H.
    if class == SHIRATSUYU_CLASS
        && mst.and_then(|mst| mst.api_tyku).is_some_and(|tyku| tyku[1] >= 70)
    {
        let guns = of(&[529]);
        if guns >= 2 || (guns >= 1 && (of(&[505]) >= 1 || strong_radar)) {
            kinds.push(47);
        }
    }
    let big_guns = battleship && count(&|part| part.large_gun) > 0 && type3_shell && director;
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
    // 大和型改二, 大和改 and 武蔵改, with 10cm連装高角砲改+増設機銃.
    if is(&[546, 911, 916, 136, 148]) && of(&[275]) >= 1 && air_radar {
        kinds.push(26);
    }
    // 大淀改, 飛龍改三.
    if is(&[321, 1031]) && of(&[275, 71, 220]) >= 1 && of(&[274]) >= 1 && air_radar {
        kinds.push(27);
    }
    // 伊勢型改 and 改二, 武蔵改 and 改二.
    if is(&[82, 88, 553, 554, 148, 546]) && of(&[274]) >= 1 && air_radar {
        kinds.push(28);
    }
    // 磯風乙改, 浜風乙改.
    if is(&[557, 558]) && high_angle >= 1 && air_radar {
        kinds.push(29);
    }
    if high_angle >= 1 && director {
        kinds.push(9);
    }
    // Gotland改 and andra.
    if is(&[579, 630]) && high_angle >= 1 && machine_guns >= 1 {
        kinds.push(33);
    }
    if concentrated >= 1 && machine_guns >= 2 && air_radar {
        kinds.push(12);
    }
    // 摩耶改二 and 飛龍改三 have kinds 10 and 11 instead.
    if !is(&[428, 1031]) && concentrated >= 1 && built_in >= 1 && air_radar {
        kinds.push(13);
    }
    // 皐月改二, 稲木改二; 鬼怒改二; 文月改二.
    if is(&[418, 979]) && concentrated >= 1 {
        kinds.push(18);
    }
    if id == 487 && concentrated >= 1 {
        kinds.push(20);
    }
    if id == 548 && concentrated >= 1 {
        kinds.push(22);
    }
    // UIT-25, 伊504.
    if is(&[539, 530]) && plain_machine_gun {
        kinds.push(23);
    }
    // 天龍改二, Gotland改 and andra, 稲木改二, 龍田改二.
    if is(&[477, 579, 630]) && high_angle >= 3 {
        kinds.push(30);
    }
    if is(&[477, 979]) && high_angle >= 2 {
        kinds.push(31);
    }
    if is(&[478, 477]) && high_angle >= 1 && plain_machine_gun {
        kinds.push(24);
    }
    // The Royal Navy and 金剛型改二: ポンポン砲, 16inch Mk.I三連装砲改+FCR type284, UP Rocket Launchers.
    if (ROYAL_NAVY_CLASSES.contains(&class) || is(&[149, 150, 151, 152, 591, 592, 593, 954, 694]))
        && ((of(&[191]) >= 1 && of(&[300]) >= 1)
            || (of(&[301]) >= 1 && of(&[191]) >= 1)
            || of(&[301]) >= 2)
    {
        kinds.push(32);
    }
    kinds
}

/// The equipment a cut-in shows: for each letter of the kind, the first piece
/// not yet shown that answers to it.
///
/// A few kinds show equipment the source names outright, whatever of it the
/// ship carries.
fn shown(kind: &Kind, mut parts: Vec<Part>) -> Vec<i64> {
    let has = |id: i64| parts.iter().any(|part| part.mst_id == id);
    match kind.id {
        34 => return vec![308, 308],
        35 => return vec![308, 313],
        36 => return vec![313, 313, 307],
        37 => return vec![313, 313],
        39 => return vec![363, 362],
        40 => return vec![362, 362, 307],
        41 => return vec![362, 362],
        32 => {
            return [[191, 300], [301, 191], [301, 301]]
                .into_iter()
                .find(|pair| has(pair[0]) && has(pair[1]))
                .map_or_else(Vec::new, Vec::from);
        }
        _ => {}
    }
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

    // The ships the source names.
    const AKIZUKI_KAI: i64 = 330;
    const FUBUKI_K2: i64 = 426;
    const MAYA_K2: i64 = 428;
    const HIRYUU_K3: i64 = 1031;
    const ISUZU_K2: i64 = 141;
    const KINU_K2: i64 = 487;
    const UIT_25: i64 = 539;
    const SHIRATSUYU_KAI: i64 = 242;
    const SHIRATSUYU_K2: i64 = 497;
    const FLETCHER: i64 = 596;
    const ATLANTA: i64 = 597;
    const YAMATO_KAI: i64 = 136;
    const MUSASHI_KAI: i64 = 148;
    const YAMATO_K2: i64 = 911;
    const KONGOU_K2: i64 = 149;

    /// 12.7cm連装高角砲 (対空 4) and 12.7cm高角砲+高射装置 (対空 8), secondary guns.
    const SECONDARY: i64 = 10;
    const SECONDARY_DIRECTOR: i64 = 130;
    /// 10cm連装高角砲改 (対空 7), and the same with 高射装置改 (対空 11).
    const HIGH_ANGLE_K: i64 = 553;
    const HIGH_ANGLE_K_DIRECTOR: i64 = 533;
    /// 10cm連装高角砲改+増設機銃.
    const HIGH_ANGLE_MACHINE_GUN: i64 = 275;
    /// 12.7cm連装砲C型改三H.
    const C_TYPE_K3H: i64 = 529;
    /// 5inch単装砲 Mk.30改, the same with GFCS Mk.37, and GFCS Mk.37 alone.
    const MK30_KAI: i64 = 313;
    const MK30_GFCS: i64 = 308;
    const GFCS: i64 = 307;
    /// 5inch連装両用砲(集中配備), without and with GFCS Mk.37.
    const DUAL: i64 = 362;
    const DUAL_GFCS: i64 = 363;
    /// 10cm連装高角砲群 集中配備 and 15m二重測距儀+21号電探改二.
    const GUN_GROUP: i64 = 464;
    const RANGEFINDER: i64 = 142;
    /// QF 2ポンド8連装ポンポン砲 and 16inch Mk.I三連装砲改+FCR type284.
    const POM_POM: i64 = 191;
    const MK1_FCR: i64 = 300;

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
        // (id, fixed, rate, modifier, priority, equipment) as `AACIDATA` and `orderKnown` give them.
        let source = [
            (1, 7, 65, 1.7, 12, "HHR"),
            (2, 6, 58, 1.7, 17, "HR"),
            (3, 4, 50, 1.6, 32, "HH"),
            (4, 6, 52, 1.5, 16, "MSAR"),
            (5, 4, 55, 1.5, 33, "BBR"),
            (6, 4, 40, 1.45, 34, "MSA"),
            (7, 3, 45, 1.35, 42, "HAR"),
            (8, 4, 50, 1.4, 39, "BR"),
            (9, 2, 40, 1.3, 52, "HA"),
            (10, 8, 60, 1.65, 6, "HCR"),
            (11, 6, 55, 1.5, 9, "HC"),
            (12, 3, 45, 1.25, 46, "CGR"),
            (13, 4, 35, 1.35, 40, "BCR"),
            (14, 4, 63, 1.45, 31, "HGR"),
            (15, 3, 55, 1.3, 41, "HG"),
            (16, 4, 62, 1.4, 30, "HGR"),
            (17, 2, 55, 1.25, 49, "HG"),
            (18, 2, 60, 1.2, 50, "C"),
            (19, 5, 55, 1.45, 26, "HC"),
            (20, 3, 65, 1.25, 43, "C"),
            (21, 5, 60, 1.45, 27, "HR"),
            (22, 2, 60, 1.2, 51, "C"),
            (23, 1, 80, 1.05, 53, "G"),
            (24, 3, 55, 1.25, 44, "HG"),
            (25, 7, 60, 1.55, 10, "GRS"),
            (26, 6, 60, 1.4, 15, "HR"),
            (27, 6, 55, 1.55, 20, "BGR"),
            (28, 4, 55, 1.4, 35, "GR"),
            (29, 5, 60, 1.55, 28, "HR"),
            (30, 3, 45, 1.3, 38, "HHH"),
            (31, 2, 50, 1.25, 47, "HH"),
            (32, 3, 50, 1.2, 45, "CM"),
            (33, 3, 42, 1.35, 37, "HG"),
            (34, 7, 60, 1.6, 13, "BB"),
            (35, 6, 55, 1.55, 18, "BH"),
            (36, 6, 55, 1.55, 19, "HHR"),
            (37, 4, 40, 1.45, 36, "HH"),
            (38, 10, 62, 1.85, 1, "BB"),
            (39, 10, 57, 1.7, 2, "BB"),
            (40, 10, 56, 1.7, 3, "BBR"),
            (41, 9, 55, 1.65, 5, "BB"),
            (42, 10, 65, 1.65, 4, "HHRG"),
            (43, 8, 58, 1.6, 7, "HHR"),
            (44, 6, 55, 1.6, 14, "HRG"),
            (45, 6, 50, 1.55, 21, "HR"),
            (46, 8, 60, 1.55, 8, "MCR"),
            (47, 2, 70, 1.3, 48, "MG"),
            (48, 8, 65, 1.75, 11, "HHR"),
            (49, 6, 50, 1.5, 23, "HHR"),
            (50, 7, 50, 1.5, 22, "HHAR"),
            (51, 5, 50, 1.4, 24, "HCR"),
            (52, 5, 50, 1.4, 25, "HHA"),
            (53, 4, 60, 1.6, 29, "BR"),
        ];
        let table: Vec<_> = KINDS
            .iter()
            .map(|kind| {
                (kind.id, kind.fixed, kind.rate, kind.modifier, kind.priority, kind.equipment)
            })
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
    fn a_named_ship_has_kinds_of_her_own() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // 摩耶改二: her own two, and never kind 13.
        assert_eq!(kinds(&codex, MAYA_K2, &[SECONDARY, CONCENTRATED, AIR_RADAR]), [10, 11]);
        assert_eq!(kinds(&codex, MAYA_K2, &[SECONDARY, CONCENTRATED]), [11]);
        let with_director = [SECONDARY_DIRECTOR, CONCENTRATED, AIR_RADAR];
        assert_eq!(kinds(&codex, MAYA_K2, &with_director), [10, 11, 8]);
        // 飛龍改三 has kind 53 besides, and no kind 13 either.
        assert_eq!(kinds(&codex, HIRYUU_K3, &with_director), [10, 11, 8]);
        assert_eq!(kinds(&codex, HIRYUU_K3, &[HIGH_ANGLE_K_DIRECTOR, AIR_RADAR]), [53, 8]);
        // 五十鈴改二.
        assert_eq!(kinds(&codex, ISUZU_K2, &[HIGH_ANGLE, MACHINE_GUN, AIR_RADAR]), [14, 15]);
        assert_eq!(kinds(&codex, ISUZU_K2, &[HIGH_ANGLE, MACHINE_GUN]), [15]);
        // 鬼怒改二: kind 19 wants a 高角砲 without a director of its own.
        assert_eq!(kinds(&codex, KINU_K2, &[HIGH_ANGLE, CONCENTRATED]), [19, 20]);
        assert_eq!(kinds(&codex, KINU_K2, &[HIGH_ANGLE_DIRECTOR, CONCENTRATED]), [20]);
        // UIT-25: a 機銃 that is not 集中配備.
        assert_eq!(kinds(&codex, UIT_25, &[MACHINE_GUN]), [23]);
        assert_eq!(kinds(&codex, UIT_25, &[CONCENTRATED]), [0; 0]);
        // 白露改二 reaches 対空 70 and 白露改 does not.
        assert_eq!(kinds(&codex, SHIRATSUYU_K2, &[C_TYPE_K3H, C_TYPE_K3H]), [47]);
        assert_eq!(kinds(&codex, SHIRATSUYU_K2, &[C_TYPE_K3H, AIR_RADAR]), [47]);
        assert_eq!(kinds(&codex, SHIRATSUYU_K2, &[C_TYPE_K3H]), [0; 0]);
        assert_eq!(kinds(&codex, SHIRATSUYU_KAI, &[C_TYPE_K3H, C_TYPE_K3H]), [0; 0]);
    }

    #[test]
    fn equipment_named_by_id_forms_a_kind() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        assert_eq!(kinds(&codex, FLETCHER, &[MK30_GFCS, MK30_GFCS]), [34]);
        assert_eq!(kinds(&codex, FLETCHER, &[MK30_GFCS, MK30_KAI]), [35]);
        assert_eq!(kinds(&codex, FLETCHER, &[MK30_KAI, MK30_KAI, GFCS]), [37, 36]);
        assert_eq!(kinds(&codex, ATLANTA, &[DUAL_GFCS, DUAL_GFCS]), [38, 41]);
        assert_eq!(kinds(&codex, ATLANTA, &[DUAL_GFCS, DUAL, GFCS]), [39, 40, 41, 5, 8]);
        // 大和改二 with her gun groups and rangefinder; 大和改 and 武蔵改 share only kind 26.
        let groups = [GUN_GROUP, GUN_GROUP, RANGEFINDER, CONCENTRATED];
        assert_eq!(kinds(&codex, YAMATO_K2, &groups), [42, 43, 44, 45, 5, 8, 13]);
        assert_eq!(kinds(&codex, YAMATO_K2, &[GUN_GROUP, RANGEFINDER]), [45, 8]);
        assert_eq!(kinds(&codex, YAMATO_KAI, &[GUN_GROUP, RANGEFINDER]), [8]);
        for ship in [YAMATO_K2, YAMATO_KAI, MUSASHI_KAI] {
            assert_eq!(kinds(&codex, ship, &[HIGH_ANGLE_MACHINE_GUN, AIR_RADAR]), [8, 26]);
        }
        // 吹雪改二 and 秋月改 with 10cm連装高角砲改, its director and 94式高射装置.
        let twin = [HIGH_ANGLE_K_DIRECTOR, HIGH_ANGLE_K_DIRECTOR, AIR_RADAR];
        let mixed = [HIGH_ANGLE_K_DIRECTOR, HIGH_ANGLE_K, AIR_RADAR, DIRECTOR];
        assert_eq!(kinds(&codex, FUBUKI_K2, &twin), [49, 5, 8]);
        assert_eq!(kinds(&codex, FUBUKI_K2, &mixed), [50, 8, 7, 9]);
        assert_eq!(kinds(&codex, FUBUKI_K2, &[HIGH_ANGLE_K, AIR_RADAR, MACHINE_GUN]), [51]);
        assert_eq!(kinds(&codex, FUBUKI_K2, &[HIGH_ANGLE_K, HIGH_ANGLE_K, DIRECTOR]), [52, 9]);
        assert_eq!(kinds(&codex, AKIZUKI_KAI, &twin), [48, 1, 2, 3]);
        assert_eq!(kinds(&codex, AKIZUKI_KAI, &mixed), [50, 1, 2, 3, 9]);
        assert_eq!(kinds(&codex, AKIZUKI_KAI, &[HIGH_ANGLE_K, HIGH_ANGLE_K, DIRECTOR]), [52, 3, 9]);
        assert_eq!(kinds(&codex, KONGOU_K2, &[POM_POM, MK1_FCR]), [32]);
    }

    #[test]
    fn some_kinds_show_the_equipment_the_source_names() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let shown_by = |id: i64, mst_id: i64, equipment: &[i64]| {
            shown(kind(id), parts(&codex, &ship(&codex, mst_id, equipment)))
        };
        assert_eq!(shown_by(34, FLETCHER, &[MK30_GFCS, MK30_GFCS]), [MK30_GFCS, MK30_GFCS]);
        // Kind 40 shows the gun without its director even when the ship carries the other.
        assert_eq!(shown_by(40, ATLANTA, &[DUAL_GFCS, DUAL, GFCS]), [DUAL, DUAL, GFCS]);
        assert_eq!(shown_by(32, KONGOU_K2, &[POM_POM, MK1_FCR]), [POM_POM, MK1_FCR]);
        // The rest go by their letters, as before.
        assert_eq!(shown_by(38, ATLANTA, &[DUAL_GFCS, DUAL_GFCS]), [DUAL_GFCS, DUAL_GFCS]);
        let maya = [SECONDARY, CONCENTRATED, AIR_RADAR];
        assert_eq!(shown_by(10, MAYA_K2, &maya), maya);
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
