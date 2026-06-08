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

#[cfg(test)]
mod tests {
    use super::*;

    /// Recompute the scratch formula from a single independently-drawn value.
    fn scratch_from_one_draw(hp: i64, random_part: i64) -> i64 {
        ((hp as f64) * 0.06 + (random_part as f64) * 0.08).floor().max(1.0) as i64
    }

    /// `roll_scratch_damage` must consume exactly one `roll_range(0, hp)` draw and
    /// apply `floor(0.06*hp + 0.08*r).max(1)`. This pins the formula and the
    /// single-draw RNG consumption that production damage depends on (plan KTD-1):
    /// the override was deleted in favor of this trait default precisely because
    /// they were draw-for-draw identical, so any future drift here is a real bug.
    #[test]
    fn roll_scratch_damage_matches_single_draw_formula() {
        for hp in [2_i64, 10, 37, 100, 999] {
            let mut actual_rng = SeededRng::new(0x5C2A7C4);
            let actual = actual_rng.roll_scratch_damage(hp);

            // Reproduce the expected single draw from an identically seeded RNG.
            let mut shadow_rng = SeededRng::new(0x5C2A7C4);
            let random_part = shadow_rng.roll_range(0, hp);
            let expected = scratch_from_one_draw(hp, random_part);

            assert_eq!(actual, expected, "scratch damage formula/draw-count drift at hp={hp}");
            // The clamp the only caller applies must never exceed current HP.
            assert!(actual >= 1 && actual <= hp, "scratch {actual} out of [1, {hp}]");
        }
    }

    /// Frozen golden vector: a fixed seed must always yield this exact sequence of
    /// scratch-damage rolls. Unlike the formula test above, this also catches a
    /// change of the underlying RNG backend or seeding algorithm (both sides of
    /// the formula test would move together and hide such a change).
    ///
    /// To regenerate intentionally: run this test, copy the `left` values from the
    /// failure into `EXPECTED`, and note the behavior change in the commit message.
    #[test]
    fn roll_scratch_damage_golden_vector() {
        const SEED: u64 = 0xC0FFEE;
        const HP: i64 = 100;
        const EXPECTED: [i64; 8] = [6, 12, 9, 7, 10, 9, 9, 10];

        let mut rng = SeededRng::new(SEED);
        let got: [i64; 8] = std::array::from_fn(|_| rng.roll_scratch_damage(HP));

        assert_eq!(got, EXPECTED, "scratch-damage RNG sequence drifted for seed {SEED:#x}");
    }

    /// `current_hp <= 1` returns 1 and must consume **no** RNG draw, so it does not
    /// perturb the battle RNG sequence for subsequent rolls.
    #[test]
    fn roll_scratch_damage_low_hp_returns_one_without_drawing() {
        const SEED: u64 = 0xBADA55;

        for hp in [i64::MIN, -5, 0, 1] {
            let mut rng = SeededRng::new(SEED);
            assert_eq!(rng.roll_scratch_damage(hp), 1, "low-hp scratch must be 1 (hp={hp})");

            // The generator must be untouched: its next draw equals a fresh RNG's first.
            let next = rng.roll_range(0, 1_000_000);
            let first = SeededRng::new(SEED).roll_range(0, 1_000_000);
            assert_eq!(next, first, "low-hp scratch must not consume an RNG draw (hp={hp})");
        }
    }
}
