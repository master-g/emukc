use std::sync::LazyLock;

use emukc_cache::prelude::*;
use emukc_model::kc2::start2::ApiManifest;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::{
    make_list::CacheList,
    prelude::{CacheListMakeStrategy, CacheListMakingError},
};

/// Sub-gauge / phase variants for regular maps (e.g. 7-3's second gauge). These are not
/// expressible from `api_mst_mapinfo` alone, so they are listed explicitly. Bases come from the
/// manifest (see [`regular_gauge_ids`]); only these phase suffixes are maintained by hand.
pub(super) const REGULAR_GAUGE_VARIANT_IDS: &[&str] = &["00702_2", "00703_2", "00705_2", "00705_3"];

/// Base regular-map gauge ids derived from the manifest: every map carrying a defeat gauge
/// (`api_required_defeat_count`) or an HP gauge (`api_max_maphp`) yields `0{area:02}{no:02}`
/// (e.g. 5-6 → `00506`). Deriving from the manifest keeps the list current as the game adds
/// maps, instead of going stale like a hardcoded list. Phase variants are not included here —
/// [`make`] discovers them by bounded crawl.
pub(super) fn regular_gauge_base_ids(mst: &ApiManifest) -> Vec<String> {
    mst.api_mst_mapinfo
        .iter()
        .filter(|m| m.api_required_defeat_count.is_some() || m.api_max_maphp.is_some())
        .map(|m| format!("0{:02}{:02}", m.api_maparea_id, m.api_no))
        .collect()
}

/// Base gauge ids plus the known sub-gauge phase variants. Used by the template / fallback path,
/// which only lists json paths and cannot auto-discover variants by crawling the way [`make`]
/// does, so it keeps the explicit variant suffixes for completeness.
pub(super) fn regular_gauge_ids(mst: &ApiManifest) -> Vec<String> {
    let mut ids = regular_gauge_base_ids(mst);
    ids.extend(REGULAR_GAUGE_VARIANT_IDS.iter().map(|s| (*s).to_string()));
    ids
}

pub(super) static EVENT_MAP_ID_LIST: LazyLock<&[&str]> = LazyLock::new(|| {
    &[
        "03801", "03802", "03803", "03804", "03805", "03901", "04001", "04101", "04201", "04301",
        "04301_2", "04302", "04302_2", "04303", "04303_2", "04401", "04402", "04402_2", "04403",
        "04403_2", "04404", "04405", "04405_2", "04501", "04502", "04502_2", "04503", "04503_2",
        "04601", "04601_2", "04602", "04701", "04701_2", "04701_3", "04801", "04801_2", "04802",
        "04802_2", "04803", "04804", "04804_2", "04804_3", "04805", "04805_2", "04806", "04806_2",
        "04807", "04807_2", "04807_3", "04901", "04901_2", "04902", "04902_2", "04903", "04903_2",
        "04903_3", "04904", "04904_2", "04904_3", "05001", "05001_2", "05001_3", "05002",
        "05002_2", "05002_3", "05003", "05003_2", "05003_3", "05004", "05004_2", "05004_3",
        "05004_4", "05005", "05005_2", "05005_3", "05101", "05101_2", "05101_3", "05102",
        "05102_2", "05102_3", "05103", "05103_2", "05103_3", "05103_4", "05201", "05201_2",
        "05202", "05202_2", "05203", "05203_2", "05203_3", "05301", "05302", "05302_2", "05303",
        "05303_2", "05303_3", "05304", "05304_2", "05304_3", "05305", "05305_2", "05305_3",
        "05401", "05402", "05402_2", "05402_3", "05403", "05403_2", "05403_3", "05404", "05404_2",
        "05404_3", "05405", "05405_2", "05405_3", "05405_4", "05501", "05502", "05502_2", "05503",
        "05503_2", "05504", "05504_2", "05505", "05505_2", "05505_3", "05505_4", "05506",
        "05506_2", "05506_3", "05506_4", "05601", "05601_2", "05602", "05602_2", "05602_3",
        "05603", "05603_2", "05603_3", "05604", "05604_2", "05605", "05605_2", "05605_3", "05606",
        "05606_2", "05606_3", "05606_4", "05701", "05701_2", "05702", "05702_2", "05703",
        "05703_2", "05703_3", "05704", "05704_2", "05704_3", "05705", "05705_2", "05705_3",
        "05706", "05706_2", "05706_3", "05707", "05707_2", "05707_3", "05707_4", "05707_5",
        "05801", "05801_2", "05802", "05802_2", "05803", "05803_2", "05803_3", "05804", "05804_2",
        "05804_3", "05804_4", "05901", "05902", "05902_2", "05902_3", "05903", "05903_2",
        "05903_3", "05903_4", "05904", "05904_2", "05904_3", "05905", "05905_2", "05905_3",
        "05905_4", "06001", "06002", "06002_2", "06003", "06003_2", "06003_3", "06004", "06004_2",
        "06004_3", "06005_2", "06005_3", "06006", "06006_2", "06006_3",
    ]
});

#[derive(Debug, Serialize, Deserialize)]
struct GaugeConfig {
    img: String,
    vertical: VerticalConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerticalConfig {
    img: String,
}

pub(super) async fn make(
    mst: &ApiManifest,
    cache: &Kache,
    strategy: &CacheListMakeStrategy,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    if *strategy == CacheListMakeStrategy::Minimal {
        return Ok(());
    }

    // Regular maps: derive bases from the manifest, then bounded-crawl each base's phase variants.
    for base in regular_gauge_base_ids(mst) {
        make_gauge_with_variants(cache, &base, list).await?;
    }

    // Event maps: the historical list (with its variants) is authoritative — events leave the
    // manifest once they end, so they can't be derived.
    for id in *EVENT_MAP_ID_LIST {
        make_gauge_by_id(cache, id, list).await?;
    }

    Ok(())
}

/// Add a base gauge and bounded-crawl its phase variants (`{base}_2`, `{base}_3`, …), stopping at
/// the first absent variant. This auto-discovers new sub-gauges (e.g. a map's second gauge)
/// without a hardcoded variant list. Bounded to 8 probes per base.
async fn make_gauge_with_variants(
    cache: &Kache,
    base: &str,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    if !make_gauge_by_id(cache, base, list).await? {
        return Ok(());
    }
    for i in 2..=9 {
        let variant_id = format!("{base}_{i}");
        // Probe with a quiet HEAD first: most bases have no further phase, and routing the
        // expected 404 through the full `get` path would log it at ERROR. `exists_on_remote`
        // reports a missing file at trace level, so the crawl stays silent.
        let json_path = format!("kcs2/resources/gauge/{variant_id}.json");
        match cache.exists_on_remote(&json_path, NoVersion).await {
            RemoteExistence::Present => {}
            // The CDN says there is no further phase: this is the one case that
            // legitimately ends the crawl.
            RemoteExistence::Absent => break,
            // No CDN answered. Ending the crawl here would silently drop the rest
            // of this gauge series from a list that drives hours of downloading,
            // so fail the whole make-list instead.
            RemoteExistence::Indeterminate => {
                return Err(CacheListMakingError::Kache(KacheError::FailedOnAllCdn));
            }
        }
        make_gauge_by_id(cache, &variant_id, list).await?;
    }
    Ok(())
}

/// Add a gauge json and the images it references to the list. Tolerant of *absence*: a gauge whose
/// json the CDN reports as 404 (a map without a gauge resource) is skipped rather than aborting the
/// whole make-list, and a malformed json adds the json path but skips its (unknown) images. It is
/// not tolerant of an unanswered fetch — see the error arm below.
///
/// Returns `true` when the gauge json was found (so the variant crawl keeps probing), `false`
/// when it is absent (so the crawl stops).
async fn make_gauge_by_id(
    cache: &Kache,
    id: &str,
    list: &mut CacheList,
) -> Result<bool, CacheListMakingError> {
    let p = format!("kcs2/resources/gauge/{id}.json");
    let mut json_file = match GetOption::new_non_mod().get(cache, &p, NoVersion).await {
        Ok(file) => file,
        // The CDN answered 404: this gauge genuinely has no json, which is the
        // tolerated case above. (`new_non_mod` leaves remote enabled, so this
        // variant can only come from the CDN here, not from a local-only miss.)
        Err(KacheError::FileNotFound(_)) => {
            tracing::trace!("gauge {id}: json absent upstream, skipping");
            return Ok(false);
        }
        // No answer at all. Returning "absent" here would drop this gauge and its
        // images from the list silently — and at the variant call site it would
        // contradict a HEAD probe that just reported `Present`.
        Err(e) => return Err(e.into()),
    };
    list.add_unversioned(p);
    let mut raw = String::new();
    json_file.read_to_string(&mut raw).await.map_err(|e| KacheError::InvalidFile(e.to_string()))?;

    let config: GaugeConfig = match serde_json::from_str(&raw) {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!("gauge {id}: malformed json ({e}), skipping its images");
            return Ok(true);
        }
    };

    for img in [config.img, config.vertical.img] {
        list.add_unversioned(format!("kcs2/resources/gauge/{img}.png"));
        list.add_unversioned(format!("kcs2/resources/gauge/{img}_light.png"));
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A CDN address nothing listens on, so every probe fails to connect and
    /// `exists_on_remote` reports `Indeterminate` rather than `Absent`.
    const UNREACHABLE_CDN: &str = "http://127.0.0.1:1";

    const BASE_ID: &str = "00701";

    fn cache_with_local_base_gauge(root: &std::path::Path) -> Kache {
        let gauge_dir = root.join("kcs2/resources/gauge");
        std::fs::create_dir_all(&gauge_dir).unwrap();
        std::fs::write(
            gauge_dir.join(format!("{BASE_ID}.json")),
            br#"{"img":"gauge_a","vertical":{"img":"gauge_a_v"}}"#,
        )
        .unwrap();

        Kache::builder()
            .with_cache_root(root.to_path_buf())
            .with_content_cdn(UNREACHABLE_CDN.to_string())
            .with_gadgets_cdn(UNREACHABLE_CDN.to_string())
            .build()
            .unwrap()
    }

    /// The crawl must not read "no CDN answered" as "no further phase exists".
    /// Doing so silently truncates a gauge series from a list that drives hours
    /// of downloading, which is the defect this plan exists to remove.
    #[tokio::test]
    async fn variant_crawl_fails_instead_of_truncating_when_no_cdn_answers() {
        let root = tempfile::TempDir::new().unwrap();
        let cache = cache_with_local_base_gauge(root.path());
        let mut list = CacheList::new();

        let err = make_gauge_with_variants(&cache, BASE_ID, &mut list)
            .await
            .expect_err("an inconclusive probe must not be swallowed");

        assert!(
            matches!(err, CacheListMakingError::Kache(KacheError::FailedOnAllCdn)),
            "got {err:?}"
        );

        // The base json came from the fixture's local file (the fetch never went
        // out), so this only shows the crawl got past the base before giving up
        // on the variants — it says nothing about remote behaviour.
        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert!(paths.iter().any(|p| p.contains(&format!("gauge/{BASE_ID}.json"))));
    }

    /// The same defect one call deeper: when the base gauge's own fetch gets no
    /// answer, `make_gauge_by_id` used to report it as "absent" and the caller
    /// dropped the whole series without an error. Nothing is pre-seeded here, so
    /// the base lookup really does go out to the unreachable CDN.
    #[tokio::test]
    async fn base_gauge_fetch_failure_is_not_reported_as_absence() {
        let root = tempfile::TempDir::new().unwrap();
        let cache = Kache::builder()
            .with_cache_root(root.path().to_path_buf())
            .with_content_cdn(UNREACHABLE_CDN.to_string())
            .with_gadgets_cdn(UNREACHABLE_CDN.to_string())
            .build()
            .unwrap();
        let mut list = CacheList::new();

        let err = make_gauge_with_variants(&cache, BASE_ID, &mut list)
            .await
            .expect_err("an unanswered base fetch must not be swallowed as absence");

        assert!(
            matches!(err, CacheListMakingError::Kache(KacheError::FailedOnAllCdn)),
            "got {err:?}"
        );
        assert!(list.items.is_empty(), "nothing should have been collected");
    }
}
