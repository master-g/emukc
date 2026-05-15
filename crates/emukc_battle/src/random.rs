/// Dependency-injected RNG interface for battle simulation.
///
/// Production code provides [`CryptoRng`](emukc_gameplay::battle::rng::CryptoRng),
/// tests provide [`SeededRng`].
pub trait BattleRng {
    /// Choose a random index in `[0, len)`.
    fn choose_index(&mut self, len: usize) -> usize {
        debug_assert!(len > 0);
        self.roll_range(0, len as i64) as usize
    }

    /// Calculate scratch (proportional) damage based on current HP.
    fn roll_scratch_damage(&mut self, current_hp: i64) -> i64 {
        let current_hp = current_hp.max(1);
        let random_part = if current_hp <= 1 {
            0
        } else {
            self.roll_range(0, current_hp)
        };
        ((current_hp as f64) * 0.06 + (random_part as f64) * 0.08).floor().max(1.0) as i64
    }

    /// Return a random `f64` in `[min, max)`.
    fn random_f64_range(&mut self, min: f64, max: f64) -> f64;

    /// Return a random `i64` in `[min, max)`. Handles `min >= max` gracefully.
    fn roll_range(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        self.roll_range_impl(min, max)
    }

    /// Inner implementation — callers should prefer [`roll_range`](Self::roll_range).
    fn roll_range_impl(&mut self, min: i64, max: i64) -> i64;
}

/// Deterministic RNG for tests. Wraps a seeded [`GameRng`](emukc_crypto::rng::GameRng).
#[cfg(test)]
pub struct SeededRng {
    inner: emukc_crypto::rng::GameRng,
}

#[cfg(test)]
impl SeededRng {
    /// Create a new seeded RNG for testing.
    pub fn new(seed: u64) -> Self {
        Self {
            inner: emukc_crypto::rng::GameRng::seeded(seed),
        }
    }
}

#[cfg(test)]
impl BattleRng for SeededRng {
    fn random_f64_range(&mut self, min: f64, max: f64) -> f64 {
        self.inner.f64_range(min, max)
    }

    fn roll_range_impl(&mut self, min: i64, max: i64) -> i64 {
        self.inner.i64(min..max)
    }
}
