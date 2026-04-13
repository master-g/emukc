use std::collections::BTreeSet;

use emukc_model::codex::map::MapDefinition;

use super::capture::CapturedMapStart;

pub(super) fn choose_stage_match(
    definition: &MapDefinition,
    capture: &CapturedMapStart,
) -> Result<String, String> {
    let captured = capture.cells.iter().map(|cell| cell.cell_no).collect::<BTreeSet<_>>();
    let exact_matches = definition
        .variants
        .iter()
        .filter(|(_, stage)| {
            stage.cells.iter().map(|cell| cell.cell_no).collect::<BTreeSet<_>>() == captured
        })
        .map(|(stage_id, _)| stage_id.clone())
        .collect::<Vec<_>>();

    match exact_matches.len() {
        1 => return Ok(exact_matches[0].clone()),
        2.. => return Err(format!("ambiguous_stage_match:{}", exact_matches.join(","))),
        _ => {}
    }

    if definition.variants.len() == 1 {
        return definition
            .variants
            .keys()
            .next()
            .cloned()
            .ok_or_else(|| "no_matching_stage".to_string());
    }

    let superset_matches = definition
        .variants
        .iter()
        .filter_map(|(stage_id, stage)| {
            let stage_cells = stage.cells.iter().map(|cell| cell.cell_no).collect::<BTreeSet<_>>();
            stage_cells.is_superset(&captured).then_some((stage_id.clone(), stage_cells.len()))
        })
        .collect::<Vec<_>>();
    let min_superset = superset_matches.iter().map(|(_, len)| *len).min().unwrap_or(usize::MAX);
    let best_superset = superset_matches
        .into_iter()
        .filter(|(_, len)| *len == min_superset)
        .map(|(stage_id, _)| stage_id)
        .collect::<Vec<_>>();
    match best_superset.len() {
        1 => return Ok(best_superset[0].clone()),
        0 => {}
        _ => {
            if let Some(default_stage_id) = definition.default_stage_id()
                && best_superset.iter().any(|stage_id| stage_id == default_stage_id)
            {
                return Ok(default_stage_id.to_string());
            }
            return Err(format!("ambiguous_stage_match:{}", best_superset.join(",")));
        }
    }

    let subset_matches = definition
        .variants
        .iter()
        .filter_map(|(stage_id, stage)| {
            let stage_cells = stage.cells.iter().map(|cell| cell.cell_no).collect::<BTreeSet<_>>();
            stage_cells.is_subset(&captured).then_some((stage_id.clone(), stage_cells.len()))
        })
        .collect::<Vec<_>>();
    let max_subset = subset_matches.iter().map(|(_, len)| *len).max().unwrap_or(0);
    let best_subset = subset_matches
        .into_iter()
        .filter(|(_, len)| *len == max_subset && *len > 0)
        .map(|(stage_id, _)| stage_id)
        .collect::<Vec<_>>();

    match best_subset.len() {
        1 => Ok(best_subset[0].clone()),
        0 => Err("no_matching_stage".to_string()),
        _ => choose_clear_transition_subset_match(definition, &captured, &best_subset)
            .ok_or_else(|| format!("ambiguous_stage_match:{}", best_subset.join(","))),
    }
}

fn choose_clear_transition_subset_match(
    definition: &MapDefinition,
    captured: &BTreeSet<i64>,
    candidates: &[String],
) -> Option<String> {
    let candidate_ids = candidates.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let transition_targets = definition
        .variants
        .iter()
        .filter_map(|(stage_id, stage)| {
            let target = stage.clear_to_variant_key.as_deref()?;
            if !candidate_ids.contains(stage_id.as_str()) || !candidate_ids.contains(target) {
                return None;
            }

            let stage_cells = stage.cells.iter().map(|cell| cell.cell_no).collect::<BTreeSet<_>>();
            (captured.is_superset(&stage_cells) && captured.len() > stage_cells.len())
                .then_some(target.to_string())
        })
        .collect::<BTreeSet<_>>();

    (transition_targets.len() == 1).then(|| transition_targets.into_iter().next().unwrap())
}
