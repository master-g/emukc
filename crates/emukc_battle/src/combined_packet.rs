//! Rewrite a finished combined-fleet packet into the client's index space.
//!
//! The simulation packs 第2艦隊 directly behind 第1艦隊 in one vector so that a
//! phase where the enemy fires at the whole friendly force is just the whole
//! slice (see [`BattleState`](crate::state::BattleState)). The client does not
//! read it that way: `_getNum` dispatches on `index >= 6` and then reads
//! `combined[index - 6]`, so 第2艦隊 always starts at 6 however few ships 第1艦隊
//! holds. Every friendly ship index the packet carries has to move, and every
//! friendly-indexed array has to grow to the fixed 12 slots
//! (`docs/apilist.txt:3096`: `api_raigeki.api_frai` is `[12]`).
//!
//! This runs once, at the end of the simulation, so nothing upstream has to
//! know about two index spaces. `BattlePacket::friendly_nowhps` is deliberately
//! left alone — it never reaches the wire (the day response reports entry HP
//! from the inputs) and the sortie session indexes it by fleet position.

use crate::combined::{ESCORT_INDEX_OFFSET, packet_index};
use crate::types::packet::{
    BattleHougeki, BattleKouku, BattleKoukuStage3Combined, BattleNightHougeki, BattleOpeningAttack,
    BattleRaigeki, DamageCell,
};
use crate::types::{BattlePacket, NightBattlePacket};

/// Width of the friendly half of every per-ship array in a combined packet.
const COMBINED_FRIENDLY_WIDTH: usize = ESCORT_INDEX_OFFSET * 2;

/// Move a friendly-indexed array into packet index space, padding to 12 slots.
fn respread<T: Clone>(values: Vec<T>, escort_start: usize, fill: &T) -> Vec<T> {
    let mut out = vec![fill.clone(); COMBINED_FRIENDLY_WIDTH];
    for (index, value) in values.into_iter().enumerate() {
        if let Some(slot) = out.get_mut(packet_index(index, escort_start)) {
            *slot = value;
        }
    }
    out
}

/// Rewrite one friendly ship index. Negatives are the API's "no target"
/// sentinels (`api_frai` / `api_erai` blank to `-1`) and pass through untouched.
fn remap_index(index: i64, escort_start: usize) -> i64 {
    if index < 0 {
        return index;
    }
    packet_index(index as usize, escort_start) as i64
}

/// Rewrite the friendly indices a list of defenders points at.
fn remap_defenders(defenders: &mut [i64], escort_start: usize) {
    for defender in defenders {
        *defender = remap_index(*defender, escort_start);
    }
}

/// Rewrite one shelling round.
///
/// `api_at_eflag` decides which end of each entry is friendly: with a friendly
/// attacker (`0`) the attacker index moves and the defenders are enemies; with
/// an enemy attacker (`1`) it is the other way round.
fn remap_hougeki(hougeki: &mut BattleHougeki, escort_start: usize) {
    let BattleHougeki {
        api_at_eflag,
        api_at_list,
        api_df_list,
        ..
    } = hougeki;
    for (entry, &eflag) in api_at_eflag.iter().enumerate() {
        if eflag == 0 {
            if let Some(attacker) = api_at_list.get_mut(entry) {
                *attacker = remap_index(*attacker, escort_start);
            }
        } else if let Some(defenders) = api_df_list.get_mut(entry) {
            remap_defenders(defenders, escort_start);
        }
    }
}

/// Rewrite one night shelling round. `api_n_mother_list` carries no ship index
/// — the simulation always writes `0` — so it is left as is.
fn remap_night_hougeki(hougeki: &mut BattleNightHougeki, escort_start: usize) {
    let BattleNightHougeki {
        api_at_eflag,
        api_at_list,
        api_df_list,
        ..
    } = hougeki;
    for (entry, &eflag) in api_at_eflag.iter().enumerate() {
        if eflag == 0 {
            if let Some(attacker) = api_at_list.get_mut(entry) {
                *attacker = remap_index(*attacker, escort_start);
            }
        } else if let Some(defenders) = api_df_list.get_mut(entry) {
            remap_defenders(defenders, escort_start);
        }
    }
}

/// Rewrite the opening torpedo payload.
///
/// The `f`-prefixed arrays are indexed by friendly ship and move; the
/// `e`-prefixed ones are indexed by enemy ship and stay, but the targets
/// recorded in `api_erai_list_items` are friendly and move with the rest. The
/// enemy arrays are cut back to the enemy's own size, which the simulation
/// over-allocates to `max(friendly, enemy)`.
fn remap_opening_attack(attack: &mut BattleOpeningAttack, escort_start: usize, enemy_len: usize) {
    attack.api_frai_list_items =
        respread(std::mem::take(&mut attack.api_frai_list_items), escort_start, &None);
    attack.api_fcl_list_items =
        respread(std::mem::take(&mut attack.api_fcl_list_items), escort_start, &None);
    attack.api_fydam_list_items =
        respread(std::mem::take(&mut attack.api_fydam_list_items), escort_start, &None);
    attack.api_fdam =
        respread(std::mem::take(&mut attack.api_fdam), escort_start, &DamageCell::Plain(0));

    for row in attack.api_erai_list_items.iter_mut().flatten() {
        remap_defenders(row, escort_start);
    }
    attack.api_erai_list_items.truncate(enemy_len);
    attack.api_ecl_list_items.truncate(enemy_len);
    attack.api_eydam_list_items.truncate(enemy_len);
    attack.api_edam.truncate(enemy_len);
}

/// Rewrite the closing torpedo payload — same index rules as the opening one.
fn remap_raigeki(raigeki: &mut BattleRaigeki, escort_start: usize, enemy_len: usize) {
    raigeki.api_frai = respread(std::mem::take(&mut raigeki.api_frai), escort_start, &-1);
    raigeki.api_fcl = respread(std::mem::take(&mut raigeki.api_fcl), escort_start, &0);
    raigeki.api_fdam =
        respread(std::mem::take(&mut raigeki.api_fdam), escort_start, &DamageCell::Plain(0));
    raigeki.api_fydam =
        respread(std::mem::take(&mut raigeki.api_fydam), escort_start, &DamageCell::Plain(0));

    remap_defenders(&mut raigeki.api_erai, escort_start);
    raigeki.api_erai.truncate(enemy_len);
    raigeki.api_ecl.truncate(enemy_len);
    raigeki.api_edam.truncate(enemy_len);
    raigeki.api_eydam.truncate(enemy_len);
}

/// Split the airstrike's stage 3 in two: `api_stage3` keeps 第1艦隊 and the
/// whole enemy fleet, `api_stage3_combined` takes 第2艦隊
/// (`docs/apilist.txt:3070`).
///
/// This one splits rather than respreads — the client reads each half by its
/// own deck's ship count, so no gap is needed.
fn split_kouku_stage3(kouku: &mut BattleKouku, escort_start: usize) {
    let stage3 = &mut kouku.api_stage3;
    let split = |values: &mut Vec<i64>| values.split_off(escort_start.min(values.len()));

    let api_frai = split(&mut stage3.api_frai);
    let api_fbak = split(&mut stage3.api_fbak);
    let api_frai_flag = split(&mut stage3.api_frai_flag);
    let api_fbak_flag = split(&mut stage3.api_fbak_flag);
    let api_fcl_flag = split(&mut stage3.api_fcl_flag);
    let api_fdam = stage3.api_fdam.split_off(escort_start.min(stage3.api_fdam.len()));
    let api_f_sp_list =
        stage3.api_f_sp_list.split_off(escort_start.min(stage3.api_f_sp_list.len()));

    kouku.api_stage3_combined = Some(BattleKoukuStage3Combined {
        api_frai,
        api_fbak,
        api_frai_flag,
        api_fbak_flag,
        api_fcl_flag,
        api_fdam,
        api_f_sp_list,
    });
}

/// Rewrite a whole day packet for a combined fleet whose 第2艦隊 starts at
/// `escort_start` in the simulation's friendly vector.
pub(crate) fn remap_day_packet(packet: &mut BattlePacket, escort_start: usize, enemy_len: usize) {
    if let Some(kouku) = packet.kouku.as_mut() {
        split_kouku_stage3(kouku, escort_start);
    }
    if let Some(taisen) = packet.opening_taisen.as_mut() {
        remap_hougeki(taisen, escort_start);
    }
    if let Some(attack) = packet.opening_attack.as_mut() {
        remap_opening_attack(attack, escort_start, enemy_len);
    }
    for hougeki in
        [&mut packet.hougeki1, &mut packet.hougeki2, &mut packet.hougeki3].into_iter().flatten()
    {
        remap_hougeki(hougeki, escort_start);
    }
    if let Some(raigeki) = packet.raigeki.as_mut() {
        remap_raigeki(raigeki, escort_start, enemy_len);
    }
}

/// Rewrite a night packet whose friendly side is 第2艦隊 alone.
///
/// A combined night battle is fought by 第2艦隊 only, so the simulation's
/// friendly vector *is* the escort deck: `escort_start` is 0 and every friendly
/// index shifts by the full offset. The HP arrays stay deck-2-sized; the
/// response builder pairs them with 第1艦隊's own, which never entered the
/// simulation.
pub(crate) fn remap_night_packet(packet: &mut NightBattlePacket) {
    if let Some(hougeki) = packet.hougeki.as_mut() {
        remap_night_hougeki(hougeki, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Summary arrays with no shield flag anywhere.
    fn plain(values: &[i64]) -> Vec<DamageCell> {
        values.iter().copied().map(DamageCell::Plain).collect()
    }
    use crate::types::packet::{BattleKoukuStage3, SiListId};

    fn hougeki(at_eflag: Vec<i64>, at_list: Vec<i64>, df_list: Vec<Vec<i64>>) -> BattleHougeki {
        let entries = at_eflag.len();
        BattleHougeki {
            api_at_eflag: at_eflag,
            api_at_list: at_list,
            api_at_type: vec![0; entries],
            api_df_list: df_list,
            api_si_list: vec![SiListId::num_from_i64(&[-1]); entries],
            api_cl_list: vec![vec![1]; entries],
            api_damage: vec![vec![DamageCell::Plain(10)]; entries],
        }
    }

    /// Deck 2's fourth ship sits at contiguous index 7 when deck 1 holds four,
    /// and the client must see it at 9.
    #[test]
    fn friendly_attacker_index_moves_enemy_one_stays() {
        let mut round = hougeki(vec![0, 1], vec![7, 2], vec![vec![1], vec![7]]);
        remap_hougeki(&mut round, 4);

        assert_eq!(round.api_at_list, vec![9, 2], "only the friendly attacker moves");
        assert_eq!(
            round.api_df_list,
            vec![vec![1], vec![9]],
            "enemy defenders stay, friendly defenders move"
        );
    }

    #[test]
    fn night_round_shifts_every_friendly_index_by_the_full_offset() {
        let mut round = BattleNightHougeki {
            api_at_eflag: vec![0, 1],
            api_at_list: vec![2, 0],
            api_n_mother_list: vec![0, 0],
            api_df_list: vec![vec![0], vec![2]],
            api_si_list: vec![SiListId::num_from_i64(&[-1]); 2],
            api_cl_list: vec![vec![1]; 2],
            api_sp_list: vec![-1, -1],
            api_damage: vec![vec![DamageCell::Plain(10)]; 2],
        };
        remap_night_hougeki(&mut round, 0);

        assert_eq!(round.api_at_list, vec![8, 0], "deck 2's 3rd ship is packet index 8");
        assert_eq!(round.api_df_list, vec![vec![0], vec![8]]);
        assert_eq!(round.api_n_mother_list, vec![0, 0], "mother list carries no ship index");
    }

    /// The friendly half grows to 12 with deck 2 parked at 6; the enemy half is
    /// cut back to the enemy's own size.
    #[test]
    fn raigeki_friendly_arrays_pad_to_twelve_and_enemy_arrays_shrink() {
        let mut raigeki = BattleRaigeki::blank(9);
        // Deck 1 has four ships, so contiguous 4 is deck 2's flagship.
        raigeki.api_frai[4] = 2;
        raigeki.api_fcl[4] = 1;
        raigeki.api_fydam[4] = DamageCell::Plain(33);
        raigeki.api_fdam[1] = DamageCell::Plain(12);
        // An enemy torpedoed contiguous friendly 5 — deck 2's second ship.
        raigeki.api_erai[0] = 5;

        remap_raigeki(&mut raigeki, 4, 3);

        assert_eq!(raigeki.api_frai.len(), 12);
        assert_eq!(raigeki.api_frai[6], 2, "deck 2 flagship moved 4 -> 6");
        assert_eq!(raigeki.api_frai[4], -1, "the gap keeps the blank fill");
        assert_eq!(raigeki.api_fcl[6], 1);
        assert_eq!(raigeki.api_fydam[6], DamageCell::Plain(33));
        assert_eq!(raigeki.api_fdam[1], DamageCell::Plain(12), "deck 1 indices do not move");
        assert_eq!(raigeki.api_erai[0], 7, "the friendly target moved 5 -> 7");
        assert_eq!(raigeki.api_erai.len(), 3, "enemy arrays shrink to the enemy fleet");
        assert_eq!(raigeki.api_edam.len(), 3);
    }

    #[test]
    fn opening_attack_follows_the_same_rules() {
        let mut attack = BattleOpeningAttack::blank(9);
        attack.api_frai_list_items[4] = Some(vec![1]);
        attack.api_fydam_list_items[4] = Some(vec![DamageCell::Plain(40)]);
        attack.api_fdam[5] = DamageCell::Plain(7);
        attack.api_erai_list_items[1] = Some(vec![4, 0]);

        remap_opening_attack(&mut attack, 4, 3);

        assert_eq!(attack.api_frai_list_items.len(), 12);
        assert_eq!(attack.api_frai_list_items[6], Some(vec![1]));
        assert_eq!(attack.api_fydam_list_items[6], Some(vec![DamageCell::Plain(40)]));
        assert_eq!(attack.api_fdam[7], DamageCell::Plain(7));
        assert_eq!(attack.api_erai_list_items[1], Some(vec![6, 0]));
        assert_eq!(attack.api_edam.len(), 3);
    }

    #[test]
    fn stage3_splits_the_friendly_arrays_and_leaves_the_enemy_whole() {
        let mut kouku = BattleKouku {
            api_plane_from: [vec![0], vec![-1]],
            api_stage1: crate::types::packet::BattleKoukuStage1 {
                api_f_count: 0,
                api_f_lostcount: 0,
                api_e_count: 0,
                api_e_lostcount: 0,
                api_disp_seiku: 0,
                api_touch_plane: [-1, -1],
            },
            api_stage2: crate::types::packet::BattleKoukuStage2 {
                api_f_count: 0,
                api_f_lostcount: 0,
                api_e_count: 0,
                api_e_lostcount: 0,
            },
            api_stage3: BattleKoukuStage3 {
                api_frai: vec![-1; 6],
                api_erai: vec![-1; 3],
                api_fbak: vec![-1; 6],
                api_ebak: vec![-1; 3],
                api_frai_flag: vec![0, 0, 0, 0, 1, 0],
                api_erai_flag: vec![0; 3],
                api_fbak_flag: vec![0; 6],
                api_ebak_flag: vec![0; 3],
                api_fcl_flag: vec![0; 6],
                api_ecl_flag: vec![0; 3],
                api_fdam: plain(&[1, 2, 3, 4, 5, 6]),
                api_edam: plain(&[0, 0, 0]),
                api_f_sp_list: vec![None; 6],
                api_e_sp_list: vec![None; 3],
            },
            api_stage3_combined: None,
        };

        split_kouku_stage3(&mut kouku, 4);

        assert_eq!(kouku.api_stage3.api_fdam, plain(&[1, 2, 3, 4]), "deck 1 keeps api_stage3");
        assert_eq!(kouku.api_stage3.api_edam.len(), 3, "the enemy half is untouched");
        let combined = kouku.api_stage3_combined.expect("deck 2 half must exist");
        assert_eq!(combined.api_fdam, plain(&[5, 6]));
        assert_eq!(combined.api_frai_flag, vec![1, 0]);
    }
}
