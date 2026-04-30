use std::sync::atomic::{AtomicUsize, Ordering};

use indicatif::ProgressBar;

use crate::progress::{make_list_style, new_progress_bar};

/// Progress tracker for greedy mode
pub struct ProgressTracker {
    total: usize,
    checked: AtomicUsize,
    found: AtomicUsize,
    pb: Option<ProgressBar>,
}

impl ProgressTracker {
    pub fn new(total: usize) -> Self {
        let pb = new_progress_bar(total as u64, "Checking resources", make_list_style());
        Self {
            total,
            checked: AtomicUsize::new(0),
            found: AtomicUsize::new(0),
            pb,
        }
    }

    pub fn increment_checked(&self) {
        let checked = self.checked.fetch_add(1, Ordering::Relaxed) + 1;
        let found = self.found.load(Ordering::Relaxed);
        if let Some(ref pb) = self.pb {
            pb.inc(1);
            pb.set_message(format!("{checked}/{} checked, {found} found", self.total));
        }
    }

    pub fn increment_found(&self) {
        self.found.fetch_add(1, Ordering::Relaxed);
    }

    pub fn report(&self) {
        let checked = self.checked.load(Ordering::Relaxed);
        let found = self.found.load(Ordering::Relaxed);

        if let Some(ref pb) = self.pb {
            pb.set_message(format!("{checked}/{} checked, {found} found", self.total));
        } else if checked > 0 {
            info!("Progress: {}/{} checked, {} found", checked, self.total, found);
        }
    }

    pub fn finish(&self) {
        if let Some(ref pb) = self.pb {
            let checked = self.checked.load(Ordering::Relaxed);
            let found = self.found.load(Ordering::Relaxed);
            pb.finish_with_message(format!(
                "Checking resources  done ({checked} checked, {found} found)"
            ));
        }
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        self.finish();
    }
}
