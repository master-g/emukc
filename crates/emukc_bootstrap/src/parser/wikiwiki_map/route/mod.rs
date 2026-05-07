mod route_condition;
mod route_predicate;
mod route_table;

pub(super) use route_condition::{
    build_nodes, check_mixed_routing_encoding, parse_route_table, postprocess_route_probabilities,
};
pub(super) use route_predicate::compact_route_raw_text;

#[cfg(test)]
pub(super) use route_condition::{
    parse_case_route_condition_text, parse_conditional_random_route_condition_text,
    parse_independent_route_condition_line, parse_inline_targeted_route_condition_text,
    parse_row_target_random_bias_condition_text,
    parse_row_target_random_bias_shorthand_condition_text,
    parse_target_random_route_condition_text,
};
#[cfg(test)]
pub(super) use route_predicate::parse_route_predicate;
pub(super) use route_table::{
    collect_formations, find_route_table_sections, parse_gauge_defeat_counts,
    route_section_variant_key,
};

// Re-export types used by tests
#[cfg(test)]
use super::RouteRuleDraft;

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_model::codex::map::RoutePredicate;

    fn make_draft(from: &str, to: &str, probability_pct: Option<f64>) -> RouteRuleDraft {
        RouteRuleDraft {
            from_label: from.to_string(),
            to_label: to.to_string(),
            probability_pct,
            predicate: RoutePredicate::Always,
            raw_text: String::new(),
            random_placeholder: false,
        }
    }

    #[test]
    fn check_mixed_routing_encoding_emits_warning_for_mixed_cell() {
        // One rule has probability_pct (probability encoding), one has None (weight encoding).
        let rules = vec![
            make_draft("A", "B", Some(50.0)), // probability encoding
            make_draft("A", "C", None),       // weight encoding
        ];
        let mut warnings = Vec::new();
        check_mixed_routing_encoding(&rules, &mut warnings);
        assert!(
            warnings.iter().any(|w| w == "mixed_routing_encoding_cell_A"),
            "expected mixed_routing_encoding_cell_A, got: {warnings:?}"
        );
    }

    #[test]
    fn check_mixed_routing_encoding_no_warning_when_all_probability() {
        let rules = vec![make_draft("B", "C", Some(60.0)), make_draft("B", "D", Some(40.0))];
        let mut warnings = Vec::new();
        check_mixed_routing_encoding(&rules, &mut warnings);
        assert!(
            !warnings.iter().any(|w| w.contains("mixed_routing_encoding")),
            "unexpected warning: {warnings:?}"
        );
    }

    #[test]
    fn check_mixed_routing_encoding_no_warning_when_all_weight() {
        let rules = vec![make_draft("C", "D", None), make_draft("C", "E", None)];
        let mut warnings = Vec::new();
        check_mixed_routing_encoding(&rules, &mut warnings);
        assert!(
            !warnings.iter().any(|w| w.contains("mixed_routing_encoding")),
            "unexpected warning: {warnings:?}"
        );
    }

    #[test]
    fn check_mixed_routing_encoding_different_cells_no_cross_contamination() {
        // Cell A: uniform probability, Cell B: uniform weight — no mixing per cell.
        let rules = vec![
            make_draft("A", "X", Some(70.0)),
            make_draft("A", "Y", Some(30.0)),
            make_draft("B", "X", None),
            make_draft("B", "Z", None),
        ];
        let mut warnings = Vec::new();
        check_mixed_routing_encoding(&rules, &mut warnings);
        assert!(
            !warnings.iter().any(|w| w.contains("mixed_routing_encoding")),
            "unexpected warning: {warnings:?}"
        );
    }
}
