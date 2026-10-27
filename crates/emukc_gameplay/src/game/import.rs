//! Import an account captured from the live server into a fresh profile.
//!
//! The order of the writes and `id: NotSet` are a matter of correctness, not style:
//! `ship.id` and `slot_item.id` are auto-increment keys shared by every profile, so a
//! live instance id cannot be written as is. Items go in first, in ascending live id,
//! then ships with their slots mapped to the new item ids, then the items' `equip_on`,
//! then whatever points at a ship. Inserting in ascending live id keeps the client's
//! 入手順, which sorts by id.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use emukc_db::{
    entity::profile::{
        self, fleet,
        furniture::record as furniture_record,
        item::slot_item,
        kdock, material, ndock,
        settings::game as game_settings,
        ship::{self, sp_effect_item},
    },
    sea_orm::{ActiveValue, IntoActiveModel, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    kc2::{
        KcApiOptionSetting, KcApiShip, KcApiSlotItem, KcUseItemType, live::LiveSnapshot,
        start2::ApiMstSlotitem,
    },
    profile::furniture::FurnitureConfig,
};
use emukc_time::chrono::{DateTime, Utc};

use super::{
    fleet::unlock_fleet_impl,
    furniture::update_furniture_config_impl,
    picturebook::{add_ship_to_picturebook_impl, add_slot_item_to_picturebook_impl},
    proficiency::exp_of_level,
    settings::{option::update_options_settings_impl, oss::update_oss_settings_impl},
    use_item::add_use_item_impl,
};
use crate::{err::GameplayError, gameplay::Ctx};

/// Master ids in a capture that the codex does not know.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MissingMasters {
    /// Ship master ids.
    pub ships: Vec<i64>,
    /// Equipment master ids.
    pub slot_items: Vec<i64>,
}

impl MissingMasters {
    /// Whether the codex knows every master id in the capture.
    pub fn is_empty(&self) -> bool {
        self.ships.is_empty() && self.slot_items.is_empty()
    }
}

/// List the ship and equipment master ids of `snapshot` that `codex` does not have.
///
/// An import with any of them would leave a fleet with a ship but not its equipment,
/// which is worse than no import, so it is refused before anything is written.
pub fn missing_masters(codex: &Codex, snapshot: &LiveSnapshot) -> MissingMasters {
    let ships: BTreeSet<i64> = snapshot
        .ships
        .iter()
        .map(|ship| ship.api_ship_id)
        .filter(|id| codex.manifest.find_ship(*id).is_none())
        .collect();
    let slot_items: BTreeSet<i64> = snapshot
        .slot_items
        .iter()
        .map(|item| item.api_slotitem_id)
        .filter(|id| codex.find::<ApiMstSlotitem>(id).is_err())
        .collect();
    MissingMasters {
        ships: ships.into_iter().collect(),
        slot_items: slot_items.into_iter().collect(),
    }
}

/// One stat where the local formulas disagree with the live server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMismatch {
    /// The field, as `api_karyoku[0]`.
    pub field: &'static str,
    /// What the live server sent.
    pub live: i64,
    /// What the local formulas give.
    pub local: i64,
}

/// A ship whose recalculated stats differ from the live ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipMismatch {
    /// The live instance id.
    pub live_id: i64,
    /// The ship master id.
    pub mst_id: i64,
    /// The fields that differ.
    pub fields: Vec<FieldMismatch>,
}

/// What an import wrote, and where the local formulas disagree with the live server.
///
/// Holds counts, field names and ids only, never the nickname, member id or comment,
/// so it can be pasted into a PR.
#[derive(Debug, Default)]
pub struct ImportReport {
    /// Ships written.
    pub ships: usize,
    /// Equipment written.
    pub slot_items: usize,
    /// Fleets written.
    pub fleets: usize,
    /// Use items written, the furniture coin included.
    pub use_items: usize,
    /// Furniture owned.
    pub furniture: usize,
    /// Each mismatch comes from a local formula, the codex's master data, or a known
    /// approximation.
    pub mismatches: Vec<ShipMismatch>,
}

/// What the first slice fills in itself or leaves alone, printed with every report.
const NOTES: &[&str] = &[
    "approximated: `married` is `api_lv >= 100`",
    "approximated: an aircraft's proficiency experience is the floor of its level",
    "approximated: a fleet on an expedition is judged by its ships when it returns",
    "not imported: `onslot_plus` (the live server does not send it), quests, map progress, expedition records, air bases, presets, album forms, war results",
];

/// How many differing ships the report lists one by one.
const LISTED_SHIPS: usize = 20;

impl fmt::Display for ImportReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "imported: {} ships, {} equipment, {} fleets, {} use items, {} furniture",
            self.ships, self.slot_items, self.fleets, self.use_items, self.furniture
        )?;
        writeln!(
            f,
            "ships whose recalculated stats differ from the live ones: {}",
            self.mismatches.len()
        )?;
        let mut by_field: BTreeMap<&str, usize> = BTreeMap::new();
        for field in self.mismatches.iter().flat_map(|ship| &ship.fields) {
            *by_field.entry(field.field).or_default() += 1;
        }
        for (field, count) in by_field {
            writeln!(f, "  {field}: {count} ships")?;
        }
        for ship in self.mismatches.iter().take(LISTED_SHIPS) {
            let fields: Vec<String> = ship
                .fields
                .iter()
                .map(|x| format!("{} live {} local {}", x.field, x.live, x.local))
                .collect();
            writeln!(f, "  ship {} (mst {}): {}", ship.live_id, ship.mst_id, fields.join(", "))?;
        }
        if self.mismatches.len() > LISTED_SHIPS {
            writeln!(f, "  … and {} more", self.mismatches.len() - LISTED_SHIPS)?;
        }
        for note in NOTES {
            writeln!(f, "{note}")?;
        }
        Ok(())
    }
}

impl Ctx {
    /// Replace a freshly created profile's game data with a live capture, in one transaction.
    pub async fn import_account(
        &self,
        profile_id: i64,
        snapshot: &LiveSnapshot,
    ) -> Result<ImportReport, GameplayError> {
        let tx = self.db.begin().await?;
        let report = import_account_impl(&tx, &self.codex, profile_id, snapshot).await?;
        tx.commit().await?;
        Ok(report)
    }
}

/// Write `snapshot` over the seeded rows of `profile_id`.
///
/// Quests are not observed: the profile's starting quests would otherwise advance on
/// hundreds of ship and equipment inserts.
pub async fn import_account_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    snapshot: &LiveSnapshot,
) -> Result<ImportReport, GameplayError>
where
    C: ConnectionTrait,
{
    let missing = missing_masters(codex, snapshot);
    if !missing.is_empty() {
        return Err(GameplayError::BadManifest(format!(
            "the codex lacks ships {:?} and equipment {:?}",
            missing.ships, missing.slot_items
        )));
    }

    import_profile(c, profile_id, snapshot).await?;
    import_materials(c, profile_id, snapshot).await?;
    let use_items = import_use_items(c, profile_id, snapshot).await?;
    import_furniture(c, profile_id, snapshot).await?;
    import_settings(c, profile_id, snapshot).await?;

    let item_ids = import_slot_items(c, codex, profile_id, snapshot).await?;
    let (ship_ids, mismatches) = import_ships(c, codex, profile_id, snapshot, &item_ids).await?;
    import_fleets(c, profile_id, snapshot, &ship_ids).await?;
    import_docks(c, profile_id, snapshot, &ship_ids).await?;

    Ok(ImportReport {
        ships: ship_ids.len(),
        slot_items: item_ids.len(),
        fleets: snapshot.decks.len(),
        use_items,
        furniture: snapshot.furniture.len(),
        mismatches,
    })
}

async fn import_profile<C: ConnectionTrait>(
    c: &C,
    profile_id: i64,
    snapshot: &LiveSnapshot,
) -> Result<(), GameplayError> {
    let basic = &snapshot.basic;
    let mut am = profile::Entity::find_by_id(profile_id)
        .one(c)
        .await?
        .ok_or(GameplayError::ProfileNotFound(profile_id))?
        .into_active_model();
    am.name = ActiveValue::Set(basic.api_nickname.clone());
    am.last_played = ActiveValue::Set(Utc::now());
    am.hq_level = ActiveValue::Set(basic.api_level);
    am.hq_rank = ActiveValue::Set(basic.api_rank);
    am.experience = ActiveValue::Set(basic.api_experience);
    am.comment = ActiveValue::Set(basic.api_comment.clone());
    am.max_ship_capacity = ActiveValue::Set(basic.api_max_chara);
    am.max_equipment_capacity = ActiveValue::Set(basic.api_max_slotitem);
    am.sortie_wins = ActiveValue::Set(basic.api_st_win);
    am.sortie_loses = ActiveValue::Set(basic.api_st_lose);
    am.expeditions = ActiveValue::Set(basic.api_ms_count);
    am.expeditions_success = ActiveValue::Set(basic.api_ms_success);
    am.practice_battles = ActiveValue::Set(basic.api_pt_win + basic.api_pt_lose);
    am.practice_battle_wins = ActiveValue::Set(basic.api_pt_win);
    am.practice_challenges = ActiveValue::Set(basic.api_pt_challenged);
    am.practice_challenge_wins = ActiveValue::Set(basic.api_pt_challenged_win);
    am.intro_completed = ActiveValue::Set(basic.api_firstflag == 1);
    am.tutorial_progress = ActiveValue::Set(basic.api_tutorial_progress);
    am.medals = ActiveValue::Set(basic.api_medals);
    am.large_dock_unlocked = ActiveValue::Set(basic.api_large_dock == 1);
    am.max_quests = ActiveValue::Set(snapshot.parallel_quest_count);
    am.extra_supply_expedition = ActiveValue::Set(snapshot.extra_supply[0] == 1);
    am.extra_supply_sortie = ActiveValue::Set(snapshot.extra_supply[1] == 1);
    am.update(c).await?;
    Ok(())
}

async fn import_materials<C: ConnectionTrait>(
    c: &C,
    profile_id: i64,
    snapshot: &LiveSnapshot,
) -> Result<(), GameplayError> {
    let mut am = material::Entity::find_by_id(profile_id)
        .one(c)
        .await?
        .ok_or_else(|| GameplayError::EntryNotFound(format!("materials of profile {profile_id}")))?
        .into_active_model();
    let now = Utc::now();
    for element in &snapshot.materials {
        let value = ActiveValue::Set(element.api_value);
        match element.api_id {
            1 => am.fuel = value,
            2 => am.ammo = value,
            3 => am.steel = value,
            4 => am.bauxite = value,
            5 => am.torch = value,
            6 => am.bucket = value,
            7 => am.devmat = value,
            8 => am.screw = value,
            other => return Err(GameplayError::WrongType(format!("material id {other}"))),
        }
    }
    am.last_update_primary = ActiveValue::Set(now);
    am.last_update_bauxite = ActiveValue::Set(now);
    am.update(c).await?;
    Ok(())
}

/// The furniture coin is not in the live `api_useitem`; it comes from `api_basic.api_fcoin`.
async fn import_use_items<C: ConnectionTrait>(
    c: &C,
    profile_id: i64,
    snapshot: &LiveSnapshot,
) -> Result<usize, GameplayError> {
    let fcoin = KcUseItemType::FCoin as i64;
    let mut written = 0;
    for item in snapshot.use_items.iter().filter(|item| item.api_id != fcoin) {
        add_use_item_impl(c, profile_id, item.api_id, item.api_count).await?;
        written += 1;
    }
    add_use_item_impl(c, profile_id, fcoin, snapshot.basic.api_fcoin).await?;
    Ok(written + 1)
}

async fn import_furniture<C: ConnectionTrait>(
    c: &C,
    profile_id: i64,
    snapshot: &LiveSnapshot,
) -> Result<(), GameplayError> {
    // The profile was seeded with the default room's six pieces.
    furniture_record::Entity::delete_many()
        .filter(furniture_record::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;
    for piece in &snapshot.furniture {
        furniture_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            furniture_id: ActiveValue::Set(piece.api_id),
        }
        .insert(c)
        .await?;
    }
    let [floor, wallpaper, window, wall_hanging, shelf, desk] = snapshot.basic.api_furniture;
    let config = FurnitureConfig {
        floor,
        wallpaper,
        window,
        wall_hanging,
        shelf,
        desk,
        ..Default::default()
    };
    update_furniture_config_impl(c, profile_id, &config).await?;
    Ok(())
}

async fn import_settings<C: ConnectionTrait>(
    c: &C,
    profile_id: i64,
    snapshot: &LiveSnapshot,
) -> Result<(), GameplayError> {
    update_oss_settings_impl(
        c,
        profile_id,
        snapshot.oss.api_language_type,
        &snapshot.oss.api_oss_items,
    )
    .await?;

    // A new profile has no option row. The volumes are not in the capture, so they are
    // the ones the client falls back to without a row (`initialize_volume` in main.js).
    let options = KcApiOptionSetting {
        api_skin_id: snapshot.skin_id,
        api_vol_bgm: 30,
        api_vol_se: 40,
        api_vol_voice: 60,
        api_v_be_left: 1,
        api_v_duty: 1,
    };
    update_options_settings_impl(c, profile_id, &options).await?;

    let mut game = game_settings::Entity::find_by_id(profile_id)
        .one(c)
        .await?
        .ok_or_else(|| GameplayError::EntryNotFound(format!("settings of profile {profile_id}")))?
        .into_active_model();
    game.position_id = ActiveValue::Set(snapshot.position_id);
    game.port_bgm = ActiveValue::Set(snapshot.port_bgm);
    game.update(c).await?;
    Ok(())
}

/// Insert the equipment in ascending live id; returns live id → local id.
async fn import_slot_items<C: ConnectionTrait>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    snapshot: &LiveSnapshot,
) -> Result<BTreeMap<i64, i64>, GameplayError> {
    let mut items: Vec<&KcApiSlotItem> = snapshot.slot_items.iter().collect();
    items.sort_by_key(|item| item.api_id);
    let mut ids = BTreeMap::new();
    for item in items {
        let mst = codex.find::<ApiMstSlotitem>(&item.api_slotitem_id)?;
        let alv = item.api_alv.unwrap_or_default();
        let model = slot_item::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            mst_id: ActiveValue::Set(item.api_slotitem_id),
            type3: ActiveValue::Set(mst.api_type[2]),
            locked: ActiveValue::Set(item.api_locked == 1),
            level: ActiveValue::Set(item.api_level),
            aircraft_lv: ActiveValue::Set(alv),
            aircraft_exp: ActiveValue::Set(exp_of_level(alv)),
            equip_on: ActiveValue::Set(0),
        }
        .insert(c)
        .await?;
        add_slot_item_to_picturebook_impl(c, profile_id, mst.api_sortno).await?;
        ids.insert(item.api_id, model.id);
    }
    Ok(ids)
}

/// Map an equipment slot: a positive live id to its local id, `-1` and `0` as they are.
fn map_slot(slot: i64, item_ids: &BTreeMap<i64, i64>, ship_id: i64) -> Result<i64, GameplayError> {
    if slot <= 0 {
        return Ok(slot);
    }
    item_ids.get(&slot).copied().ok_or_else(|| {
        GameplayError::EntryNotFound(format!(
            "ship {ship_id} carries equipment {slot}, which is not in the capture"
        ))
    })
}

/// Insert the ships in ascending live id with their stats recalculated locally;
/// returns live id → local id, and the ships whose stats came out different.
async fn import_ships<C: ConnectionTrait>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    snapshot: &LiveSnapshot,
    item_ids: &BTreeMap<i64, i64>,
) -> Result<(BTreeMap<i64, i64>, Vec<ShipMismatch>), GameplayError> {
    let items_by_id: BTreeMap<i64, &KcApiSlotItem> =
        snapshot.slot_items.iter().map(|item| (item.api_id, item)).collect();
    let mut ships: Vec<&KcApiShip> = snapshot.ships.iter().collect();
    ships.sort_by_key(|ship| ship.api_id);

    let mut ids = BTreeMap::new();
    let mut mismatches = Vec::new();
    for live in ships {
        let married = live.api_lv >= 100;
        // `cal_ship_status` works in place and looks equipment up by id, so it runs on a
        // copy that still carries the live item ids; `live` stays for the comparison.
        let mut local = live.clone();
        let carried: Vec<KcApiSlotItem> = live
            .api_slot
            .iter()
            .chain([&live.api_slot_ex])
            .filter_map(|id| items_by_id.get(id).map(|item| (*item).clone()))
            .collect();
        codex.cal_ship_status(&mut local, &carried, married).map_err(|e| {
            GameplayError::BadManifest(format!(
                "ship {} (mst {}): {e}",
                live.api_id, live.api_ship_id
            ))
        })?;
        let fields = compare_stats(live, &local);
        if !fields.is_empty() {
            mismatches.push(ShipMismatch {
                live_id: live.api_id,
                mst_id: live.api_ship_id,
                fields,
            });
        }

        for slot in local.api_slot.iter_mut() {
            *slot = map_slot(*slot, item_ids, live.api_id)?;
        }
        local.api_slot_ex = map_slot(local.api_slot_ex, item_ids, live.api_id)?;

        let mut am: ship::ActiveModel = local.clone().into();
        am.id = ActiveValue::NotSet;
        am.profile_id = ActiveValue::Set(profile_id);
        am.married = ActiveValue::Set(married);
        let model = am.insert(c).await?;
        ids.insert(live.api_id, model.id);

        for item_id in local.api_slot.iter().chain([&local.api_slot_ex]).filter(|id| **id > 0) {
            slot_item::ActiveModel {
                id: ActiveValue::Unchanged(*item_id),
                equip_on: ActiveValue::Set(model.id),
                ..Default::default()
            }
            .update(c)
            .await?;
        }
        for (index, sp) in live.api_sp_effect_items.iter().flatten().enumerate() {
            sp_effect_item::ActiveModel {
                id: ActiveValue::NotSet,
                profile_id: ActiveValue::Set(profile_id),
                ship_id: ActiveValue::Set(model.id),
                index: ActiveValue::Set(index as i64),
                kind: ActiveValue::Set(sp.api_kind),
                houg: ActiveValue::Set(sp.api_houg.unwrap_or_default()),
                raig: ActiveValue::Set(sp.api_raig.unwrap_or_default()),
                kaih: ActiveValue::Set(sp.api_kaih.unwrap_or_default()),
                souk: ActiveValue::Set(sp.api_souk.unwrap_or_default()),
            }
            .insert(c)
            .await?;
        }
        // Several ships share an album entry: only ever set `married`, never clear it.
        add_ship_to_picturebook_impl(c, profile_id, live.api_sortno, None, married.then_some(true))
            .await?;
    }
    Ok((ids, mismatches))
}

/// The stats the local formulas derive, live value against local value.
fn compare_stats(live: &KcApiShip, local: &KcApiShip) -> Vec<FieldMismatch> {
    let pairs = [
        ("api_karyoku[0]", live.api_karyoku[0], local.api_karyoku[0]),
        ("api_karyoku[1]", live.api_karyoku[1], local.api_karyoku[1]),
        ("api_raisou[0]", live.api_raisou[0], local.api_raisou[0]),
        ("api_raisou[1]", live.api_raisou[1], local.api_raisou[1]),
        ("api_taiku[0]", live.api_taiku[0], local.api_taiku[0]),
        ("api_taiku[1]", live.api_taiku[1], local.api_taiku[1]),
        ("api_soukou[0]", live.api_soukou[0], local.api_soukou[0]),
        ("api_soukou[1]", live.api_soukou[1], local.api_soukou[1]),
        ("api_kaihi[0]", live.api_kaihi[0], local.api_kaihi[0]),
        ("api_kaihi[1]", live.api_kaihi[1], local.api_kaihi[1]),
        ("api_taisen[0]", live.api_taisen[0], local.api_taisen[0]),
        ("api_taisen[1]", live.api_taisen[1], local.api_taisen[1]),
        ("api_sakuteki[0]", live.api_sakuteki[0], local.api_sakuteki[0]),
        ("api_sakuteki[1]", live.api_sakuteki[1], local.api_sakuteki[1]),
        ("api_lucky[0]", live.api_lucky[0], local.api_lucky[0]),
        ("api_lucky[1]", live.api_lucky[1], local.api_lucky[1]),
        ("api_maxhp", live.api_maxhp, local.api_maxhp),
        ("api_soku", live.api_soku, local.api_soku),
        ("api_leng", live.api_leng, local.api_leng),
        ("api_exp[1]", live.api_exp[1], local.api_exp[1]),
        ("api_ndock_time", live.api_ndock_time, local.api_ndock_time),
        ("api_ndock_item[0]", live.api_ndock_item[0], local.api_ndock_item[0]),
        ("api_ndock_item[1]", live.api_ndock_item[1], local.api_ndock_item[1]),
        ("api_srate", live.api_srate, local.api_srate),
        ("api_locked_equip", live.api_locked_equip, local.api_locked_equip),
    ];
    pairs
        .into_iter()
        .filter(|(_, live, local)| live != local)
        .map(|(field, live, local)| FieldMismatch {
            field,
            live,
            local,
        })
        .collect()
}

fn time_of_millis(millis: i64) -> Option<DateTime<Utc>> {
    (millis > 0).then(|| DateTime::from_timestamp_millis(millis)).flatten()
}

fn map_ship(id: i64, ship_ids: &BTreeMap<i64, i64>) -> Result<i64, GameplayError> {
    if id <= 0 {
        return Ok(id);
    }
    ship_ids
        .get(&id)
        .copied()
        .ok_or_else(|| GameplayError::EntryNotFound(format!("ship {id} is not in the capture")))
}

async fn import_fleets<C: ConnectionTrait>(
    c: &C,
    profile_id: i64,
    snapshot: &LiveSnapshot,
    ship_ids: &BTreeMap<i64, i64>,
) -> Result<(), GameplayError> {
    for deck in &snapshot.decks {
        let existing = fleet::Entity::find()
            .filter(fleet::Column::ProfileId.eq(profile_id))
            .filter(fleet::Column::Index.eq(deck.api_id))
            .one(c)
            .await?;
        let model = match existing {
            Some(model) => model,
            None => unlock_fleet_impl(c, profile_id, deck.api_id).await?,
        };
        let [status, mission_id, return_time, _] = deck.api_mission;
        let mut am = model.into_active_model();
        am.name = ActiveValue::Set(deck.api_name.clone());
        am.mission_status = ActiveValue::Set(match status {
            0 => fleet::MissionStatus::Idle,
            1 => fleet::MissionStatus::InMission,
            2 => fleet::MissionStatus::Returning,
            3 => fleet::MissionStatus::ForceReturning,
            other => {
                return Err(GameplayError::WrongType(format!(
                    "fleet {} has mission status {other}",
                    deck.api_id
                )));
            }
        });
        am.mission_id = ActiveValue::Set(mission_id);
        am.return_time = ActiveValue::Set(time_of_millis(return_time));
        let [s1, s2, s3, s4, s5, s6] = deck.api_ship;
        am.ship_1 = ActiveValue::Set(map_ship(s1, ship_ids)?);
        am.ship_2 = ActiveValue::Set(map_ship(s2, ship_ids)?);
        am.ship_3 = ActiveValue::Set(map_ship(s3, ship_ids)?);
        am.ship_4 = ActiveValue::Set(map_ship(s4, ship_ids)?);
        am.ship_5 = ActiveValue::Set(map_ship(s5, ship_ids)?);
        am.ship_6 = ActiveValue::Set(map_ship(s6, ship_ids)?);
        am.update(c).await?;
    }
    Ok(())
}

async fn import_docks<C: ConnectionTrait>(
    c: &C,
    profile_id: i64,
    snapshot: &LiveSnapshot,
    ship_ids: &BTreeMap<i64, i64>,
) -> Result<(), GameplayError> {
    for dock in &snapshot.ndocks {
        let mut am = ndock::Entity::find()
            .filter(ndock::Column::ProfileId.eq(profile_id))
            .filter(ndock::Column::Index.eq(dock.api_id))
            .one(c)
            .await?
            .ok_or_else(|| GameplayError::EntryNotFound(format!("repair dock {}", dock.api_id)))?
            .into_active_model();
        am.status = ActiveValue::Set(match dock.api_state {
            -1 => ndock::Status::Locked,
            0 => ndock::Status::Idle,
            1 => ndock::Status::Busy,
            other => return Err(GameplayError::WrongType(format!("repair dock state {other}"))),
        });
        am.ship_id = ActiveValue::Set(map_ship(dock.api_ship_id, ship_ids)?);
        am.complete_time = ActiveValue::Set(time_of_millis(dock.api_complete_time));
        am.fuel = ActiveValue::Set(dock.api_item1);
        am.steel = ActiveValue::Set(dock.api_item3);
        am.update(c).await?;
    }
    for dock in &snapshot.kdocks {
        let mut am = kdock::Entity::find()
            .filter(kdock::Column::ProfileId.eq(profile_id))
            .filter(kdock::Column::Index.eq(dock.api_id))
            .one(c)
            .await?
            .ok_or_else(|| {
                GameplayError::EntryNotFound(format!("construction dock {}", dock.api_id))
            })?
            .into_active_model();
        am.status = ActiveValue::Set(match dock.api_state {
            -1 => kdock::Status::Locked,
            0 => kdock::Status::Idle,
            2 => kdock::Status::Busy,
            3 => kdock::Status::Completed,
            other => {
                return Err(GameplayError::WrongType(format!("construction dock state {other}")));
            }
        });
        // `kdock.ship_id` holds the master id of the ship being built, not an instance id.
        am.ship_id = ActiveValue::Set(dock.api_created_ship_id);
        am.complete_time = ActiveValue::Set(time_of_millis(dock.api_complete_time));
        // Normal construction takes at most 999 of each material, large at least 1500 fuel.
        am.is_large = ActiveValue::Set(dock.api_item1 > 999);
        am.fuel = ActiveValue::Set(dock.api_item1);
        am.ammo = ActiveValue::Set(dock.api_item2);
        am.steel = ActiveValue::Set(dock.api_item3);
        am.bauxite = ActiveValue::Set(dock.api_item4);
        am.devmat = ActiveValue::Set(dock.api_item5);
        am.update(c).await?;
    }
    Ok(())
}
