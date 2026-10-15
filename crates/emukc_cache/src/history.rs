//! Every version of a resource the cache has held, so a version bump never loses what it
//! replaces. The CDN keeps only the current file of each path, so what is not kept here is
//! gone.
//!
//! See `docs/plans/2026-10-15-007-feat-seasonal-archive-plan.md` (L0).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use emukc_crypto::hash::sha256_hex;
use redb::{ReadableDatabase, ReadableTable, TableDefinition};

use crate::{
    error::Error,
    kache::Kache,
    unified_rel_path,
    ver::{IntoVersion, cmp_version},
};

/// `(path, version)` → `(sha256, size, Last-Modified, kept at in unix seconds)`. The bytes live
/// in the blob directory under their sha256, once however many versions share them.
pub(crate) const HISTORY_TABLE: TableDefinition<(&str, &str), (&str, u64, Option<&str>, u64)> =
    TableDefinition::new("kache_history");

/// Tells apart the partial files of concurrent writers: the download lock is per path, and
/// different paths often share one content.
static PARTIAL_SEQ: AtomicU64 = AtomicU64::new(0);

impl Kache {
    fn blob_path(&self, sha: &str) -> PathBuf {
        self.history_root.join("blobs").join(&sha[..2]).join(sha)
    }

    /// Keep `file` as the content of `rel_path` at `version`, leaving the live file and its
    /// recorded version alone. Returns the sha256 of what was kept.
    ///
    /// A row kept again with the same content keeps the `Last-Modified` and time it already
    /// had when this call brings none.
    pub async fn keep_version(
        &self,
        rel_path: &str,
        version: &str,
        file: &Path,
        last_modified: Option<&str>,
    ) -> Result<String, Error> {
        if version.into_version().is_none() {
            return Err(Error::InvalidFileVersion(rel_path.to_owned()));
        }
        let data = tokio::fs::read(file).await?;
        let sha = sha256_hex(&data);
        let blob = self.blob_path(&sha);
        if !blob.exists() {
            if let Some(dir) = blob.parent() {
                tokio::fs::create_dir_all(dir).await?;
            }
            let seq = PARTIAL_SEQ.fetch_add(1, Ordering::Relaxed);
            let partial = blob.with_extension(format!("{}.{seq}.partial", std::process::id()));
            tokio::fs::write(&partial, &data).await?;
            // A concurrent writer of the same content may have landed first; replacing its
            // blob with identical bytes is harmless.
            tokio::fs::rename(&partial, &blob).await?;
        }

        let db = self.db.clone();
        let rel_path = unified_rel_path(rel_path);
        let version = version.to_owned();
        let size = data.len() as u64;
        let last_modified = last_modified.map(str::to_owned);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
        let row_sha = sha.clone();
        tokio::task::spawn_blocking(move || -> Result<(), Error> {
            let write_txn = db.begin_write()?;
            {
                let mut table = write_txn.open_table(HISTORY_TABLE)?;
                let key = (rel_path.as_str(), version.as_str());
                let kept = table.get(key)?.and_then(|row| {
                    let (kept_sha, _, kept_last_modified, kept_at) = row.value();
                    (kept_sha == row_sha).then(|| (kept_last_modified.map(str::to_owned), kept_at))
                });
                let (last_modified, kept_at) = match kept {
                    Some((kept_last_modified, kept_at)) => {
                        (last_modified.or(kept_last_modified), kept_at)
                    }
                    None => (last_modified, now),
                };
                table.insert(key, (row_sha.as_str(), size, last_modified.as_deref(), kept_at))?;
            }
            write_txn.commit()?;
            Ok(())
        })
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e)))??;

        Ok(sha)
    }

    /// The kept content of `rel_path` as of `version`: the latest record not after it, or the
    /// earliest record when `version` predates them all. Records whose blob is missing are
    /// passed over.
    pub(crate) async fn find_in_history(
        &self,
        rel_path: &str,
        version: &str,
    ) -> Result<Option<PathBuf>, Error> {
        let db = self.db.clone();
        let rel_path = unified_rel_path(rel_path);
        let records =
            tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>, Error> {
                let read_txn = db.begin_read()?;
                let table = read_txn.open_table(HISTORY_TABLE)?;
                let mut records = Vec::new();
                for row in table.range((rel_path.as_str(), "")..)? {
                    let (key, value) = row?;
                    let (path, version) = key.value();
                    if path != rel_path {
                        break;
                    }
                    records.push((version.to_owned(), value.value().0.to_owned()));
                }
                Ok(records)
            })
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))??;

        let kept: Vec<(String, PathBuf)> = records
            .into_iter()
            .map(|(version, sha)| (version, self.blob_path(&sha)))
            .filter(|(_, blob)| blob.exists())
            .collect();
        let not_after = kept
            .iter()
            .filter(|(kept, _)| cmp_version(kept, version).is_le())
            .max_by(|(a, _), (b, _)| cmp_version(a, b));
        let chosen = not_after.or_else(|| kept.iter().min_by(|(a, _), (b, _)| cmp_version(a, b)));
        Ok(chosen.map(|(_, blob)| blob.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn keeping_the_same_content_again_keeps_its_last_modified() {
        let dir = tempfile::tempdir().unwrap();
        let kache = Kache::builder()
            .with_cache_root(dir.path().to_path_buf())
            .with_gadgets_cdn("example.invalid".to_string())
            .with_content_cdn("example.invalid".to_string())
            .build()
            .unwrap();
        let file = dir.path().join("voice.mp3");
        std::fs::write(&file, b"voice").unwrap();

        let lm = "Wed, 14 Oct 2026 11:23:42 GMT";
        kache.keep_version("kcs/sound/x/1.mp3", "1014", &file, Some(lm)).await.unwrap();
        // As `keep_live` does before the next bump: the same bytes, nothing about them.
        kache.keep_version("kcs/sound/x/1.mp3", "1014", &file, None).await.unwrap();

        let read_txn = kache.db.begin_read().unwrap();
        let table = read_txn.open_table(HISTORY_TABLE).unwrap();
        let row = table.get(("kcs/sound/x/1.mp3", "1014")).unwrap().unwrap();
        assert_eq!(row.value().2, Some(lm));
    }

    #[tokio::test]
    async fn an_unversioned_file_is_not_kept() {
        let dir = tempfile::tempdir().unwrap();
        let kache = Kache::builder()
            .with_cache_root(dir.path().to_path_buf())
            .with_gadgets_cdn("example.invalid".to_string())
            .with_content_cdn("example.invalid".to_string())
            .build()
            .unwrap();
        let file = dir.path().join("a.png");
        std::fs::write(&file, b"png").unwrap();
        assert!(kache.keep_version("a.png", "1", &file, None).await.is_err());
    }
}
