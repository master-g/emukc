use std::collections::BTreeMap;

use emukc_cache::{GetOption, Kache, NoVersion};
use emukc_model::kc2::start2::ApiManifest;
use tokio::io::AsyncReadExt;

use crate::{
    make_list::CacheList,
    prelude::{CacheListMakeStrategy, CacheListMakingError},
};

mod img;

pub(super) async fn make(
    mst: &ApiManifest,
    cache: &Kache,
    strategy: &CacheListMakeStrategy,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    // Force remote fetch to ensure we always get the latest version.json.
    // Using NoVersion is intentional here — we want whatever the CDN serves,
    // not a version-gated cache lookup.
    let mut version_file =
        GetOption::new_remote_only().get(cache, "kcs2/version.json", NoVersion).await?;
    let mut raw: String = String::new();
    version_file.read_to_string(&mut raw).await?;
    trace!("version.json fetched from remote, {} bytes", raw.len());
    let version_info: BTreeMap<String, String> = serde_json::from_str(&raw)?;

    img::make(mst, cache, &version_info, strategy, list).await?;

    Ok(())
}
