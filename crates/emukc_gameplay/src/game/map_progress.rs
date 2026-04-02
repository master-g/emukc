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
