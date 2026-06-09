//! RNG facade module.
//!
//! Provides a stable API for all game RNG operations. Backend can be swapped
//! by changing only this module.

use std::cell::RefCell;
use std::ops::{Range, RangeInclusive};

/// Seeded RNG for deterministic simulations (battles, tests).
pub struct GameRng {
    inner: RefCell<fastrand::Rng>,
}

impl std::fmt::Debug for GameRng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameRng").finish_non_exhaustive()
    }
}

impl GameRng {
    /// Create a new seeded RNG instance.
    pub fn seeded(seed: u64) -> Self {
        Self {
            inner: RefCell::new(fastrand::Rng::with_seed(seed)),
        }
    }

    /// Return a random `i64` in `[start, end)`.
    pub fn i64(&self, range: Range<i64>) -> i64 {
        self.inner.borrow_mut().i64(range)
    }

    /// Return a random `i64` in `[start, end]` (inclusive).
    pub fn i64_inclusive(&self, range: RangeInclusive<i64>) -> i64 {
        let start = *range.start();
        let end = *range.end();
        if start == end {
            return start;
        }
        self.inner.borrow_mut().i64(start..end.saturating_add(1))
    }

    /// Return a random `usize` in `[start, end)`.
    pub fn usize(&self, range: Range<usize>) -> usize {
        self.inner.borrow_mut().usize(range)
    }

    /// Return a random `u32` in `[start, end)`.
    pub fn u32(&self, range: Range<u32>) -> u32 {
        self.inner.borrow_mut().u32(range)
    }

    /// Return a random `u64` in `[start, end)`.
    pub fn u64(&self, range: Range<u64>) -> u64 {
        self.inner.borrow_mut().u64(range)
    }

    /// Return a random `f64` in `[0.0, 1.0)`.
    pub fn f64(&self) -> f64 {
        self.inner.borrow_mut().f64()
    }

    /// Return a random `f64` in `[min, max)`.
    pub fn f64_range(&self, min: f64, max: f64) -> f64 {
        min + self.inner.borrow_mut().f64() * (max - min)
    }

    /// Shuffle a slice in place.
    pub fn shuffle<T>(&self, slice: &mut [T]) {
        self.inner.borrow_mut().shuffle(slice);
    }

    /// Choose a random element from a slice. Returns `None` if empty.
    pub fn choose<'a, T>(&self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            return None;
        }
        let idx = self.inner.borrow_mut().usize(0..slice.len());
        Some(&slice[idx])
    }

    /// Return `true` with probability `p` (0.0 = never, 1.0 = always).
    pub fn bool(&self, p: f64) -> bool {
        self.inner.borrow_mut().f64() < p
    }
}

// Thread-local free functions

/// Seed the thread-local RNG for deterministic harness and test runs.
///
/// Forwards to `fastrand`'s thread-local seed. The seed is per-OS-thread and
/// persists for the lifetime of that thread, so each harness run must re-seed at
/// its start. This is a harness/test entry point only: the live server path
/// never calls it, so production RNG stays entropy-seeded. Determinism also
/// requires a current-thread executor — on a multi-thread runtime a task can
/// migrate to an unseeded worker thread between `.await` points.
pub fn seed(s: u64) {
    fastrand::seed(s);
}

/// Return a random `i64` in `[start, end)`.
pub fn i64(range: Range<i64>) -> i64 {
    fastrand::i64(range)
}

/// Return a random `i64` in `[start, end]` (inclusive).
pub fn i64_inclusive(range: RangeInclusive<i64>) -> i64 {
    let start = *range.start();
    let end = *range.end();
    if start == end {
        return start;
    }
    fastrand::i64(start..end.saturating_add(1))
}

/// Return a random `usize` in `[start, end)`.
pub fn usize(range: Range<usize>) -> usize {
    fastrand::usize(range)
}

/// Return a random `u32` in `[start, end)`.
pub fn u32(range: Range<u32>) -> u32 {
    fastrand::u32(range)
}

/// Return a random `u64` in `[start, end)`.
pub fn u64(range: Range<u64>) -> u64 {
    fastrand::u64(range)
}

/// Return a random `f64` in `[0.0, 1.0)`.
pub fn f64() -> f64 {
    fastrand::f64()
}

/// Return a random `f64` in `[min, max)`.
pub fn f64_range(min: f64, max: f64) -> f64 {
    min + fastrand::f64() * (max - min)
}

/// Shuffle a slice in place.
pub fn shuffle<T>(slice: &mut [T]) {
    fastrand::shuffle(slice);
}

/// Choose a random element from a slice. Returns `None` if empty.
pub fn choose<T>(slice: &[T]) -> Option<&T> {
    if slice.is_empty() {
        return None;
    }
    let idx = fastrand::usize(0..slice.len());
    Some(&slice[idx])
}

/// Choose a random element from an iterator. Returns `None` if empty.
pub fn choose_iter<I>(mut iter: I) -> Option<I::Item>
where
    I: Iterator + ExactSizeIterator,
{
    let len = iter.len();
    if len == 0 {
        return None;
    }
    let idx = fastrand::usize(0..len);
    iter.nth(idx)
}

/// Return `true` with probability `p` (0.0 = never, 1.0 = always).
pub fn bool(p: f64) -> bool {
    fastrand::f64() < p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw_sequence() -> Vec<i64> {
        let mut out = Vec::new();
        for _ in 0..32 {
            out.push(usize(0..1000) as i64);
            out.push(i64(-500..500));
            out.push((f64() * 1_000_000.0) as i64);
        }
        out
    }

    #[test]
    fn seeded_sequence_is_reproducible() {
        seed(0x1234_5678);
        let first = draw_sequence();
        seed(0x1234_5678);
        let second = draw_sequence();
        assert_eq!(first, second, "same seed must reproduce the thread-local draw sequence");
    }

    #[test]
    fn distinct_seeds_diverge() {
        seed(1);
        let a = draw_sequence();
        seed(2);
        let b = draw_sequence();
        assert_ne!(a, b, "different seeds should not produce identical sequences");
    }

    #[test]
    fn reseeding_midstream_resets_sequence() {
        seed(42);
        let reference = draw_sequence();

        seed(42);
        let _ = draw_sequence(); // advance the stream past its start
        seed(42); // reseed mid-stream
        let after_reset = draw_sequence();

        assert_eq!(
            reference, after_reset,
            "reseeding mid-stream must reset to the same starting point"
        );
    }
}
