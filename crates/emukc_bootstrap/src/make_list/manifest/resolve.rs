use emukc_model::kc2::start2::ApiManifest;

/// Resolve a ship MST ID source expression to concrete ship IDs.
/// Known patterns map to "all friendly ships" (ships with `api_sortno` or `api_aftershipid`).
/// Unknown patterns emit a warning and return empty.
pub(crate) fn resolve_ship_ids(source: &str, mst: &ApiManifest) -> Vec<i64> {
    if is_universal_ship_source(source) {
        mst.api_mst_ship
            .iter()
            .filter(|s| s.api_sortno.is_some() || s.api_aftershipid.is_some())
            .map(|s| s.api_id)
            .collect()
    } else {
        warn!("Unknown shipMstIdSource expression: {source}, skipping entry");
        Vec::new()
    }
}

fn is_universal_ship_source(source: &str) -> bool {
    match source {
        "self.shipModel.mstID"
        | "this._mst_id"
        | "this._ship_mst_id"
        | "this._ship_mstid"
        | "mst_id"
        | "mstID"
        | "ship_mstid"
        | "self._mst_id"
        | "this.ship_mstid" => true,
        s if s.starts_with("_0x") => true,
        s if s.ends_with(".mstID") => true,
        s if s.ends_with(".mst_id") => true,
        s if s.contains("_attackers") => true,
        _ => false,
    }
}

/// Resolve slotitem MST ID source expressions to concrete slotitem IDs.
pub(crate) fn resolve_slotitem_ids(sources: &[String], mst: &ApiManifest) -> Vec<i64> {
    let any_known = sources.iter().any(|s| is_universal_slotitem_source(s));
    if any_known {
        mst.api_mst_slotitem.iter().filter(|s| s.api_sortno > 0).map(|s| s.api_id).collect()
    } else {
        let sources_str = sources.join(", ");
        warn!("Unknown slotMstIdSources expressions: [{sources_str}], skipping entry");
        Vec::new()
    }
}

fn is_universal_slotitem_source(source: &str) -> bool {
    let normalized = source.to_ascii_lowercase();
    match normalized.as_str() {
        "this._mst_id" | "self._mst_id" | "mst_id" | "slotitemmstid" => true,
        s if s.starts_with("_0x") => true,
        s if s.ends_with(".mst_id") => true,
        s if s.ends_with("mstid") => true,
        s if s.ends_with("_item_id") => true,
        s if s.contains("._slot") => true,
        s if s.contains("._plane") => true,
        s if s.ends_with("[0]") => true,
        s if s.ends_with("[1]") => true,
        s if s.ends_with("[2]") => true,
        _ => false,
    }
}

/// Resolve a damaged source expression.
/// Returns Some(false) for "false", Some(true) for "true", None for unknown (generate both).
pub(crate) fn resolve_damaged(source: &str) -> Option<bool> {
    match source {
        "false" => Some(false),
        "true" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damaged_resolution() {
        assert_eq!(resolve_damaged("false"), Some(false));
        assert_eq!(resolve_damaged("true"), Some(true));
        assert_eq!(resolve_damaged("damaged"), None);
        assert_eq!(resolve_damaged("_0x12345"), None);
    }

    #[test]
    fn test_ship_source_classification() {
        assert!(is_universal_ship_source("self.shipModel.mstID"));
        assert!(is_universal_ship_source("this._mst_id"));
        assert!(is_universal_ship_source("_0x1d0f2d"));
        assert!(is_universal_ship_source("_0x3e5c95.mstID"));
        assert!(is_universal_ship_source("this._attackers[0].mst_id"));
        assert!(!is_universal_ship_source("completely_unknown"));
    }

    #[test]
    fn test_slotitem_source_classification() {
        assert!(is_universal_slotitem_source("this._mst_id"));
        assert!(is_universal_slotitem_source("_0x14e26f"));
        assert!(is_universal_slotitem_source("this._slot1.mstID"));
        assert!(is_universal_slotitem_source("slotitemMstID"));
        assert!(!is_universal_slotitem_source("totally_unknown"));
    }

    #[test]
    fn test_readable_slotitem_source_classification() {
        for source in [
            "this._after_item_id",
            "this._before_item_id",
            "this._zuiunMstId",
            "this._mainArmamentMstID",
            "this._plane_mst_id",
            "this._plane1",
            "this._plane2",
            "this._plane3",
            "this._plane_f",
            "this._plane_e",
        ] {
            assert!(is_universal_slotitem_source(source), "unrecognized source: {source}");
        }
    }
}
