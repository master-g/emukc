mod types;
#[cfg_attr(
    not(test),
    expect(unused_imports, reason = "`RouteRuleDraft` is only named by the `map_pipeline` tests")
)]
pub use types::{
    EnemyNodeRows, LabelOverlay, LabelOverlayCatalog, LabelOverlayDefinition, RouteRuleDraft,
    ShipDropDraft,
};
