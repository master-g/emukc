---
title: "Cache resource history: keeping every version a bump replaces"
date: 2026-10-15
category: best-practices
module: emukc_cache
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "A cached resource gets a new version"
  - "Asking the cache for an older version than the one it holds"
  - "Keeping seasonal art or voices before a maintenance replaces them"
tags: [cache, kache, history, versioning, seasonal, cdn, season]
related_components: [emukc_cache]
---

# Cache resource history: keeping every version a bump replaces

## Context

The CDN keeps only the current file at each path. `?version=` only busts caches, and every
value returns the same object. Seasonal art is a new file uploaded over the old path. Once
the cache overwrote a file, the content it replaced was gone for good. Plan
`docs/plans/2026-10-15-007-feat-seasonal-archive-plan.md` (L0) adds a history so the
content can be served again later. Picking a season to serve (L1, below) builds on it.

## Rules

- **The live tree is unchanged.** `z/cache/<path>` is still the current version, so mods,
  `cache-list-oracle` and the headless check read it as before.
- **History rows sit next to the live file.** The table `kache_history` in `kache.redb` maps
  `(path, version)` to `(sha256, size, Last-Modified, kept at)`. The bytes live once per content
  under `z/cache/.history/blobs/<sha[..2]>/<sha>`, no matter how many versions or paths
  share them. For example, 2,684 snapshot rows took only 1,092 blobs.
- **Nothing is overwritten unkept.**
  - Before a download replaces the live file, the file is kept under its recorded version,
    inside the per-path download lock.
  - The new live file is not copied. It is kept when it in turn gets replaced, so the cache
    does not double on disk.
  - Writing the same content again keeps the row's existing `Last-Modified` and time.
  - Unversioned files (`""`, `"1"`) are never kept: nothing asks for them by version.
- **An older version is answered from history.** A request older than the recorded version
  gets the latest kept version not after it, or the earliest kept one when the request
  predates them all. Only when nothing is kept does the old rule apply: serve the newer live
  file and leave the recorded version alone. A request that queued on the download lock while
  another one moved the file past its version is answered the same way, not downloaded.
- **A version bump checks size before downloading.**
  - First a HEAD to the CDN, reading the `Content-Length` header (`content_length()` reads 0
    on a HEAD).
  - If the size equals the live file's, the new version is recorded against the live bytes
    and nothing is downloaded. `Last-Modified` is recorded too.
  - A live file with no recorded version is downloaded whatever its size.
  - Otherwise, or on a 404 or error, the GET runs as before. A real change costs one extra
    HEAD.
  - Remote-only lookups such as `version.json` skip the size check.
- **A snapshot can be imported.** `cargo run -- cache archive-import <dir>` keeps every file
  of `<dir>/manifest.json` under its version, with the manifest's `last_modified` when present.
  Each entry has `path`, `version`, `size`, `sha256` and optionally `last_modified`.
  - Before keeping anything, it checks every size and sha256, and rejects paths that are not
    plain relative paths.
  - It skips unversioned entries with a warning.
  - It leaves the live files and their versions alone.

## Seasons

- **A season is the state of a date.** `crates/emukc_bootstrap/assets/seasons.json` lists each
  by date id, a human label and the `kcwiki/kancolle-data` commit holding that date's start2. It
  is maintained by hand.
- `cargo run -- season sync` keeps each season's `api_mst_shipgraph` at
  `.data/seasons/<id>.json`, and skips a season already there. After a season's `start2_commit`
  changes, delete its file first.
- `season = "<id>"` in the config dresses the ships when the server starts. Every ship the
  season has takes that season's `api_version` and coordinates. File name, sort number and
  `api_sp_flag` stay today's. Ships that came after it keep today's entry. The client then asks
  for that date's versions, and the history above answers them.
  - Between 09-20 and 10-14, only `api_version` and the coordinates differed.
  - Unset, getData is exactly today's.
- A season's art is only as complete as the history. A file the cache never held before the
  CDN replaced it is lost. Its request falls back to the earliest kept version, or else to the
  live file.
- `make headless-check SCENARIO=season_2026_09_20` (and `season_2026_10_14`) photographs 吹雪 in
  the port. See `headless-client-check.md`.

## Why size alone

On 2026-10-14, 1,339 bumped files got a new `Last-Modified`. Of those, 672 had changed and
667 were byte-identical re-uploads. Size separated the two groups exactly. `Last-Modified`
cannot separate them, and it also differs by seconds to minutes between mirrors. The known
gap is an edit that keeps the same length: it would be missed. None has been seen yet. If
one turns up, a ranged GET of a few slices would catch it.

## Not done

- `Last-Modified` is recorded only when the size check's HEAD runs. The download path does
  not surface response headers.
- There is no command that back-fills `Last-Modified` for the files already cached.
- Snapshots of 2026-09-20 and 2026-10-14 sit in `z/seasons/`. Each holds what was live on
  that date, which is not necessarily the standard art; see the plan's Evidence.
