//! Aircraft proficiency: how a slot's experience rises and falls with the
//! battles it flies.
//!
//! When it changes follows the wikiwiki page 艦載機熟練度. The amounts are a
//! fit to player statistics, not anything read from the game: the article
//! <https://cc-jabberwock.hatenablog.com/entry/2023/08/08/211433> and its two
//! sheets (`1tGEtDid5q7ngve1m07TK5JO2MF2BCTd6OiyObPRpkqU` for the formulas,
//! `1lLRXpfBqxfoNxT8GPRvWQXU4V3txYWSfvgXw8ElRrfc` for the growth constants),
//! taken 2026-10-10. See `docs/plans/2026-10-10-005`.

use emukc_model::kc2::KcSlotItemType3;

/// The most experience a slot holds.
pub(crate) const EXP_MAX: i64 = 120;
/// The experience each level starts at.
const LEVEL_FLOOR: [i64; 8] = [0, 10, 25, 40, 55, 70, 85, 100];
/// The experience equipment handed out at a level is given: the same table the
/// battle reads a level with.
const LEVEL_EXP: [i64; 8] = [0, 10, 25, 40, 55, 70, 85, 120];
/// The growth constant of an aircraft the sheet does not list: its median.
const DEFAULT_GROWTH: i64 = 6;

/// The experience behind a level the client shows.
pub(crate) fn exp_of_level(level: i64) -> i64 {
    LEVEL_EXP[level.clamp(0, 7) as usize]
}

/// The level the client shows for an experience.
pub(crate) fn level_of_exp(exp: i64) -> i64 {
    LEVEL_FLOOR.iter().rposition(|floor| exp >= *floor).unwrap_or(0) as i64
}

fn growth_constant(mst_id: i64) -> i64 {
    GROWTH
        .binary_search_by_key(&mst_id, |(id, _)| *id)
        .map_or(DEFAULT_GROWTH, |found| GROWTH[found].1)
}

/// How many times a battle lets this kind of aircraft grow, or `None` for
/// equipment that has no proficiency here. Search always succeeds in the
/// simulation, so scouts grow in every battle.
// ponytail: patrol planes and autogyros (which grow when their ship attacks a
// submarine) and the jet assault's extra roll are left out.
fn growth_rolls(type3: i64, air_battle: bool) -> Option<i64> {
    use KcSlotItemType3 as T;
    match KcSlotItemType3::n(type3)? {
        T::CarrierBasedFighter
        | T::CarrierBasedDiveBomber
        | T::CarrierBasedTorpedoBomber
        | T::SeaplaneFighter
        | T::JetFighter
        | T::JetFighterBomber
        | T::JetAttacker => Some(i64::from(air_battle)),
        T::CarrierBasedRecon | T::SeaBasedRecon | T::LargeFlyingBoat => Some(1),
        T::SeaBasedBomber => Some(1 + i64::from(air_battle)),
        _ => None,
    }
}

/// Whether this kind of aircraft is a scout, which loses experience only by
/// being wiped out.
fn is_scout(type3: i64) -> bool {
    matches!(
        KcSlotItemType3::n(type3),
        Some(
            KcSlotItemType3::CarrierBasedRecon
                | KcSlotItemType3::SeaBasedRecon
                | KcSlotItemType3::LargeFlyingBoat
        )
    )
}

/// What one growth roll adds. `roll` is in `0..1`.
fn gain(constant: i64, exp: i64, roll: f64) -> i64 {
    let spread = if exp < 20 {
        12.0
    } else if exp < 50 {
        10.0
    } else if exp < 100 {
        8.0
    } else {
        6.0
    };
    (constant as f64 * (0.5 + 0.044 * roll * (spread - 1.0))).floor() as i64
}

/// What losing aircraft takes from a slot that still has some. `roll` is in `0..1`.
// ponytail: whole points each battle; the sheet keeps the fraction until port.
fn loss(exp: i64, before: i64, after: i64, roll: f64) -> i64 {
    // In tenths and whole aircraft, so that a tenth lost is exactly a tenth.
    let lost = before - after;
    let share = if 2 * lost > before {
        5
    } else {
        3
    };
    let base = (share * exp * lost / (10 * before)) as f64;
    (base * (0.5 - 0.05 * roll)).round() as i64
}

/// A slot's experience after one battle that left it `after` of its `before`
/// aircraft. A slot that flew nothing is untouched and one that lost
/// everything starts over; otherwise it loses for what was shot down, then
/// grows if `grows`.
pub(crate) fn settle(
    mst_id: i64,
    type3: i64,
    exp: i64,
    (before, after): (i64, i64),
    air_battle: bool,
    grows: bool,
    mut roll: impl FnMut() -> f64,
) -> i64 {
    let Some(rolls) = growth_rolls(type3, air_battle) else {
        return exp;
    };
    if before <= 0 {
        return exp;
    }
    if after <= 0 {
        return 0;
    }
    let mut settled = exp;
    if !is_scout(type3) {
        settled -= loss(exp, before, after, roll());
    }
    if grows {
        let constant = growth_constant(mst_id);
        for _ in 0..rolls {
            settled += gain(constant, exp, roll());
        }
    }
    settled.clamp(0, EXP_MAX)
}

/// Each aircraft's growth constant, by master id: how readily it gains
/// experience. Estimates from the sheet named at the top of this file, kept by
/// hand; an aircraft missing here takes [`DEFAULT_GROWTH`]. Sorted by id.
#[rustfmt::skip]
const GROWTH: &[(i64, i64)] = &[
    (16, 8), // 九七式艦攻
    (17, 7), // 天山
    (18, 7), // 流星
    (19, 9), // 九六式艦戦
    (20, 9), // 零式艦戦21型
    (21, 8), // 零式艦戦52型
    (22, 7), // 試製烈風 後期型
    (23, 8), // 九九式艦爆
    (24, 7), // 彗星
    (25, 9), // 零式水上偵察機
    (26, 8), // 瑞雲
    (52, 5), // 流星改
    (53, 6), // 烈風 一一型
    (54, 7), // 彩雲
    (55, 7), // 紫電改二
    (56, 5), // 震電改
    (57, 6), // 彗星一二型甲
    (59, 8), // 零式水上観測機
    (60, 7), // 零式艦戦62型(爆戦)
    (61, 6), // 二式艦上偵察機
    (62, 6), // 試製晴嵐
    (64, 7), // Ju87C改
    (69, 6), // カ号観測機
    (70, 7), // 三式指揮連絡機(対潜)
    (79, 6), // 瑞雲(六三四空)
    (80, 7), // 瑞雲12型
    (81, 6), // 瑞雲12型(六三四空)
    (82, 7), // 九七式艦攻(九三一空)
    (83, 8), // 天山(九三一空)
    (93, 6), // 九七式艦攻(友永隊)
    (94, 6), // 天山一二型(友永隊)
    (96, 7), // 零式艦戦21型(熟練)
    (97, 7), // 九九式艦爆(熟練)
    (98, 7), // 九七式艦攻(熟練)
    (99, 7), // 九九式艦爆(江草隊)
    (100, 5), // 彗星(江草隊)
    (102, 8), // 九八式水上偵察機(夜偵)
    (109, 6), // 零戦52型丙(六〇一空)
    (111, 7), // 彗星(六〇一空)
    (112, 6), // 天山(六〇一空)
    (113, 7), // 流星(六〇一空)
    (115, 9), // Ar196改
    (118, 7), // 紫雲
    (138, 8), // 二式大艇
    (143, 6), // 九七式艦攻(村田隊)
    (144, 5), // 天山一二型(村田隊)
    (148, 5), // 試製南山
    (151, 6), // 試製景雲(艦偵型)
    (152, 7), // 零式艦戦52型(熟練)
    (153, 6), // 零戦52型丙(付岩井小隊)
    (154, 6), // 零戦62型(爆戦/岩井隊)
    (155, 6), // 零戦21型(付岩本小隊)
    (156, 6), // 零戦52型甲(付岩本小隊)
    (157, 5), // 零式艦戦53型(岩本隊)
    (158, 8), // Bf109T改
    (159, 8), // Fw190T改
    (163, 7), // Ro.43水偵
    (164, 8), // Ro.44水上戦闘機
    (165, 6), // 二式水戦改
    (168, 9), // 九六式陸攻
    (169, 8), // 一式陸攻
    (170, 8), // 一式陸攻(野中隊)
    (171, 9), // OS2U
    (178, 7), // PBY-5A Catalina
    (180, 8), // 一式陸攻 二二型甲
    (181, 7), // 零式艦戦32型
    (182, 7), // 零式艦戦32型(熟練)
    (184, 8), // Re.2001 OR改
    (186, 8), // 一式陸攻 三四型
    (187, 7), // 銀河
    (188, 8), // Re.2001 G改
    (189, 6), // Re.2005 改
    (194, 5), // Laté 298B
    (195, 6), // SBD
    (196, 6), // TBD
    (197, 6), // F4F-3
    (198, 6), // F4F-4
    (199, 3), // 噴式景雲改
    (200, 4), // 橘花改
    (205, 6), // F6F-3
    (206, 6), // F6F-5
    (207, 4), // 瑞雲(六三一空)
    (208, 5), // 晴嵐(六三一空)
    (212, 6), // 彩雲(東カロリン空)
    (215, 7), // Ro.44水上戦闘機bis
    (216, 5), // 二式水戦改(熟練)
    (217, 5), // 強風改
    (219, 6), // 零式艦戦63型(爆戦)
    (224, 7), // 爆装一式戦 隼III型改(65戦隊)
    (228, 9), // 九六式艦戦改
    (233, 5), // F4U-1D
    (237, 5), // 瑞雲(六三四空／熟練)
    (238, 7), // 零式水上偵察機11型乙
    (239, 6), // 零式水上偵察機11型乙(熟練)
    (242, 9), // Swordfish
    (243, 9), // Swordfish Mk.II(熟練)
    (244, 9), // Swordfish Mk.III(熟練)
    (248, 8), // Skua
    (249, 8), // Fulmar
    (252, 7), // Seafire Mk.III改
    (254, 5), // F6F-3N
    (255, 5), // F6F-5N
    (256, 6), // TBF
    (257, 5), // TBM-3D
    (269, 6), // 試製東海
    (270, 6), // 東海(九〇一空)
    (271, 6), // 紫電改四
    (273, 6), // 彩雲(偵四)
    (277, 7), // FM-2
    (291, 7), // 彗星二二型(六三四空)
    (292, 6), // 彗星二二型(六三四空／熟練)
    (302, 6), // 九七式艦攻(九三一空/熟練)
    (304, 7), // S9 Osprey
    (305, 5), // Ju87C改二(KMX搭載機)
    (306, 4), // Ju87C改二(KMX搭載機/熟練)
    (311, 7), // 二式陸上偵察機
    (312, 6), // 二式陸上偵察機(熟練)
    (316, 8), // Re.2001 CB改
    (319, 6), // 彗星一二型(六三四空／三号爆弾搭載機)
    (320, 6), // 彗星一二型(三一号光電管爆弾搭載機)
    (322, 5), // 瑞雲改二(六三四空)
    (323, 5), // 瑞雲改二(六三四空／熟練)
    (324, 5), // オ号観測機改
    (325, 5), // オ号観測機改二
    (326, 4), // S-51J
    (327, 4), // S-51J改
    (335, 4), // 烈風改(試製艦載型)
    (336, 4), // 烈風改二
    (338, 3), // 烈風改二戊型
    (339, 2), // 烈風改二戊型(一航戦／熟練)
    (342, 4), // 流星改(一航戦)
    (343, 3), // 流星改(一航戦／熟練)
    (344, 5), // 九七式艦攻改 試製三号戊型(空六号電探改装備機)
    (345, 4), // 九七式艦攻改(熟練) 試製三号戊型(空六号電探改装備機)
    (353, 6), // Fw190 A-5改(熟練)
    (367, 8), // Swordfish(水上機型)
    (368, 7), // Swordfish Mk.III改(水上機型)
    (369, 7), // Swordfish Mk.III改(水上機型／熟練)
    (370, 7), // Swordfish Mk.II改(水偵型)
    (371, 7), // Fairey Seafox改
    (372, 6), // 天山一二型甲
    (373, 5), // 天山一二型甲改(空六号電探改装備機)
    (374, 4), // 天山一二型甲改(熟練/空六号電探改装備機)
    (375, 3), // XF5U
    (388, 6), // 銀河(江草隊)
    (389, 4), // TBM-3W ＋ 3S
    (391, 8), // 九九式艦爆二二型
    (392, 7), // 九九式艦爆二二型(熟練)
    (395, 6), // 深山
    (396, 7), // 深山改
    (401, 7), // Do 17 Z-2
    (403, 7), // 四式重爆 飛龍
    (404, 6), // 四式重爆 飛龍(熟練)
    (405, 5), // Do 217 E-5＋Hs293初期型
    (406, 5), // Do 217 K-2＋Fritz-X
    (414, 9), // SOC Seagull
    (415, 7), // SO3C Seamew改
    (419, 6), // SBD-5
    (420, 6), // SB2C-3
    (421, 6), // SB2C-5
    (422, 4), // FR-1 Fireball
    (423, 6), // Fulmar(戦闘偵察／熟練)
    (424, 6), // Barracuda Mk.II
    (425, 6), // Barracuda Mk.III
    (431, 7), // SM.79
    (432, 7), // SM.79 bis
    (433, 6), // SM.79 bis(熟練)
    (434, 5), // Corsair Mk.II
    (435, 4), // Corsair Mk.II(Ace)
    (437, 4), // 試製 陣風
    (444, 6), // 四式重爆 飛龍＋イ号一型甲 誘導弾
    (447, 6), // 零式艦戦64型(複座KMX搭載機)
    (451, 6), // 三式指揮連絡機改
    (453, 7), // キ102乙
    (454, 6), // キ102乙改+イ号一型乙 誘導弾
    (459, 7), // B-25
    (466, 4), // 流星改(熟練)
    (469, 7), // 零式水上偵察機11型乙改(夜偵)
    (471, 7), // Loire 130M
    (473, 5), // F4U-2 Night Corsair
    (474, 6), // F4U-4
    (475, 6), // AU-1
    (476, 6), // F4U-7
    (479, 6), // Mosquito FB Mk.VI
    (480, 5), // Mosquito PR Mk.IV
    (481, 6), // Mosquito TR Mk.33
    (484, 5), // 四式重爆 飛龍(熟練)＋イ号一型甲 誘導弾
    (485, 5), // 強風改二
    (486, 6), // 零式艦戦64型(制空戦闘機仕様)
    (487, 7), // 零式艦戦64型(熟練爆戦)
    (489, 6), // 一式戦 隼II型改(20戦隊)
    (490, 5), // 試製 夜間瑞雲(攻撃装備)
    (491, 6), // 一式戦 隼III型改(熟練／20戦隊)
    (492, 6), // 零戦52型丙(八幡部隊)
    (493, 8), // 一式陸攻(八幡部隊)
    (504, 6), // 銀河(熟練)
    (510, 8), // Walrus
    (515, 7), // Sea Otter
    (521, 6), // 紫雲(熟練)
    (522, 7), // 零式小型水上機
    (523, 6), // 零式小型水上機(熟練)
    (538, 6), // Loire 130M改(熟練)
    (539, 7), // SOC Seagull 後期型(熟練)
    (540, 7), // 零式水上偵察機11型甲改二
    (541, 7), // SBD(Yellow Wings)
    (542, 7), // TBD(Yellow Wings)
    (543, 6), // SBD VS-2(偵察飛行隊)
    (544, 5), // SBD VB-2(爆撃飛行隊)
    (545, 4), // 天山一二型甲改二(村田隊/電探装備)
    (547, 5), // 震電改二(艦戦型改二)
    (548, 4), // 震電改三(試製 噴式震電)
    (549, 5), // 三式指揮連絡機改二
    (550, 7), // 試製 明星(増加試作機)
    (551, 6), // 明星改
    (552, 5), // 九九式練爆二二型改(夜間装備実験機)
    (554, 7), // 九七式艦攻改(北東海軍航空隊)
    (557, 6), // 零式艦戦62型改(夜間爆戦)
    (558, 5), // 零式艦戦62型改(熟練/夜間爆戦)
    (559, 6), // Ju87 D-4(Fliegerass)
    (560, 7), // Bf109 T-3(G)
    (561, 3), // Ho229
    (562, 5), // Do 217 E-5+TV誘導型 Hs293D
    (567, 8), // Sea Gladiator
    (568, 5), // 強風改二(熟練)
    (570, 3), // 流星改(友永隊)
    (578, 7), // SB2U-2
    (585, 8), // PL101(偵察)
    (586, 8), // PL101(爆装)
    (587, 7), // V-156F(SB2U輸出型)
    (588, 6), // G-36A(F4F輸出型)
];

#[cfg(test)]
mod tests {
    use super::*;

    const FIGHTER: i64 = KcSlotItemType3::CarrierBasedFighter as i64;
    const RECON: i64 = KcSlotItemType3::SeaBasedRecon as i64;
    const SEAPLANE_BOMBER: i64 = KcSlotItemType3::SeaBasedBomber as i64;

    #[test]
    fn levels_and_experience_agree_at_the_thresholds() {
        assert_eq!(level_of_exp(0), 0);
        assert_eq!(level_of_exp(9), 0);
        assert_eq!(level_of_exp(10), 1);
        assert_eq!(level_of_exp(99), 6);
        assert_eq!(level_of_exp(100), 7);
        assert_eq!(level_of_exp(EXP_MAX), 7);
        for level in 0..=7 {
            assert_eq!(level_of_exp(exp_of_level(level)), level);
        }
    }

    #[test]
    fn the_table_is_sorted_and_falls_back_to_the_median() {
        assert!(GROWTH.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert_eq!(growth_constant(19), 9); // 九六式艦戦
        assert_eq!(growth_constant(343), 3); // 流星改(一航戦/熟練)
        assert_eq!(growth_constant(-1), DEFAULT_GROWTH);
    }

    #[test]
    fn a_roll_adds_between_half_the_constant_and_the_bands_ceiling() {
        // A constant of 2 always adds 1, as the article says.
        for exp in [0, 30, 70, 110] {
            assert_eq!(gain(2, exp, 0.0), 1);
            assert_eq!(gain(2, exp, 0.999), 1);
        }
        // The spread narrows as experience rises: 12, 10, 8, 6.
        assert_eq!(gain(9, 0, 0.0), 4);
        assert_eq!(gain(9, 19, 0.999), 8);
        assert_eq!(gain(9, 20, 0.999), 8);
        assert_eq!(gain(9, 50, 0.999), 7);
        assert_eq!(gain(9, 100, 0.999), 6);
    }

    #[test]
    fn losses_follow_the_articles_two_examples() {
        // 40 aircraft down to 36 at 100: between 1.35 and 1.5.
        assert_eq!(loss(100, 40, 36, 0.0), 2);
        assert_eq!(loss(100, 40, 36, 0.999), 1);
        // 30 down to 1: between 21.6 and 24.
        assert_eq!(loss(100, 30, 1, 0.0), 24);
        assert_eq!(loss(100, 30, 1, 0.999), 22);
        // Nothing lost, nothing taken.
        assert_eq!(loss(100, 18, 18, 0.0), 0);
    }

    #[test]
    fn a_battle_settles_a_slot() {
        let low = || 0.0;
        // A fighter grows in an air battle and not without one.
        assert_eq!(settle(19, FIGHTER, 0, (18, 18), true, true, low), 4);
        assert_eq!(settle(19, FIGHTER, 0, (18, 18), false, true, low), 0);
        // A scout grows in any battle; a seaplane bomber twice in an air battle.
        assert_eq!(settle(-1, RECON, 0, (2, 2), false, true, low), 3);
        assert_eq!(settle(-1, SEAPLANE_BOMBER, 0, (2, 2), true, true, low), 6);
        // An escort's aircraft only lose.
        assert_eq!(settle(19, FIGHTER, 100, (30, 1), true, false, low), 76);
        // A wiped slot starts over, an empty one is left alone.
        assert_eq!(settle(19, FIGHTER, 100, (18, 0), true, true, low), 0);
        assert_eq!(settle(19, FIGHTER, 100, (0, 0), true, true, low), 100);
        // Nothing passes the ceiling, and a gun has no proficiency.
        assert_eq!(settle(19, FIGHTER, 119, (18, 18), true, true, low), EXP_MAX);
        assert_eq!(settle(1, 1, 50, (1, 0), true, true, low), 50);
    }
}
