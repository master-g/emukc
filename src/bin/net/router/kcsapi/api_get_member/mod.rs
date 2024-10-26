use axum::{routing::post, Router};

mod basic;
mod deck;
mod furniture;
mod kdock;
mod material;
mod ndock;
mod payitem;
mod picture_book;
mod preset_deck;
mod record;
mod require_info;
mod ship2;
mod ship3;
mod slot_item;
mod sortie_conditions;
mod unsetslot;
mod useitem;

pub(super) fn router() -> Router {
	Router::new()
		.route("/basic", post(basic::handler))
		.route("/deck", post(deck::handler))
		.route("/furniture", post(furniture::handler))
		.route("/kdock", post(kdock::handler))
		.route("/material", post(material::handler))
		.route("/ndock", post(ndock::handler))
		.route("/payitem", post(payitem::handler))
		.route("/picture_book", post(picture_book::handler))
		.route("/preset_deck", post(preset_deck::handler))
		.route("/record", post(record::handler))
		.route("/require_info", post(require_info::handler))
		.route("/ship2", post(ship2::handler))
		.route("/ship3", post(ship3::handler))
		.route("/slot_item", post(slot_item::handler))
		.route("/sortie_conditions", post(sortie_conditions::handler))
		.route("/unsetslot", post(unsetslot::handler))
		.route("/useitem", post(useitem::handler))
}
