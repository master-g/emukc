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

    // `choose_index` keeps its override: it draws via `emukc_crypto::rng::usize`,
    // whose generator-consumption differs from the trait default's `i64` path,
    // so removing it could shift the production RNG sequence. The
    // `roll_scratch_damage` override was deleted (the trait default routes through
    // the same `emukc_crypto::rng::i64` backend, so it is draw-for-draw identical).
    fn choose_index(&mut self, len: usize) -> usize {
        emukc_crypto::rng::usize(0..len)
    }
}
