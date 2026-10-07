//! Fetches the pinned `KC3Kai` files the equipment bonus table is converted from.
//!
//! `KC3Kai` (`KC3Kai/KC3Kai`, MIT) keeps the visible equipment bonuses as one declarative
//! table. Only a pinned commit is ever fetched, so the asset converted from it is
//! reproducible; upgrading means changing [`KC3KAI_SOURCE_COMMIT`] and re-running the
//! conversion.

use std::path::{Path, PathBuf};

use emukc_network::{client::new_reqwest_client, download::Request};

use crate::download::BootstrapDownloadError;

/// The repository holding `KC3Kai`.
pub const KC3KAI_SOURCE_REPO: &str = "KC3Kai/KC3Kai";

/// The commit the equipment bonus table is converted from.
pub const KC3KAI_SOURCE_COMMIT: &str = "ee4d7dfb1461723ff2bd0722a290d4e4c2e90d83";

/// The bonus table.
pub const KC3KAI_GEAR_BONUS_PATH: &str = "src/library/objects/GearBonus.js";

/// The ship class to nation table the bonus table refers to.
pub const KC3KAI_META_PATH: &str = "src/library/modules/Meta.js";

const KC3KAI_SOURCE_ROOT: &str = "kc3kai";
const LICENSE_PATH: &str = "LICENSE";
const EXPECTED_LICENSE: &str = "The MIT License (MIT)";

/// Where the pinned files are kept under `data_root`, at their paths in the repository.
pub fn kc3kai_source_dir(data_root: impl AsRef<Path>) -> PathBuf {
    data_root.as_ref().join(KC3KAI_SOURCE_ROOT).join(KC3KAI_SOURCE_COMMIT)
}

/// Fetch the pinned files into [`kc3kai_source_dir`], unless they are already there.
///
/// Returns the directory and whether anything was downloaded.
#[expect(clippy::result_large_err)]
pub async fn sync_kc3kai_source(
    data_root: impl AsRef<Path>,
    proxy: Option<&str>,
) -> Result<(PathBuf, bool), BootstrapDownloadError> {
    let target = kc3kai_source_dir(&data_root);
    let mut downloaded = false;

    // The license goes first: nothing else is fetched from a commit whose terms changed.
    for path in [LICENSE_PATH, KC3KAI_GEAR_BONUS_PATH, KC3KAI_META_PATH] {
        let save_as = target.join(path);
        if !save_as.exists() {
            let client = new_reqwest_client(proxy, None).map_err(|source| {
                BootstrapDownloadError::ReqwestClient {
                    proxy: proxy.map(ToOwned::to_owned),
                    source,
                }
            })?;
            Request::builder()
                .url(format!(
                    "https://raw.githubusercontent.com/{KC3KAI_SOURCE_REPO}/{KC3KAI_SOURCE_COMMIT}/{path}"
                ))
                .save_as(&save_as)
                .overwrite(true)
                .skip_header_check(true)
                .build()?
                .execute(Some(client))
                .await?;
            downloaded = true;
        }
        if path == LICENSE_PATH {
            check_license(&save_as)?;
        }
    }
    Ok((target, downloaded))
}

/// The conversion redistributes a table derived from this source, which the MIT license
/// allows. A different license at the pinned commit has to be looked at by a person.
#[expect(clippy::result_large_err)]
fn check_license(path: &Path) -> Result<(), BootstrapDownloadError> {
    let license = std::fs::read_to_string(path)?;
    let first_line = license.lines().next().unwrap_or_default().trim();
    if first_line != EXPECTED_LICENSE {
        return Err(BootstrapDownloadError::Generic(format!(
            "KC3Kai license is {first_line:?}, expected {EXPECTED_LICENSE:?}; \
             review it before converting the bonus table from this commit"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn present_files_are_not_fetched_again() {
        let root = tempfile::tempdir().unwrap();
        let dir = kc3kai_source_dir(root.path());
        for path in [LICENSE_PATH, KC3KAI_GEAR_BONUS_PATH, KC3KAI_META_PATH] {
            let file = dir.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, format!("{EXPECTED_LICENSE}\n")).unwrap();
        }

        assert_eq!(sync_kc3kai_source(root.path(), None).await.unwrap(), (dir, false));
    }

    #[tokio::test]
    async fn another_license_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let license = kc3kai_source_dir(root.path()).join(LICENSE_PATH);
        std::fs::create_dir_all(license.parent().unwrap()).unwrap();
        std::fs::write(&license, "GNU GENERAL PUBLIC LICENSE\n").unwrap();

        let err = sync_kc3kai_source(root.path(), None).await.unwrap_err();
        assert!(err.to_string().contains("review it"), "{err}");
    }
}
