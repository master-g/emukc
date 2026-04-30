## 1. Dependency Setup

- [x] 1.1 Add `indicatif` to workspace `Cargo.toml` with version pin
- [x] 1.2 Add `indicatif` dependency to `crates/emukc_bootstrap/Cargo.toml`

## 2. Progress Bar Infrastructure

- [x] 2.1 Create `crates/emukc_bootstrap/src/progress.rs` with a helper module: TTY detection (`std::io::stdout().is_terminal()`), `ProgressBar` style constants, and a `new_progress_bar(total, message)` function that returns `Option<ProgressBar>` (None if non-TTY)
- [x] 2.2 Create `new_multi_progress()` helper that returns `Option<MultiProgress>` (None if non-TTY)
- [x] 2.3 Register `progress` module in `crates/emukc_bootstrap/src/lib.rs`

## 3. Replace populate.rs Progress

- [x] 3.1 Replace `print_progress()` function with `indicatif::ProgressBar` — use `new_progress_bar(total_files, "Populating cache")` from progress module
- [x] 3.2 Replace `AtomicUsize` + `print_progress()` calls with `pb.inc(1)` inside the async task closure
- [x] 3.3 Replace `println!()` at end with `pb.finish_and_clear()` (or no-op if non-TTY)

## 4. Replace make_list/progress.rs

- [x] 4.1 Replace `ProgressTracker` struct with `indicatif::ProgressBar` — map `increment_checked` to `pb.inc(1)`, `increment_found` to `pb.set_message()`, `report()` to automatic bar refresh
- [x] 4.2 Style: show checked/total, found count, rate, ETA via `ProgressStyle::with_template`

## 5. Add MultiProgress to download.rs

- [x] 5.1 Wrap `download_all()` with a `MultiProgress` — add aggregate bar for resource count and per-resource spinners
- [x] 5.2 Wrap `download_web_assets()` with phase-labeled progress output using `MultiProgress::suspend()` for `info!/warn!` calls
- [x] 5.3 Ensure non-TTY fallback: skip all bar creation, let existing logs work as-is

## 6. Bootstrap Phase Labels

- [x] 6.1 Add phase headers to `bootstrap` command flow (Phase 1: Resources, Phase 2: Parse, Phase 3: Web Assets, Phase 4: Save) using `MultiProgress::suspend()` to print headers above bars
- [x] 6.2 Wire the progress bars from steps 3-5 into the bootstrap flow, ensuring phase transitions are visible

## 7. Verification

- [x] 7.1 Run `cargo build` — verify no compile errors
- [x] 7.2 Run `cargo clippy --workspace` — no new warnings
- [x] 7.3 Test non-TTY path: pipe `cache populate` output and verify no ANSI escape codes
