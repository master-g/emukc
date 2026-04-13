use axum::{Router, routing::post};

mod airbattle;
mod battle;
mod battleresult;
mod goback_port;
mod ld_airbattle;
mod ld_shooting;

pub(super) fn router() -> Router {
    Router::new()
        .route("/airbattle", post(airbattle::handler))
        .route("/battle", post(battle::handler))
        .route("/battleresult", post(battleresult::handler))
        .route("/goback_port", post(goback_port::handler))
        .route("/ld_airbattle", post(ld_airbattle::handler))
        .route("/ld_shooting", post(ld_shooting::handler))
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
    async fn battle_battleresult_and_goback_handlers_drive_sortie_flow() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        context.state.start_sortie(pid, 1, 1, 1, 1).await.unwrap();

        let battle = battle::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(battle::Params {
                api_formation: 1,
                api_recovery_type: 0,
                api_supply_flag: None,
                api_ration_flag: None,
                api_smoke_flag: None,
            }),
        )
        .await
        .unwrap();
        let battle_data = battle.api_data.unwrap();
        assert_eq!(battle_data["api_deck_id"], 1);

        let result = battleresult::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(battleresult::Params::default()),
        )
        .await
        .unwrap();
        let result_data = result.api_data.unwrap();
        assert!(result_data["api_win_rank"].as_str().is_some());
        assert!(result_data["api_get_flag"].as_array().is_some());

        let goback =
            goback_port::handler(app_state(&context.state), Extension(context.session.clone()))
                .await
                .unwrap();
        assert_eq!(goback.api_result, 1);
    }

    #[tokio::test]
    async fn airbattle_handler_returns_packet_with_no_shelling() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        context.state.start_sortie(pid, 1, 1, 1, 1).await.unwrap();

        let resp = airbattle::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(airbattle::Params {
                api_formation: 1,
                api_recovery_type: 0,
                api_supply_flag: None,
                api_ration_flag: None,
                api_smoke_flag: None,
            }),
        )
        .await
        .unwrap();
        let data = resp.api_data.unwrap();
        assert_eq!(data["api_deck_id"], 1);
        // AirBattle mode: shelling and torpedo should be absent
        let hourai = data["api_hourai_flag"].as_array().unwrap();
        assert_eq!(hourai[0].as_i64().unwrap(), 0, "hougeki1 should not run in airbattle");
        assert_eq!(hourai[3].as_i64().unwrap(), 0, "raigeki should not run in airbattle");
    }

    #[tokio::test]
    async fn ld_airbattle_handler_disallows_midnight() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        context.state.start_sortie(pid, 1, 1, 1, 1).await.unwrap();

        let resp = ld_airbattle::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(ld_airbattle::Params {
                api_formation: 1,
                api_recovery_type: 0,
                api_supply_flag: None,
                api_ration_flag: None,
                api_smoke_flag: None,
            }),
        )
        .await
        .unwrap();
        let data = resp.api_data.unwrap();
        assert_eq!(data["api_deck_id"], 1);
        assert_eq!(
            data["api_midnight_flag"].as_i64().unwrap(),
            0,
            "LdAirBattle must not allow midnight"
        );
    }

    #[tokio::test]
    async fn ld_shooting_handler_disallows_midnight() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        seed_single_ship_fleet(&context.state, pid).await;

        context.state.start_sortie(pid, 1, 1, 1, 1).await.unwrap();

        let resp = ld_shooting::handler(
            app_state(&context.state),
            Extension(context.session.clone()),
            Form(ld_shooting::Params {
                api_formation: 1,
                api_recovery_type: 0,
                api_supply_flag: None,
                api_ration_flag: None,
                api_smoke_flag: None,
            }),
        )
        .await
        .unwrap();
        let data = resp.api_data.unwrap();
        assert_eq!(data["api_deck_id"], 1);
        assert_eq!(
            data["api_midnight_flag"].as_i64().unwrap(),
            0,
            "LdShooting must not allow midnight"
        );
    }
}
