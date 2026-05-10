use emukc_battle::BattleShipInput;
use emukc_crypto::rng;
use emukc_model::{
    codex::{
        Codex,
        map::{EnemyComposition, EnemyFleetDefinition, MapDefinition, MapVariantDefinition},
    },
    kc2::{KcApiShip, KcApiSlotItem, UserHQRank, level, start2::ApiMstShip},
};
use tracing::warn;

use crate::err::GameplayError;

/// Fallback enemy ship: Abyssal DD I-class (駆逐イ級).
/// Used when map data is missing enemy fleet definitions.
const FALLBACK_ENEMY_SHIP_ID: i64 = 1501;

pub(super) fn build_sortie_enemy_ships(
    codex: &Codex,
    definition: &MapDefinition,
    enemy_fleet: &EnemyFleetDefinition,
    composition: &EnemyComposition,
) -> Result<(Vec<BattleShipInput>, i64, String, String), GameplayError> {
    let enemy_level = (definition.level.max(1) * 5 + enemy_fleet.cell_no).max(1);
    let enemy_rank = UserHQRank::RearAdmiral.get_name().to_string();
    let enemy_deck_name = format!("{}海域敵艦隊", definition.name);
    let ship_ids = if composition.ship_ids.is_empty() {
        vec![FALLBACK_ENEMY_SHIP_ID]
    } else {
        composition.ship_ids.clone()
    };

    let enemy_ships = ship_ids
        .into_iter()
        .map(|ship_id| build_sortie_enemy_ship(codex, ship_id, enemy_level))
        .collect::<Result<Vec<_>, GameplayError>>()?;

    Ok((enemy_ships, enemy_level, enemy_rank, enemy_deck_name))
}

pub(super) fn build_sortie_enemy_ship(
    codex: &Codex,
    ship_id: i64,
    enemy_level: i64,
) -> Result<BattleShipInput, GameplayError> {
    if let Some((mut api_ship, slot_items)) = codex.new_enemy_ship(ship_id) {
        let exp_now = level::ship_level_required_exp(enemy_level.min(99));
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        api_ship.api_lv = enemy_level;
        api_ship.api_exp = [exp_now, next_exp, 0];
        return Ok(BattleShipInput {
            ship: api_ship,
            slot_items,
            effect_list: vec![0],
            married: false,
        });
    }

    if let Some((mut api_ship, slot_items)) = codex.new_ship(ship_id) {
        warn!(ship_id, "enemy bootstrap data missing; using ship_extra fallback for sortie enemy",);
        let exp_now = level::ship_level_required_exp(enemy_level.min(99));
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        api_ship.api_lv = enemy_level;
        api_ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut api_ship, &slot_items, false)?;
        for (idx, slot_item) in slot_items.iter().take(5).enumerate() {
            api_ship.api_slot[idx] = slot_item.api_slotitem_id;
        }
        return Ok(BattleShipInput {
            ship: api_ship,
            slot_items,
            effect_list: vec![0],
            married: false,
        });
    }

    let mst =
        codex.manifest.find_ship(ship_id).ok_or_else(|| {
            warn!(
                ship_id,
                "enemy bootstrap data missing and no manifest entry found for sortie enemy",
            );
            GameplayError::ManifestNotFound(ship_id)
        })?;
    Ok(build_manifest_only_sortie_enemy_ship(mst, ship_id, enemy_level))
}

#[derive(Debug)]
struct ManifestOnlyEnemyStats {
    sortno: i64,
    hp: [i64; 2],
    firepower: [i64; 2],
    torpedo: [i64; 2],
    aa: [i64; 2],
    armor: [i64; 2],
    asw: [i64; 2],
    luck: [i64; 2],
    range: i64,
    backs: i64,
    fuel: i64,
    bull: i64,
    missing_fields: Vec<&'static str>,
}

fn build_manifest_only_sortie_enemy_ship(
    mst: &ApiMstShip,
    ship_id: i64,
    enemy_level: i64,
) -> BattleShipInput {
    let fallback = manifest_only_enemy_stats(mst);
    if fallback.missing_fields.is_empty() {
        warn!(ship_id, "enemy bootstrap data missing; using manifest-only sortie enemy fallback",);
    } else {
        warn!(
            ship_id,
            missing_fields = ?fallback.missing_fields,
            "enemy bootstrap data missing; using degraded manifest-only sortie enemy fallback",
        );
    }
    let exp_now = level::ship_level_required_exp(enemy_level.min(99));
    let (_, next_exp) = level::exp_to_ship_level(exp_now);
    let hp = fallback.hp;
    let api_ship = KcApiShip {
        api_id: 0,
        api_sortno: fallback.sortno,
        api_ship_id: ship_id,
        api_lv: enemy_level,
        api_exp: [exp_now, next_exp, 0],
        api_nowhp: hp[0].max(1),
        api_maxhp: hp[0].max(1),
        api_soku: mst.api_soku,
        api_leng: fallback.range,
        api_slot: [-1; 5],
        api_onslot: [0; 5],
        api_slot_ex: 0,
        api_kyouka: [0; 7],
        api_backs: fallback.backs,
        api_fuel: fallback.fuel,
        api_bull: fallback.bull,
        api_slotnum: mst.api_slot_num,
        api_ndock_time: 0,
        api_ndock_item: [0, 0],
        api_srate: 0,
        api_cond: 49,
        api_karyoku: fallback.firepower,
        api_raisou: fallback.torpedo,
        api_taiku: fallback.aa,
        api_soukou: fallback.armor,
        api_kaihi: [0, 0],
        api_taisen: fallback.asw,
        api_sakuteki: [0, 0],
        api_lucky: fallback.luck,
        api_locked: 0,
        api_locked_equip: 0,
        api_sally_area: 0,
        api_sp_effect_items: None,
    };

    BattleShipInput {
        ship: api_ship,
        slot_items: Vec::<KcApiSlotItem>::new(),
        effect_list: vec![0],
        married: false,
    }
}

fn manifest_only_enemy_stats(mst: &ApiMstShip) -> ManifestOnlyEnemyStats {
    let mut missing_fields = Vec::new();
    let _ = manifest_onslot_or_default(mst.api_maxeq, "api_maxeq", &mut missing_fields);
    ManifestOnlyEnemyStats {
        sortno: mst.api_sortno.unwrap_or(mst.api_sort_id),
        hp: manifest_pair_or_default(mst.api_taik, [1, 1], "api_taik", &mut missing_fields),
        firepower: manifest_pair_or_default(mst.api_houg, [0, 0], "api_houg", &mut missing_fields),
        torpedo: manifest_pair_or_default(mst.api_raig, [0, 0], "api_raig", &mut missing_fields),
        aa: manifest_pair_or_default(mst.api_tyku, [0, 0], "api_tyku", &mut missing_fields),
        armor: manifest_pair_or_default(mst.api_souk, [0, 0], "api_souk", &mut missing_fields),
        asw: manifest_single_pair_or_default(mst.api_tais, [0, 0], "api_tais", &mut missing_fields),
        luck: manifest_pair_or_default(mst.api_luck, [0, 0], "api_luck", &mut missing_fields),
        range: mst.api_leng.unwrap_or(-1),
        backs: mst.api_backs.unwrap_or(-1),
        fuel: mst.api_fuel_max.unwrap_or(0),
        bull: mst.api_bull_max.unwrap_or(0),
        missing_fields,
    }
}

fn manifest_pair_or_default(
    value: Option<[i64; 2]>,
    default: [i64; 2],
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 2] {
    value.unwrap_or_else(|| {
        missing_fields.push(field);
        default
    })
}

fn manifest_single_pair_or_default(
    value: Option<[i64; 1]>,
    default: [i64; 2],
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 2] {
    value.map(|[stat]| [stat, stat]).unwrap_or_else(|| {
        missing_fields.push(field);
        default
    })
}

fn manifest_onslot_or_default(
    value: Option<[i64; 5]>,
    field: &'static str,
    missing_fields: &mut Vec<&'static str>,
) -> [i64; 5] {
    value.unwrap_or_else(|| {
        missing_fields.push(field);
        [0; 5]
    })
}

pub(super) fn resolve_sortie_enemy_fleet(
    map_id: i64,
    variant: &MapVariantDefinition,
    cell_no: i64,
) -> EnemyFleetDefinition {
    if let Some(enemy_fleet) = variant.enemy_fleets.get(&cell_no) {
        return enemy_fleet.clone();
    }

    warn!(
        map_id,
        cell_no, "missing enemy fleet definition for sortie cell; using fallback composition",
    );
    fallback_enemy_fleet(cell_no)
}

fn fallback_enemy_fleet(cell_no: i64) -> EnemyFleetDefinition {
    EnemyFleetDefinition {
        cell_no,
        battle_kind: 1,
        formations: vec![1],
        compositions: vec![fallback_enemy_composition(cell_no)],
    }
}

pub(super) fn fallback_enemy_composition(cell_no: i64) -> EnemyComposition {
    EnemyComposition {
        comp_id: format!("fallback:{cell_no}"),
        weight: 1,
        ship_ids: vec![FALLBACK_ENEMY_SHIP_ID],
        formation: Some(1),
        raw_ship_names: Vec::new(),
    }
}

pub(super) fn select_random_enemy_composition(
    enemy_fleet: &EnemyFleetDefinition,
) -> Option<EnemyComposition> {
    if enemy_fleet.compositions.is_empty() {
        return None;
    }

    let total_weight = enemy_fleet
        .compositions
        .iter()
        .map(|composition| composition.weight.max(1) as u64)
        .sum::<u64>();
    if total_weight == 0 {
        return enemy_fleet.compositions.first().cloned();
    }

    let roll = rng::u64(0..total_weight);
    select_enemy_composition_for_roll(enemy_fleet, roll).cloned()
}

pub(super) fn select_enemy_composition_for_roll(
    enemy_fleet: &EnemyFleetDefinition,
    mut roll: u64,
) -> Option<&EnemyComposition> {
    for composition in &enemy_fleet.compositions {
        let weight = composition.weight.max(1) as u64;
        if roll < weight {
            return Some(composition);
        }
        roll -= weight;
    }

    enemy_fleet.compositions.last()
}
