use axum::{Router, routing::post};

mod change;
mod combined;
mod lock;
mod preset_delete;
mod preset_expand;
mod preset_lock;
mod preset_order_change;
mod preset_register;
mod preset_select;

pub(super) fn router() -> Router {
    Router::new()
        .route("/change", post(change::handler))
        .route("/combined", post(combined::handler))
        .route("/lock", post(lock::handler))
        .route("/preset_delete", post(preset_delete::handler))
        .route("/preset_expand", post(preset_expand::handler))
        .route("/preset_lock", post(preset_lock::handler))
        .route("/preset_order_change", post(preset_order_change::handler))
        .route("/preset_register", post(preset_register::handler))
        .route("/preset_select", post(preset_select::handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::auth::Pid;
    use crate::net::router::kcsapi::test_utils::{app_state, new_test_context};
    use axum::Form;
    use emukc::model::profile::preset_deck::PresetDeckItem;
    use emukc_internal::prelude::*;

    fn preset(index: i64, name: &str) -> PresetDeckItem {
        PresetDeckItem {
            index,
            name: name.to_string(),
            ships: [1, -1, -1, -1, -1, -1, -1],
            locked: false,
        }
    }

    #[tokio::test]
    async fn a_locked_preset_is_neither_overwritten_nor_deleted() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        context.state.register_preset_deck(pid, &preset(1, "first")).await.unwrap();

        let toggle = || {
            preset_lock::handler(
                app_state(&context.state),
                Pid(pid),
                Form(preset_lock::Params {
                    api_preset_no: 1,
                }),
            )
        };
        toggle().await.unwrap();

        let decks: KcApiPresetDeck = context.state.get_preset_decks(pid).await.unwrap().into();
        assert_eq!(decks.api_deck["1"].api_lock_flag, 1, "the client reads the lock here");
        assert!(context.state.register_preset_deck(pid, &preset(1, "other")).await.is_err());
        context.state.delete_preset_deck(pid, 1).await.unwrap();
        assert_eq!(context.state.find_preset_deck(pid, 1).await.unwrap().name, "first");

        toggle().await.unwrap();
        context.state.delete_preset_deck(pid, 1).await.unwrap();
        assert!(context.state.find_preset_deck(pid, 1).await.is_err(), "unlocked, it goes");
    }

    #[tokio::test]
    async fn reordering_exchanges_two_preset_numbers() {
        let context = new_test_context().await;
        let pid = context.session.profile.id;
        context.state.register_preset_deck(pid, &preset(1, "first")).await.unwrap();
        context.state.register_preset_deck(pid, &preset(2, "second")).await.unwrap();

        let exchange = |from, to| {
            preset_order_change::handler(
                app_state(&context.state),
                Pid(pid),
                Form(preset_order_change::Params {
                    api_preset_from: from,
                    api_preset_to: to,
                }),
            )
        };

        exchange(1, 2).await.unwrap();
        assert_eq!(context.state.find_preset_deck(pid, 1).await.unwrap().name, "second");
        assert_eq!(context.state.find_preset_deck(pid, 2).await.unwrap().name, "first");

        // Onto an empty number: the preset moves and leaves its own empty.
        exchange(1, 3).await.unwrap();
        assert!(context.state.find_preset_deck(pid, 1).await.is_err());
        assert_eq!(context.state.find_preset_deck(pid, 3).await.unwrap().name, "second");

        assert!(exchange(1, 99).await.is_err(), "there is no preset 99");
    }
}
