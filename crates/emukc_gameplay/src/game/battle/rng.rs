use emukc_battle::BattleRng;

/// Production RNG backed by thread-local `fastrand`.
pub(crate) struct CryptoRng;

impl BattleRng for CryptoRng {
    fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
        emukc_crypto::rng::f64_range(min, max)
    }

    fn roll_range_impl(&mut self, min: i64, max: i64) -> i64 {
        emukc_crypto::rng::i64(min..max)
    }

    fn choose_index(&mut self, len: usize) -> usize {
        emukc_crypto::rng::usize(0..len)
    }

    fn roll_scratch_damage(&mut self, current_hp: i64) -> i64 {
        let current_hp = current_hp.max(1);
        let random_part = if current_hp <= 1 {
            0
        } else {
            emukc_crypto::rng::i64(0..current_hp)
        };
        ((current_hp as f64) * 0.06 + (random_part as f64) * 0.08).floor().max(1.0) as i64
    }
}
