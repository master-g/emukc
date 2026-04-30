use std::io::{IsTerminal, Write};
use std::sync::Arc;
use std::time::Duration;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

fn is_tty() -> bool {
    std::io::stderr().is_terminal()
}

const POPULATE_STYLE: &str = "{msg}  {bar:40.cyan/blue}  {pos}/{len} ({percent}%, ETA: {eta})";
const MAKE_LIST_STYLE: &str =
    "{msg}  {bar:40.cyan/blue}  {pos}/{len} ({percent}%, {per_sec}, ETA: {eta})";
const DOWNLOAD_AGGREGATE_STYLE: &str =
    "{msg}  {bar:40.cyan/blue}  {pos}/{len} ({percent}%, ETA: {eta})";
const SPINNER_STYLE: &str = "{spinner} {msg}";

/// Create a progress bar registered with a [`MultiProgress`] from the start.
///
/// The bar is added to `mp` before any drawing occurs, ensuring no stray
/// lines appear on the terminal.
pub fn new_progress_bar_on_mp(
    total: u64,
    message: &str,
    style_template: &str,
    mp: &MultiProgress,
) -> ProgressBar {
    let pb = mp.add(ProgressBar::new(total));
    pb.set_style(
        ProgressStyle::with_template(style_template)
            .expect("invalid progress style template")
            .progress_chars("━╸─"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a standalone progress bar (not managed by [`MultiProgress`]).
pub fn new_progress_bar(total: u64, message: &str, style_template: &str) -> Option<ProgressBar> {
    if !is_tty() {
        return None;
    }

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(style_template)
            .expect("invalid progress style template")
            .progress_chars("━╸─"),
    );
    pb.set_message(message.to_string());
    pb.tick();
    Some(pb)
}

/// Create a spinner registered with a [`MultiProgress`] from the start.
///
/// The spinner is added to `mp` before steady tick is enabled,
/// ensuring all draws go through [`MultiProgress`].
pub fn new_spinner_on_mp(message: &str, mp: &MultiProgress) -> ProgressBar {
    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::with_template(SPINNER_STYLE)
            .expect("invalid spinner style template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a standalone spinner (not managed by [`MultiProgress`]).
pub fn new_spinner(message: &str) -> Option<ProgressBar> {
    if !is_tty() {
        return None;
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template(SPINNER_STYLE)
            .expect("invalid spinner style template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    Some(pb)
}

/// Create a stats bar registered with a [`MultiProgress`] from the start.
pub fn new_stats_bar_on_mp(max_concurrent: usize, mp: &MultiProgress) -> ProgressBar {
    let pb = mp.add(ProgressBar::new(0));
    pb.set_style(ProgressStyle::with_template("{msg}").expect("invalid stats style template"));
    pb.set_message(format!("0/{max_concurrent} active │ 0 errors"));
    pb.enable_steady_tick(Duration::from_millis(200));
    pb
}

/// Create a standalone stats bar (not managed by [`MultiProgress`]).
pub fn new_stats_bar(max_concurrent: usize) -> Option<ProgressBar> {
    if !is_tty() {
        return None;
    }

    let pb = ProgressBar::new(0);
    pb.set_style(ProgressStyle::with_template("{msg}").expect("invalid stats style template"));
    pb.set_message(format!("0/{max_concurrent} active │ 0 errors"));
    pb.enable_steady_tick(Duration::from_millis(200));
    Some(pb)
}

pub fn update_stats_message(pb: &ProgressBar, active: usize, max_concurrent: usize, errors: usize) {
    pb.set_message(format!("{active}/{max_concurrent} active │ {errors} errors"));
}

pub fn log_with_mp(mp: &Option<MultiProgress>, f: impl FnOnce()) {
    match mp {
        Some(mp) => mp.suspend(f),
        None => f(),
    }
}

pub fn new_multi_progress() -> Option<MultiProgress> {
    if !is_tty() {
        return None;
    }
    Some(MultiProgress::new())
}

pub fn populate_style() -> &'static str {
    POPULATE_STYLE
}

pub fn make_list_style() -> &'static str {
    MAKE_LIST_STYLE
}

pub fn download_aggregate_style() -> &'static str {
    DOWNLOAD_AGGREGATE_STYLE
}

pub struct PopulateStats {
    pub total: usize,
    pub succeeded: usize,
    pub retried: usize,
    pub recovered: usize,
    pub failed: usize,
    pub elapsed: Duration,
}

#[derive(Debug, Clone)]
pub struct FailedItem {
    pub path: String,
    pub version: Option<String>,
    pub error: String,
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else {
        let m = secs / 60;
        let s = secs % 60;
        format!("{m}m {s}s")
    }
}

pub fn print_populate_summary(
    mp: &Arc<Option<MultiProgress>>,
    stats: &PopulateStats,
    failures: &[FailedItem],
) {
    let lines = build_summary_lines(stats, failures);
    match mp.as_ref() {
        Some(mp) => {
            mp.suspend(|| {
                for line in &lines {
                    eprintln!("{line}");
                }
            });
        }
        None => {
            for line in &lines {
                eprintln!("{line}");
            }
            let _ = std::io::stderr().flush();
        }
    }
}

fn build_summary_lines(stats: &PopulateStats, failures: &[FailedItem]) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("── Populate Summary ─────────────────────".to_string());

    let mut parts = vec![format!("Total: {}", stats.total), format!("OK: {}", stats.succeeded)];
    if stats.retried > 0 {
        parts.push(format!("Retried: {}", stats.retried));
        parts.push(format!("Recovered: {}", stats.recovered));
    }
    if stats.failed > 0 {
        parts.push(format!("Failed: {}", stats.failed));
    }
    parts.push(format!("Time: {}", format_duration(stats.elapsed)));
    lines.push(parts.join(" │ "));

    if !failures.is_empty() {
        lines.push(String::new());
        lines.push("Failed files:".to_string());
        for f in failures {
            lines.push(format!("  ✗ {} ({})", f.path, f.error));
        }
    }

    lines
}
