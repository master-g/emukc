use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use async_trait::async_trait;
use emukc_db::{
    entity::profile::map_record,
    sea_orm::{ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait, entity::prelude::*},
};
#[cfg(test)]
use emukc_model::codex::map::MapStageDefinition;
use emukc_model::{
    codex::{
        Codex,
        map::{MapCatalog, MapDefinition, MapResetPolicy},
    },
    kc2::{KcApiEventmap, KcApiMapInfo},
    profile::map_record::MapSelectRank,
};
use emukc_time::{KcTime, chrono::Utc};

use crate::{err::GameplayError, gameplay::HasContext};

use super::{
    basic::find_profile,
    fleet::get_fleet_ships_impl,
    map_progress::{active_stage_for_record, assign_stage_id, select_stage_id_for_rank},
};

#[derive(Debug, Clone)]
pub struct EventMapRankSelection {
    pub now_maphp: i64,
    pub max_maphp: i64,
    pub gauge_type: i64,
    pub gauge_num: i64,
    pub sally_flag: [i64; 3],
}

/// A trait for map related gameplay.
#[async_trait]
pub trait MapOps {
    /// Get map records of a profile.
    async fn get_map_records(
        &self,
        profile_id: i64,
    ) -> Result<Vec<map_record::Model>, GameplayError>;

    /// Get map info view for KCS API.
    async fn get_map_infos(&self, profile_id: i64) -> Result<Vec<KcApiMapInfo>, GameplayError>;

    /// Select event map rank.
    async fn select_eventmap_rank(
        &self,
        profile_id: i64,
        maparea_id: i64,
        mapinfo_no: i64,
        rank: i64,
    ) -> Result<EventMapRankSelection, GameplayError>;

    /// Get current combined fleet type.
    async fn get_combined_type(&self, profile_id: i64) -> Result<i64, GameplayError>;

    /// Set current combined fleet type.
    async fn set_combined_type(
        &self,
        profile_id: i64,
        combined_type: i64,
    ) -> Result<i64, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> MapOps for T {
    async fn get_map_records(
        &self,
        profile_id: i64,
    ) -> Result<Vec<map_record::Model>, GameplayError> {
        let codex = self.codex();
        let db = self.db();
        let tx = db.begin().await?;

        ensure_map_records_impl(&tx, codex, profile_id).await?;
        refresh_all_map_records_impl(&tx, codex, profile_id).await?;
        let records = get_map_records_impl(&tx, profile_id).await?;

        tx.commit().await?;
        Ok(records)
    }

    async fn get_map_infos(&self, profile_id: i64) -> Result<Vec<KcApiMapInfo>, GameplayError> {
        let codex = self.codex();
        let db = self.db();
        let tx = db.begin().await?;

        ensure_map_records_impl(&tx, codex, profile_id).await?;
        refresh_all_map_records_impl(&tx, codex, profile_id).await?;
        let records = get_map_records_impl(&tx, profile_id).await?;
        let infos = build_map_infos(codex, records);

        tx.commit().await?;
        Ok(infos)
    }

    async fn select_eventmap_rank(
        &self,
        profile_id: i64,
        maparea_id: i64,
        mapinfo_no: i64,
        rank: i64,
    ) -> Result<EventMapRankSelection, GameplayError> {
        let codex = self.codex();
        let db = self.db();
        let tx = db.begin().await?;

        ensure_map_records_impl(&tx, codex, profile_id).await?;
        refresh_all_map_records_impl(&tx, codex, profile_id).await?;
        let definition = find_map_definition(codex, maparea_id, mapinfo_no)?;
        if !definition.is_event {
            return Err(GameplayError::WrongType(format!(
                "map {maparea_id}-{mapinfo_no} is not an event map",
            )));
        }

        let selected_rank = map_record::SelectedRank::from(parse_map_select_rank(rank)?);
        let record = find_map_record_impl(&tx, profile_id, definition.map_id).await?;
        let current_hp = record.current_hp;
        let mut am = record.into_active_model();
        am.selected_rank = ActiveValue::Set(selected_rank);
        assign_stage_id(&mut am, select_stage_id_for_rank(&definition, rank));
        if definition.max_hp.is_some() && current_hp.is_none() {
            am.current_hp = ActiveValue::Set(definition.max_hp);
        }
        am.gauge_index = ActiveValue::Set(1);
        am.event_state = ActiveValue::Set(Some(1));
        let updated = am.update(&tx).await?;

        tx.commit().await?;

        Ok(EventMapRankSelection {
            now_maphp: updated.current_hp.unwrap_or(definition.max_hp.unwrap_or(0)),
            max_maphp: definition.max_hp.unwrap_or(updated.current_hp.unwrap_or(0)),
            gauge_type: definition.gauge_type.unwrap_or(2),
            gauge_num: definition.gauge_count.unwrap_or(updated.gauge_index),
            sally_flag: sally_flag_array(&definition),
        })
    }

    async fn get_combined_type(&self, profile_id: i64) -> Result<i64, GameplayError> {
        let db = self.db();
        let profile = find_profile(db, profile_id).await?;
        Ok(profile.combined_type)
    }

    async fn set_combined_type(
        &self,
        profile_id: i64,
        combined_type: i64,
    ) -> Result<i64, GameplayError> {
        if !(0..=3).contains(&combined_type) {
            return Err(GameplayError::WrongType(
                format!("invalid combined_type {combined_type}",),
            ));
        }

        let db = self.db();
        let tx = db.begin().await?;

        find_profile(&tx, profile_id).await?;
        if combined_type > 0 {
            let fleet1 = get_fleet_ships_impl(&tx, profile_id, 1).await?;
            let fleet2 = get_fleet_ships_impl(&tx, profile_id, 2).await?;
            if fleet1.is_empty() || fleet2.is_empty() {
                return Err(GameplayError::WrongType(
                    "combined fleet requires both deck 1 and deck 2 to have ships".to_string(),
                ));
            }
        }

        let profile = find_profile(&tx, profile_id).await?;
        let mut am = profile.into_active_model();
        am.combined_type = ActiveValue::Set(combined_type);
        am.update(&tx).await?;

        tx.commit().await?;
        Ok(i64::from(combined_type > 0))
    }
}

pub(crate) async fn get_map_records_impl<C>(
    c: &C,
    profile_id: i64,
) -> Result<Vec<map_record::Model>, GameplayError>
where
    C: ConnectionTrait,
{
    let records = map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .order_by_asc(map_record::Column::MapId)
        .all(c)
        .await?;

    Ok(records)
}

pub(crate) async fn find_map_record_impl<C>(
    c: &C,
    profile_id: i64,
    map_id: i64,
) -> Result<map_record::Model, GameplayError>
where
    C: ConnectionTrait,
{
    map_record::Entity::find()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .filter(map_record::Column::MapId.eq(map_id))
        .one(c)
        .await?
        .ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "map_record for map {map_id} not found for profile {profile_id}",
            ))
        })
}

pub(crate) async fn ensure_map_records_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let existing = get_map_records_impl(c, profile_id)
        .await?
        .into_iter()
        .map(|record| record.map_id)
        .collect::<BTreeSet<_>>();
    let now = Utc::now();
    let catalog = active_map_catalog(codex);

    for definition in catalog.known_maps() {
        if existing.contains(&definition.map_id) {
            continue;
        }

        let unlocked = is_map_unlocked_by_default(codex, definition.map_id, &existing);

        map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile_id),
            map_id: ActiveValue::Set(definition.map_id),
            cleared: ActiveValue::Set(false),
            unlocked: ActiveValue::Set(unlocked),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(Some(now)),
            defeat_count: ActiveValue::Set(definition.required_defeat_count.map(|_| 0)),
            current_hp: ActiveValue::Set(definition.max_hp),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(select_stage_id_for_rank(definition, 0)),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(definition.max_hp.map(|_| 1)),
        }
        .insert(c)
        .await?;
    }

    Ok(())
}

pub(crate) async fn refresh_all_map_records_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let now = Utc::now();
    let refresh_boundary = KcTime::jst_0500_of_nth_day(1);
    let definitions = active_map_catalog(codex)
        .known_maps()
        .into_iter()
        .map(|definition| (definition.map_id, definition.clone()))
        .collect::<BTreeMap<_, _>>();

    for record in get_map_records_impl(c, profile_id).await? {
        let Some(definition) = definitions.get(&record.map_id) else {
            continue;
        };
        let selected_rank = record.selected_rank.clone() as i64;
        if definition.reset_policy == MapResetPolicy::Monthly
            && now >= refresh_boundary
            && record.last_reset_at.is_none_or(|ts| ts < refresh_boundary)
        {
            let mut am = record.into_active_model();
            am.cleared = ActiveValue::Set(false);
            am.last_cleared_at = ActiveValue::Set(None);
            am.last_reset_at = ActiveValue::Set(Some(now));
            am.defeat_count = ActiveValue::Set(definition.required_defeat_count.map(|_| 0));
            am.current_hp = ActiveValue::Set(definition.max_hp);
            am.gauge_index = ActiveValue::Set(1);
            assign_stage_id(&mut am, select_stage_id_for_rank(definition, selected_rank));
            am.event_state = ActiveValue::Set(definition.max_hp.map(|_| 1));
            am.update(c).await?;
        }
    }

    Ok(())
}

pub(crate) fn find_map_definition(
    codex: &Codex,
    maparea_id: i64,
    mapinfo_no: i64,
) -> Result<MapDefinition, GameplayError> {
    active_map_catalog(codex)
        .as_ref()
        .map_definition_by_area_no(maparea_id, mapinfo_no)
        .cloned()
        .ok_or_else(|| {
            GameplayError::EntryNotFound(format!(
                "map definition not found for {maparea_id}-{mapinfo_no}",
            ))
        })
}

pub(crate) fn active_map_catalog(codex: &Codex) -> Cow<'_, MapCatalog> {
    codex.map_catalog()
}

pub(crate) fn build_map_infos(codex: &Codex, records: Vec<map_record::Model>) -> Vec<KcApiMapInfo> {
    let record_map =
        records.into_iter().map(|record| (record.map_id, record)).collect::<BTreeMap<_, _>>();
    let manifest_ids =
        codex.manifest.api_mst_mapinfo.iter().map(|map| map.api_id).collect::<BTreeSet<_>>();
    let catalog = active_map_catalog(codex);

    let mut infos = catalog
        .as_ref()
        .known_maps()
        .into_iter()
        .filter(|definition| manifest_ids.contains(&definition.map_id))
        .collect::<Vec<_>>();
    infos.sort_by_key(|definition| definition.map_id);

    infos
        .into_iter()
        .filter_map(|definition| {
            record_map
                .get(&definition.map_id)
                .filter(|record| record.unlocked)
                .map(|record| build_map_info(definition, record))
        })
        .collect()
}

fn build_map_info(definition: &MapDefinition, record: &map_record::Model) -> KcApiMapInfo {
    let active_stage = active_stage_for_record(definition, record);
    let required_defeat_count = active_stage
        .and_then(|stage| stage.required_defeat_count)
        .or(definition.required_defeat_count);
    let mut info = KcApiMapInfo {
        api_id: definition.map_id,
        api_cleared: record.cleared as i64,
        api_defeat_count: required_defeat_count.map(|_| record.defeat_count.unwrap_or(0)),
        api_gauge_num: definition.gauge_count.or_else(|| required_defeat_count.map(|_| 1)),
        api_gauge_type: definition.gauge_type,
        api_required_defeat_count: required_defeat_count,
        api_air_base_decks: definition.airbase_count,
        api_eventmap: None,
        api_s_no: None,
        api_m10: None,
        api_sally_flag: None,
    };

    if definition.is_event
        || definition.max_hp.is_some()
        || record.selected_rank != map_record::SelectedRank::NotSet
    {
        info.api_eventmap = Some(KcApiEventmap {
            api_now_maphp: record.current_hp.unwrap_or(definition.max_hp.unwrap_or(0)),
            api_max_maphp: definition.max_hp.unwrap_or(record.current_hp.unwrap_or(0)),
            api_selected_rank: record.selected_rank.clone() as i64,
            api_state: record.event_state.unwrap_or(1),
        });
        info.api_sally_flag = Some(sally_flag_array(definition));
        info.api_s_no = Some(definition.mapinfo_no);
        info.api_m10 = Some(0);
    }

    info
}

fn parse_map_select_rank(rank: i64) -> Result<MapSelectRank, GameplayError> {
    MapSelectRank::n(rank as i32)
        .ok_or_else(|| GameplayError::WrongType(format!("invalid map rank {rank}")))
}

pub(crate) async fn check_and_unlock_dependencies_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    cleared_map_id: i64,
) -> Result<Vec<i64>, GameplayError>
where
    C: ConnectionTrait,
{
    let catalog = active_map_catalog(codex);
    let dependents = catalog.dependents_of(cleared_map_id);
    if dependents.is_empty() {
        return Ok(vec![]);
    }

    let mut newly_unlocked = Vec::new();
    for dep_id in dependents {
        let record = find_map_record_impl(c, profile_id, dep_id).await?;
        if !record.unlocked {
            let mut am = record.into_active_model();
            am.unlocked = ActiveValue::Set(true);
            am.update(c).await?;
            newly_unlocked.push(dep_id);
        }
    }

    Ok(newly_unlocked)
}

pub(crate) async fn is_map_unlocked_impl<C>(
    c: &C,
    profile_id: i64,
    map_id: i64,
) -> Result<bool, GameplayError>
where
    C: ConnectionTrait,
{
    let record = find_map_record_impl(c, profile_id, map_id).await?;
    Ok(record.unlocked)
}

fn is_map_unlocked_by_default(codex: &Codex, map_id: i64, existing: &BTreeSet<i64>) -> bool {
    let catalog = active_map_catalog(codex);
    match catalog.prerequisite_for(map_id) {
        // Maps with a prerequisite: unlocked only if prerequisite record already exists
        Some(prereq_id) => existing.contains(&prereq_id),
        // Maps without a prerequisite entry:
        // - Regular maps (areas 1-7): only 1-1 (map_id=11) starts unlocked
        // - Event maps, test maps, and maps outside the regular area range: unlocked by default
        None => {
            let (area, _) = emukc_model::codex::map::split_map_id(map_id);
            if (1..=7).contains(&area) {
                map_id == 11
            } else {
                true
            }
        }
    }
}

fn sally_flag_array(definition: &MapDefinition) -> [i64; 3] {
    let mut result = [0; 3];
    for (idx, value) in definition.sally_flag.iter().copied().take(3).enumerate() {
        result[idx] = value;
    }
    result
}

pub(super) async fn init<C>(c: &C, codex: &Codex, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    ensure_map_records_impl(c, codex, profile_id).await
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    map_record::Entity::delete_many()
        .filter(map_record::Column::ProfileId.eq(profile_id))
        .exec(c)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_definition() -> MapDefinition {
        MapDefinition {
            map_id: 1,
            maparea_id: 1,
            mapinfo_no: 1,
            name: "test".to_string(),
            level: 1,
            sally_flag: vec![],
            is_event: false,
            reset_policy: MapResetPolicy::Never,
            airbase_count: None,
            gauge_type: None,
            gauge_count: None,
            required_defeat_count: None,
            max_hp: None,
            default_variant: "legacy".to_string(),
            rank_stage_ids: BTreeMap::from([(4, "hard".to_string())]),
            variants: BTreeMap::from([
                (
                    "legacy".to_string(),
                    MapStageDefinition {
                        variant_key: "legacy".to_string(),
                        ..Default::default()
                    },
                ),
                (
                    "current".to_string(),
                    MapStageDefinition {
                        variant_key: "current".to_string(),
                        ..Default::default()
                    },
                ),
                (
                    "hard".to_string(),
                    MapStageDefinition {
                        variant_key: "hard".to_string(),
                        ..Default::default()
                    },
                ),
            ]),
        }
    }

    fn sample_record(stage_id: Option<&str>) -> map_record::Model {
        map_record::Model {
            id: 1,
            profile_id: 1,
            map_id: 1,
            cleared: false,
            unlocked: true,
            last_cleared_at: None,
            last_reset_at: None,
            defeat_count: None,
            current_hp: None,
            gauge_index: 1,
            stage_id: stage_id.map(ToOwned::to_owned),
            selected_rank: map_record::SelectedRank::NotSet,
            event_state: None,
        }
    }

    #[test]
    fn active_stage_for_record_uses_stage_id_when_present() {
        let definition = sample_definition();
        let record = sample_record(Some("current"));

        let stage = active_stage_for_record(&definition, &record).unwrap();

        assert_eq!(stage.variant_key, "current");
    }

    #[test]
    fn active_stage_for_record_falls_back_to_default_stage() {
        let definition = sample_definition();
        let record = sample_record(None);

        let stage = active_stage_for_record(&definition, &record).unwrap();

        assert_eq!(stage.variant_key, "legacy");
    }

    #[test]
    fn select_stage_id_for_rank_prefers_rank_specific_mapping() {
        let definition = sample_definition();

        assert_eq!(select_stage_id_for_rank(&definition, 4).as_deref(), Some("hard"));
        assert_eq!(select_stage_id_for_rank(&definition, 1).as_deref(), Some("legacy"));
    }

    fn load_codex() -> Codex {
        Codex::load_without_cache_source("../../.data/codex").unwrap()
    }

    #[test]
    fn build_regular_prerequisites_returns_correct_chain() {
        let codex = load_codex();
        let catalog = codex.map_catalog();

        // 1-1 has no prerequisite
        assert_eq!(catalog.prerequisite_for(11), None);

        // Same-area sequential: 1-2 requires 1-1, 1-3 requires 1-2, 1-4 requires 1-3
        assert_eq!(catalog.prerequisite_for(12), Some(11));
        assert_eq!(catalog.prerequisite_for(13), Some(12));
        assert_eq!(catalog.prerequisite_for(14), Some(13));

        // Cross-area: 2-1 requires 1-4
        assert_eq!(catalog.prerequisite_for(21), Some(14));
        assert_eq!(catalog.prerequisite_for(24), Some(23));

        // 3-1 requires 2-4
        assert_eq!(catalog.prerequisite_for(31), Some(24));

        // 7-1 through 7-3
        assert_eq!(catalog.prerequisite_for(71), Some(64));
        assert_eq!(catalog.prerequisite_for(72), Some(71));
        assert_eq!(catalog.prerequisite_for(73), Some(72));
    }

    #[test]
    fn dependents_of_map_11_returns_12() {
        let codex = load_codex();
        let catalog = codex.map_catalog();

        let mut deps = catalog.dependents_of(11);
        deps.sort();
        assert_eq!(deps, vec![12]);
    }

    #[test]
    fn dependents_of_map_14_returns_15_and_21() {
        let codex = load_codex();
        let catalog = codex.map_catalog();

        let mut deps = catalog.dependents_of(14);
        deps.sort();
        // 1-5 (EO) and 2-1 both depend on 1-4
        assert_eq!(deps, vec![15, 21]);
    }

    #[test]
    fn is_map_unlocked_by_default_true_for_no_prerequisite() {
        let codex = load_codex();
        let existing = BTreeSet::new();

        assert!(is_map_unlocked_by_default(&codex, 11, &existing));
    }

    #[test]
    fn is_map_unlocked_by_default_false_when_prerequisite_missing() {
        let codex = load_codex();
        let existing = BTreeSet::new();

        assert!(!is_map_unlocked_by_default(&codex, 12, &existing));
    }

    #[test]
    fn is_map_unlocked_by_default_true_when_prerequisite_exists() {
        let codex = load_codex();
        let existing = BTreeSet::from([11]);

        assert!(is_map_unlocked_by_default(&codex, 12, &existing));
    }
}
