use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Progress tracker for greedy mode
pub struct ProgressTracker {
	total: usize,
	checked: AtomicUsize,
	found: AtomicUsize,
	start_time: Instant,
}

impl ProgressTracker {
	pub fn new(total: usize) -> Self {
		Self {
			total,
			checked: AtomicUsize::new(0),
			found: AtomicUsize::new(0),
			start_time: Instant::now(),
		}
	}

	pub fn increment_checked(&self) {
		self.checked.fetch_add(1, Ordering::Relaxed);
	}

	pub fn increment_found(&self) {
		self.found.fetch_add(1, Ordering::Relaxed);
	}

	pub fn report(&self) {
		let checked = self.checked.load(Ordering::Relaxed);
		let found = self.found.load(Ordering::Relaxed);
		let elapsed = self.start_time.elapsed();
		let rate = checked as f64 / elapsed.as_secs_f64();

		if checked > 0 {
			let eta_secs = ((self.total - checked) as f64 / rate) as u64;
			info!(
				"Progress: {}/{} checked, {} found ({:.1}%), {:.1} checks/s, ETA: {}s",
				checked,
				self.total,
				found,
				(found as f64 / checked as f64) * 100.0,
				rate,
				eta_secs
			);
		}
	}
}
