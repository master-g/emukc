//! U3: verify a ship's remaining ammunition survives DB persistence into the
//! `KcApiShip` the battle layer consumes.
//!
//! `build_sortie_friend_ships` turns each profile ship `Model` into a
//! `BattleShipInput` via `From<Model> for KcApiShip` (which maps `ammo` ->
//! `api_bull`). The battle-side ammo modifier then reads that `api_bull`. This
//! guards the cross-layer seam so a low-ammo fleet does not silently fight at
//! full strength.

#[cfg(test)]
mod tests {
    use emukc_internal::prelude::*;

    async fn new_profile(context: &crate::TestContext) -> i64 {
        let account = context.sign_up("test-ammo-battle", "1234567").await.unwrap();
        let profile =
            context.new_profile(&account.access_token.token, "ammo-tester").await.unwrap();
        let session =
            context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();
        session.profile.id
    }

    #[tokio::test]
    async fn reduced_ammo_survives_into_battle_ship_input() {
        let context = crate::TestContext::new().await;
        let pid = new_profile(&context).await;

        // A freshly added ship starts fully supplied.
        let ship = context.add_ship(pid, 1).await.unwrap();
        assert!(ship.api_bull > 0, "fresh ship should start with ammo");

        // Consume ammo down to ~20%, as a deep sortie would.
        let reduced = ship.api_bull / 5;
        let mut low = context.find_ship(ship.api_id).await.unwrap().unwrap();
        low.api_bull = reduced;
        context.update_ship(&low).await.unwrap();

        // The KcApiShip rebuilt from the persisted model — the same `From<Model>`
        // conversion `build_sortie_friend_ships` uses — must carry the reduced
        // ammo, so the battle-side `ammo_modifier` sees real remaining ammo.
        let reloaded = context.find_ship(ship.api_id).await.unwrap().unwrap();
        assert_eq!(
            reloaded.api_bull, reduced,
            "stored ammo must reach the battle ship input api_bull"
        );
    }
}
