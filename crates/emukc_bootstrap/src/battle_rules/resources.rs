//! Resources a battle response makes the client request, and whether
//! `make_list` generates them.
//!
//! Derivation (ship artwork, slot item textures, the display equipment behind
//! `api_si_list`) plus the R4 coverage check. Split out of `battle_rules`
//! because a derived path is only ever compared against the generation rules,
//! never against the protocol shape.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DamageState {
    Healthy,
    Shouha,
    Chuuha,
    Taiha,
    Sunk,
}

#[derive(Debug, Default)]
pub(super) struct SlotitemResourceTargets {
    pub(super) expected: BTreeSet<String>,
    pub(super) candidate: BTreeSet<String>,
}
pub(super) fn parse_slot_resource_path(path: &str) -> Option<(String, i64)> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() < 5 || parts[0] != "kcs2" || parts[1] != "resources" || parts[2] != "slot" {
        return None;
    }
    let target_type = parts[3].to_string();
    let file_name = parts[4];
    let slot_prefix = file_name.split('_').next()?;
    let slot_id = slot_prefix.parse::<i64>().ok()?;
    Some((target_type, slot_id))
}

/// The equipment ids named by one `api_si_list` row.
///
/// CI / special-attack entries serialize as JSON strings (e.g. `"22"`) while
/// normal-attack entries are integers, so both are accepted — the CI entries
/// are exactly the ones most likely to drive a missing-resource incident. The
/// `-1` sentinel, and any other non-positive id, names no equipment.
pub(super) fn si_list_row_ids(row: &serde_json::Value) -> impl Iterator<Item = i64> + '_ {
    row.as_array()
        .into_iter()
        .flatten()
        .filter_map(|value| {
            value.as_i64().or_else(|| value.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .filter(|slot_id| *slot_id > 0)
}
pub(super) fn collect_slotitem_target_types(
    assets: &BattleKnowledgeAssets,
) -> SlotitemResourceTargets {
    let mut targets = SlotitemResourceTargets::default();

    for target_type in assets
        .resource_rules
        .rules
        .iter()
        .filter_map(|rule| {
            (rule.resource_kind == "slotitem").then_some(rule.target_type.as_deref()).flatten()
        })
        .filter(|target_type| matches!(*target_type, "item_on" | "item_up" | "btxt_flat"))
    {
        match target_type {
            // `btxt_flat` is not cut-in-only: `CutinAttack` requests it for
            // `si_list[0]` on plain shelling too, so a miss is a real 404 and
            // belongs in `expected`, where it is reported as an error.
            "item_up" | "btxt_flat" => {
                targets.expected.insert(target_type.to_string());
            }
            "item_on" => {
                targets.candidate.insert(target_type.to_string());
            }
            _ => {}
        }
    }

    targets
}

pub(super) fn classify_damage_state(now_hp: i64, max_hp: i64) -> Option<DamageState> {
    if max_hp <= 0 {
        return None;
    }

    let ratio = 100 * now_hp / max_hp;
    Some(if ratio <= 0 {
        DamageState::Sunk
    } else if ratio <= 25 {
        DamageState::Taiha
    } else if ratio <= 50 {
        DamageState::Chuuha
    } else if ratio <= 75 {
        DamageState::Shouha
    } else {
        DamageState::Healthy
    })
}

pub(super) fn uses_damaged_ship_resources(state: DamageState) -> bool {
    matches!(state, DamageState::Chuuha | DamageState::Taiha | DamageState::Sunk)
}

pub(super) fn build_ship_resource_path(
    manifest: &ApiManifest,
    ship_id: i64,
    target_type: &str,
    damaged: bool,
) -> Option<String> {
    let _ = manifest.find_ship(ship_id)?;
    let graph = manifest.find_shipgraph(ship_id)?;
    let category = if target_type == "full" {
        if damaged {
            "full_dmg"
        } else {
            "full"
        }
    } else if damaged {
        "banner_dmg"
    } else {
        "banner"
    };

    // A graph id in the hole table has no artwork upstream, so `make_list`
    // skips it on purpose. Deriving the path anyway would turn a deliberate
    // skip into a permanent finding.
    if !has_enemy_ship_coverage(ship_id, category) {
        return None;
    }

    let filename = (target_type == "full").then_some(graph.api_filename.as_str());
    Some(SuffixUtils::format_kc2_resource(ship_id as u64, "ship", category, "png", filename))
}

/// Whether `make_list`'s generation rules emit this derived resource. A target
/// type with no coverage table answers `true`: an unmodelled category is not
/// evidence of a hole.
pub(super) fn has_battle_resource_coverage(kind: &str, entity_id: i64, target_type: &str) -> bool {
    match (kind, target_type) {
        ("slotitem", "btxt_flat") => has_btxt_flat_coverage(entity_id),
        ("slotitem", "item_up") => has_repo_item_up_coverage(repo_item_up_slot_id(entity_id)),
        ("ship", category) => has_enemy_ship_coverage(entity_id, category),
        _ => true,
    }
}

/// R4: the client will request each derived path, so a path `make_list` never
/// generates is a 404 waiting to happen. Deriving these paths and then only
/// counting them is what let the `102 -> btxt_flat` incident through.
pub(super) fn push_uncovered_resource_findings(report: &mut BattleValidationReport) {
    let uncovered: Vec<ExpectedBattleResource> = report
        .expected_resources
        .iter()
        .filter(|resource| {
            !has_battle_resource_coverage(&resource.kind, resource.entity_id, &resource.target_type)
        })
        .cloned()
        .collect();

    for resource in uncovered {
        push_error(
            report,
            BattleValidationFindingKind::ProtocolSuspicion,
            None,
            format!(
                "battle response makes the client request `{}` for {} `{}` (`{}`, from {}), which make-list generation does not cover",
                resource.path,
                resource.kind,
                resource.entity_id,
                resource.target_type,
                resource.protocol_source.as_deref().unwrap_or("api_eSlot"),
            ),
            Some(resource.path),
        );
    }
}

/// Derive the slot resources one equipment id makes the client request.
///
/// `name_plate` says whether the client will also ask for this id's
/// `btxt_flat` name plate. Only display equipment triggers that, and the
/// carrier cut-in triggers it only at night, so it cannot be decided from the
/// target-type table alone.
pub(super) fn push_slotitem_resources(
    report: &mut BattleValidationReport,
    manifest: &ApiManifest,
    targets: &SlotitemResourceTargets,
    slot_id: i64,
    field: &str,
    protocol_source: Option<&str>,
    name_plate: bool,
) {
    if manifest.find_slotitem(slot_id).is_none() {
        push_error(
            report,
            BattleValidationFindingKind::UnknownSlotitemMstId,
            Some(field),
            format!("slotitem mst id `{slot_id}` is not present in the manifest"),
            None,
        );
        return;
    }

    for target_type in targets.expected.iter() {
        if target_type == "btxt_flat" && !name_plate {
            continue;
        }
        report.expected_resources.push(ExpectedBattleResource {
            kind: "slotitem".to_string(),
            entity_id: slot_id,
            target_type: target_type.to_string(),
            path: build_slotitem_resource_path(slot_id, target_type),
            note: "potential battle preload slotitem resource".to_string(),
            protocol_source: protocol_source.map(str::to_string),
            consumer_module: None,
        });
    }
    for target_type in targets.candidate.iter() {
        report.candidate_resources.push(ExpectedBattleResource {
            kind: "slotitem".to_string(),
            entity_id: slot_id,
            target_type: target_type.to_string(),
            path: build_slotitem_resource_path(slot_id, target_type),
            note: "lower-confidence battle preload slotitem resource".to_string(),
            protocol_source: protocol_source.map(str::to_string),
            consumer_module: None,
        });
    }
}

/// Day-battle attack type whose display equipment reaches `PreloadCutinKubo`,
/// which loads a name plate only at night.
///
/// The same number is `DayAttackType::CarrierCI` in `emukc_battle`, which this
/// crate cannot depend on. Two copies of a display-type fact drifting apart is
/// what produced the archived `102 -> btxt_flat` incident, so each side pins it:
/// here against the decoded acceptance asset in
/// `embedded_attack_type_acceptance_covers_every_dispatch_stage`, and there
/// against the enum in `day_attack_type_discriminants_match_the_protocol`.
pub(super) const CARRIER_CUTIN_ATTACK_TYPE: i64 = 7;

/// Display equipment ids per phase, split by whether the client will also ask
/// for each id's name plate.
///
/// `CutinAttack`, `CutinDouble` and the destroyer cut-in preloads all request
/// `btxt_flat` for the equipment they draw, with no guard. `PreloadCutinKubo`
/// -- the carrier cut-in -- requests it only when `night == 1`, so the same id
/// is a name-plate request at night and not one by day.
pub(super) fn collect_display_slot_ids(
    object: &serde_json::Map<String, serde_json::Value>,
    fields: &[&str],
    night: bool,
) -> BTreeMap<String, BTreeMap<i64, bool>> {
    let mut by_source: BTreeMap<String, BTreeMap<i64, bool>> = BTreeMap::new();

    for field in fields.iter().copied() {
        let Some(phase) = object.get(field).and_then(serde_json::Value::as_object) else {
            continue;
        };
        let Some(rows) = phase.get("api_si_list").and_then(serde_json::Value::as_array) else {
            continue;
        };
        let attack_types = phase.get("api_at_type").and_then(serde_json::Value::as_array);
        let entry = by_source.entry(format!("{field}.api_si_list[*][*]")).or_default();

        for (row_index, row) in rows.iter().enumerate() {
            let carrier_cutin = attack_types
                .and_then(|types| types.get(row_index))
                .and_then(serde_json::Value::as_i64)
                .is_some_and(|at_type| at_type == CARRIER_CUTIN_ATTACK_TYPE);
            let name_plate = night || !carrier_cutin;

            for slot_id in si_list_row_ids(row) {
                let seen = entry.entry(slot_id).or_insert(name_plate);
                *seen = *seen || name_plate;
            }
        }
    }

    by_source
}

/// Display equipment the response puts on screen. This is the only place a name
/// plate comes from: `api_eSlot` is the enemy loadout, which drives stats and
/// never a text image.
pub(super) fn push_display_slotitem_resources(
    object: &serde_json::Map<String, serde_json::Value>,
    report: &mut BattleValidationReport,
    manifest: &ApiManifest,
    targets: &SlotitemResourceTargets,
    hougeki_fields: &[&str],
    night: bool,
) {
    for (protocol_source, slot_ids) in collect_display_slot_ids(object, hougeki_fields, night) {
        let field = protocol_source.split('.').next().unwrap_or_default().to_string();
        for (slot_id, name_plate) in slot_ids {
            push_slotitem_resources(
                report,
                manifest,
                targets,
                slot_id,
                &field,
                Some(&protocol_source),
                name_plate,
            );
        }
    }
}

pub(super) fn build_slotitem_resource_path(slot_id: i64, target_type: &str) -> String {
    // `item_up` generation remaps abyssal ids before it emits a path, so the
    // raw protocol id would point at a file `make_list` never produces.
    let slot_id = if target_type == "item_up" {
        repo_item_up_slot_id(slot_id)
    } else {
        slot_id
    };
    let item_id = format!("{slot_id:04}");
    let key = SuffixUtils::create(&item_id, format!("slot_{target_type}").as_str());
    format!("kcs2/resources/slot/{target_type}/{item_id}_{key}.png")
}

pub(super) fn dedupe_resources(
    resources: Vec<ExpectedBattleResource>,
) -> Vec<ExpectedBattleResource> {
    let mut seen = BTreeSet::new();
    resources
        .into_iter()
        .filter(|resource| {
            seen.insert((
                resource.kind.clone(),
                resource.entity_id,
                resource.target_type.clone(),
                resource.path.clone(),
            ))
        })
        .collect()
}
