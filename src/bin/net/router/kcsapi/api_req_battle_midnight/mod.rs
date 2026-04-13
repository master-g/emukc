use axum::{Router, routing::post};

mod battle;
mod sp_midnight;

pub(super) fn router() -> Router {
    Router::new()
        .route("/battle", post(battle::handler))
        .route("/sp_midnight", post(sp_midnight::handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::router::kcsapi::test_utils::{
        app_state, new_test_context, seed_single_ship_fleet,
    };
    use axum::{Extension, Form};
    use emukc_internal::prelude::SortieOps;

    #[tokio::test]
    async fn sp_midnight_handler_accepts_formation_and_returns_night_packet() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        context.state.start_sortie(pid, 1, 1, 1, 1).await.unwrap();

        let resp = sp_midnight::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(sp_midnight::Params {
                api_formation: 3,
            }),
        )
        .await
        .unwrap();
        let data = resp.api_data.unwrap();
        assert_eq!(data["api_deck_id"], 1);
        // Night battle packet should have hougeki
        assert!(data["api_hougeki"].is_object(), "sp_midnight should contain night hougeki");
    }
}
