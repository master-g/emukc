use emukc_db::{entity::profile::map_record, sea_orm::ActiveValue};
use emukc_model::codex::map::{MapDefinition, MapStageDefinition};

pub(crate) fn resolve_record_stage_id(
    definition: &MapDefinition,
    record: &map_record::Model,
) -> Option<String> {
    record
        .stage_id
        .as_deref()
        .filter(|stage_id| definition.stage(stage_id).is_some())
        .map(ToOwned::to_owned)
        .or_else(|| definition.default_stage_id().map(ToOwned::to_owned))
}

pub(crate) fn active_stage_for_record<'a>(
    definition: &'a MapDefinition,
    record: &map_record::Model,
) -> Option<&'a MapStageDefinition> {
    record
        .stage_id
        .as_deref()
        .and_then(|stage_id| definition.stage(stage_id))
        .or_else(|| definition.active_stage(None))
}

pub(crate) fn select_stage_id_for_rank(definition: &MapDefinition, rank: i64) -> Option<String> {
    definition.resolve_stage_id_for_rank(rank).map(ToOwned::to_owned)
}

pub(crate) fn assign_stage_id(record: &mut map_record::ActiveModel, stage_id: Option<String>) {
    record.stage_id = ActiveValue::Set(stage_id);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use emukc_model::codex::map::{MapDefinition, MapResetPolicy, MapStageDefinition};

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
    fn resolve_record_stage_id_returns_stage_id_when_valid() {
        let definition = sample_definition();
        let record = sample_record(Some("current"));
        assert_eq!(resolve_record_stage_id(&definition, &record), Some("current".to_string()));
    }

    #[test]
    fn resolve_record_stage_id_returns_stage_id_even_when_not_a_variant_key() {
        let definition = sample_definition();
        let record = sample_record(Some("nonexistent"));
        assert_eq!(resolve_record_stage_id(&definition, &record), Some("nonexistent".to_string()));
    }

    #[test]
    fn resolve_record_stage_id_falls_back_to_default_when_stage_id_none() {
        let definition = sample_definition();
        let record = sample_record(None);
        assert_eq!(resolve_record_stage_id(&definition, &record), Some("legacy".to_string()));
    }

    #[test]
    fn assign_stage_id_sets_value() {
        let mut record = map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(1),
            map_id: ActiveValue::Set(1),
            cleared: ActiveValue::Set(false),
            unlocked: ActiveValue::Set(true),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(None),
            defeat_count: ActiveValue::Set(None),
            current_hp: ActiveValue::Set(None),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(None),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
        };
        assign_stage_id(&mut record, Some("hard".to_string()));
        assert_eq!(record.stage_id, ActiveValue::Set(Some("hard".to_string())));
    }

    #[test]
    fn assign_stage_id_clears_to_none() {
        let mut record = map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(1),
            map_id: ActiveValue::Set(1),
            cleared: ActiveValue::Set(false),
            unlocked: ActiveValue::Set(true),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(None),
            defeat_count: ActiveValue::Set(None),
            current_hp: ActiveValue::Set(None),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(Some("current".to_string())),
            selected_rank: ActiveValue::Set(map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
        };
        assign_stage_id(&mut record, None);
        assert_eq!(record.stage_id, ActiveValue::Set(None));
    }
}
