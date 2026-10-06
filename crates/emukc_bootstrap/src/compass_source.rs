//! Fetches the pinned source of the compass simulator, the upstream of the routing rules.
//!
//! The simulator (`X-20A/X-20A.github.io`, branch `compass_dev`, MIT) encodes every map's
//! branching as one TypeScript function per map. Only a pinned commit is ever fetched, so the
//! asset converted from it is reproducible; upgrading means changing
//! [`COMPASS_SOURCE_COMMIT`] and re-running the conversion.

use std::path::{Path, PathBuf};

use emukc_network::{client::new_reqwest_client, download::Request};

use crate::download::BootstrapDownloadError;

/// The repository holding the compass simulator.
pub const COMPASS_SOURCE_REPO: &str = "X-20A/X-20A.github.io";

/// The `compass_dev` commit the routing rules are converted from.
pub const COMPASS_SOURCE_COMMIT: &str = "4f32c40e70d5d7361ef589b0cabed08ccdf20cd9";

const COMPASS_SOURCE_ROOT: &str = "x20a_compass";
const EXPECTED_LICENSE: &str = "MIT License";

/// The archive URL of one commit of the compass simulator repository.
pub fn compass_source_archive_url(commit: &str) -> String {
    format!("https://github.com/{COMPASS_SOURCE_REPO}/archive/{commit}.zip")
}

/// Where the pinned commit is unpacked under `data_root`.
pub fn compass_source_dir(data_root: impl AsRef<Path>) -> PathBuf {
    data_root.as_ref().join(COMPASS_SOURCE_ROOT).join(COMPASS_SOURCE_COMMIT)
}

/// Fetch the pinned commit into [`compass_source_dir`], unless it is already there.
///
/// Returns the directory and whether anything was downloaded.
#[expect(clippy::result_large_err)]
pub async fn sync_compass_source(
    data_root: impl AsRef<Path>,
    proxy: Option<&str>,
) -> Result<(PathBuf, bool), BootstrapDownloadError> {
    let target = compass_source_dir(&data_root);
    if target.exists() {
        return Ok((target, false));
    }

    let root = data_root.as_ref().join(COMPASS_SOURCE_ROOT);
    std::fs::create_dir_all(&root)?;
    let archive = root.join(format!("{COMPASS_SOURCE_COMMIT}.zip"));
    let url = compass_source_archive_url(COMPASS_SOURCE_COMMIT);

    let client = new_reqwest_client(proxy, None).map_err(|source| {
        BootstrapDownloadError::ReqwestClient {
            proxy: proxy.map(ToOwned::to_owned),
            source,
        }
    })?;
    Request::builder()
        .url(&url)
        .save_as(&archive)
        .overwrite(true)
        .skip_header_check(true)
        .build()?
        .execute(Some(client))
        .await?;

    unpack_compass_archive(&url, &archive, &target)?;
    std::fs::remove_file(&archive)?;
    Ok((target, true))
}

/// Unpack `archive` into `target`, which only appears once the unpacked tree passed the
/// license check — an interrupted or rejected unpack leaves no `target` behind.
#[expect(clippy::result_large_err)]
fn unpack_compass_archive(
    url: &str,
    archive: &Path,
    target: &Path,
) -> Result<(), BootstrapDownloadError> {
    let save_as = archive.display().to_string();
    let unzip_error = |action, source| BootstrapDownloadError::Unzip {
        url: url.to_owned(),
        save_as: save_as.clone(),
        action,
        source,
    };

    let staging = target.with_extension("partial");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;

    let file = std::fs::File::open(archive)?;
    zip::ZipArchive::new(file)
        .map_err(|source| unzip_error("reading zip archive", source))?
        .extract_unwrapped_root_dir(&staging, zip::read::root_dir_common_filter)
        .map_err(|source| unzip_error("extracting zip archive", source))?;

    check_license(&staging)?;
    std::fs::rename(&staging, target)?;
    Ok(())
}

/// The conversion redistributes rules derived from this source, which the MIT license
/// allows. A different license at the pinned commit has to be looked at by a person.
#[expect(clippy::result_large_err)]
fn check_license(dir: &Path) -> Result<(), BootstrapDownloadError> {
    let path = dir.join("LICENSE");
    let license = std::fs::read_to_string(&path).map_err(|err| {
        BootstrapDownloadError::Generic(format!(
            "compass source has no readable LICENSE at {}: {err}",
            path.display()
        ))
    })?;
    let first_line = license.lines().next().unwrap_or_default().trim();
    if first_line != EXPECTED_LICENSE {
        return Err(BootstrapDownloadError::Generic(format!(
            "compass source license is {first_line:?}, expected {EXPECTED_LICENSE:?}; \
             review it before converting rules from this commit"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn write_archive(path: &Path, license: &str) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("X-20A.github.io-abc/LICENSE", options).unwrap();
        zip.write_all(license.as_bytes()).unwrap();
        zip.start_file("X-20A.github.io-abc/src/core/branch/world1/1-1.ts", options).unwrap();
        zip.write_all(b"export const calc_1_1 = () => '1';").unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn archive_url_names_the_commit() {
        assert_eq!(
            compass_source_archive_url("abc123"),
            "https://github.com/X-20A/X-20A.github.io/archive/abc123.zip"
        );
    }

    #[tokio::test]
    async fn present_source_is_not_fetched_again() {
        let root = tempfile::tempdir().unwrap();
        let target = compass_source_dir(root.path());
        std::fs::create_dir_all(&target).unwrap();

        // An unusable proxy would fail any request, so success means none was made.
        let (dir, downloaded) =
            sync_compass_source(root.path(), Some("not a proxy url")).await.unwrap();

        assert_eq!(dir, target);
        assert!(!downloaded);
    }

    #[test]
    fn unpack_strips_the_archive_root_directory() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("source.zip");
        let target = root.path().join("unpacked");
        write_archive(&archive, "MIT License\n\nCopyright (c) 2025 X-20A\n");

        unpack_compass_archive("url", &archive, &target).unwrap();

        assert!(target.join("LICENSE").is_file());
        assert!(target.join("src/core/branch/world1/1-1.ts").is_file());
        assert!(!target.with_extension("partial").exists());
    }

    #[test]
    fn unpack_rejects_a_changed_license_and_leaves_no_target() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("source.zip");
        let target = root.path().join("unpacked");
        write_archive(&archive, "GNU GENERAL PUBLIC LICENSE\n");

        let err = unpack_compass_archive("url", &archive, &target).unwrap_err();

        assert!(err.to_string().contains("GNU GENERAL PUBLIC LICENSE"), "{err}");
        assert!(!target.exists());
    }
}
