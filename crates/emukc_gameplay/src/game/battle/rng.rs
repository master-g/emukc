use emukc_battle::BattleRng;

/// Non-cryptographic production RNG backed by `emukc_crypto::rng` (fastrand).
/// For deterministic test runs, use `SeededRng` from `emukc_battle::random`.
pub(crate) struct ProductionRng;

impl BattleRng for ProductionRng {
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
    fn choose_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        Some(emukc_crypto::rng::usize(0..len))
    }
}

#[cfg(test)]
mod tests {
    use super::ProductionRng;
    use emukc_battle::BattleRng;

    fn production_draws() -> Vec<i64> {
        let mut rng = ProductionRng;
        let mut out = Vec::new();
        for _ in 0..16 {
            out.push(rng.roll_range_impl(0, 100));
            out.push((rng.random_f64_range(0.0, 1.0) * 1_000_000.0) as i64);
            out.push(rng.choose_index(8).expect("len 8 by construction") as i64);
        }
        out
    }

    #[test]
    fn production_rng_is_deterministic_after_seed() {
        // The keystone: seeding the one thread-local generator determinizes the
        // battle math, because ProductionRng draws through the emukc_crypto facade.
        emukc_crypto::rng::seed(0x00C0_FFEE);
        let first = production_draws();
        emukc_crypto::rng::seed(0x00C0_FFEE);
        let second = production_draws();
        // Restore entropy before asserting so a failure cannot skip the cleanup
        // and leak the seeded stream to other tests on this thread.
        emukc_crypto::rng::reseed_from_entropy();
        assert_eq!(first, second, "seeding the thread-local must determinize ProductionRng draws");
    }
}
