use axum::{Router, routing::post};

mod next;
mod projection;
mod select_eventmap_rank;
mod start;

pub(super) fn router() -> Router {
    Router::new()
        .route("/next", post(next::handler))
        .route("/select_eventmap_rank", post(select_eventmap_rank::handler))
        .route("/start", post(start::handler))
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
    async fn start_and_next_handlers_drive_map_progression() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        let start = start::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(start::Params {
                api_deck_id: 1,
                api_maparea_id: 1,
                api_mapinfo_no: 1,
                api_serial_cid: String::new(),
            }),
        )
        .await
        .unwrap();
        let start_data = start.api_data.unwrap();
        assert_eq!(start_data["api_maparea_id"], 1);
        assert_eq!(start_data["api_mapinfo_no"], 1);

        let next = next::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(next::Params {
                api_recovery_type: 0,
                api_cell_id: Some(2),
            }),
        )
        .await
        .unwrap();
        let next_data = next.api_data.unwrap();
        assert_eq!(next_data["api_maparea_id"], 1);
        assert_eq!(next_data["api_mapinfo_no"], 1);
        assert_eq!(next_data["api_from_no"], start_data["api_no"]);
    }

    #[tokio::test]
    async fn start_handler_returns_exact_cells_for_world_1_1() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        let start = start::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(start::Params {
                api_deck_id: 1,
                api_maparea_id: 1,
                api_mapinfo_no: 1,
                api_serial_cid: String::new(),
            }),
        )
        .await
        .unwrap();
        let start_data = start.api_data.unwrap();
        let cells = start_data["api_cell_data"].as_array().unwrap();

        let api_nos = cells.iter().map(|cell| cell["api_no"].as_i64().unwrap()).collect::<Vec<_>>();
        let api_ids = cells.iter().map(|cell| cell["api_id"].as_i64().unwrap()).collect::<Vec<_>>();

        assert_eq!(api_nos, vec![0, 1, 2, 3]);
        assert_eq!(api_ids, vec![3001, 3002, 3003, 3004]);
        assert!(!cells.iter().any(|cell| cell["api_no"].as_i64() == Some(4)));
        assert!(!cells.iter().any(|cell| cell["api_id"].as_i64() == Some(1104)));
    }

    #[tokio::test]
    async fn next_handler_rejects_while_battle_result_is_pending() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        start::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(start::Params {
                api_deck_id: 1,
                api_maparea_id: 1,
                api_mapinfo_no: 1,
                api_serial_cid: String::new(),
            }),
        )
        .await
        .unwrap();

        context.state.sortie_battle(pid, 1).await.unwrap();

        assert!(
            next::handler(
                app_state(&context.state),
                Extension(context.session.clone()),
                Form(next::Params {
                    api_recovery_type: 0,
                    api_cell_id: None,
                }),
            )
            .await
            .is_err()
        );
    }
}
