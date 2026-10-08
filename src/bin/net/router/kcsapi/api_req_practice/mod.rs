use axum::{Router, routing::post};

mod battle;
mod battle_result;
mod change_matching_kind;
mod midnight_battle;

pub(super) fn router() -> Router {
    Router::new()
        .route("/battle", post(battle::handler))
        .route("/battle_result", post(battle_result::handler))
        .route("/change_matching_kind", post(change_matching_kind::handler))
        .route("/midnight_battle", post(midnight_battle::handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::auth::Pid;
    use crate::net::router::kcsapi::test_utils::{app_state, new_test_context};
    use axum::Form;

    #[tokio::test]
    async fn the_matching_kind_is_kept_for_the_next_list() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;

        let choose = |kind| {
            change_matching_kind::handler(
                app_state(&context.state),
                Pid(pid),
                Form(change_matching_kind::Params {
                    api_selected_kind: kind,
                }),
            )
        };

        let data = choose(2).await.unwrap().api_data.unwrap();
        assert_eq!(data["api_update_flag"], 1);
        let rivals = context.state.get_practice_rivals(pid).await.unwrap();
        assert_eq!(rivals.cfg.selected_type as i64, 2, "what `practice` reports as selected");

        assert!(choose(3).await.is_err(), "there are three groups");
    }
}
