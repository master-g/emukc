use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let materials = state.get_materials(pid).await?;
    let materials: Vec<KcApiMaterialElement> = materials.into();
    Ok(KcApiResponse::success(&materials))
}
