use axum::{Router, routing::post};

mod airbattle;
mod battle;
mod battleresult;
mod goback_port;

pub(super) fn router() -> Router {
	Router::new()
		.route("/airbattle", post(airbattle::handler))
		.route("/battle", post(battle::handler))
		.route("/battleresult", post(battleresult::handler))
		.route("/goback_port", post(goback_port::handler))
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
}
