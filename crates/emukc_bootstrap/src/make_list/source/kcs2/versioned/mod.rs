use std::collections::BTreeMap;

use emukc_cache::{GetOption, Kache, NoVersion};
use emukc_model::kc2::start2::ApiManifest;
use tokio::io::AsyncReadExt;

use crate::{
    make_list::CacheList,
    prelude::{CacheListMakeStrategy, CacheListMakingError},
};

mod img;

/// Parse `kcs2/version.json` into a flat string map.
///
/// Upstream added a nested `resources` object (observed 2026-08, e.g.
/// `resources.map.621`). Nested objects are flattened with dotted keys so
/// every version stays addressable; non-string scalars are ignored.
fn parse_version_info(raw: &str) -> Result<BTreeMap<String, String>, serde_json::Error> {
    fn walk(prefix: &str, value: serde_json::Value, out: &mut BTreeMap<String, String>) {
        match value {
            serde_json::Value::String(s) => {
                out.insert(prefix.to_owned(), s);
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    let key = if prefix.is_empty() {
                        k
                    } else {
                        format!("{prefix}.{k}")
                    };
                    walk(&key, v, out);
                }
            }
            _ => {}
        }
    }

    let raw_map: BTreeMap<String, serde_json::Value> = serde_json::from_str(raw)?;
    let mut flat = BTreeMap::new();
    for (k, v) in raw_map {
        walk(&k, v, &mut flat);
    }
    Ok(flat)
}

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
    let version_info = parse_version_info(&raw)?;

    img::make(mst, cache, &version_info, strategy, list).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_version_info;

    #[test]
    fn parses_flat_and_nested_version_entries() {
        let raw = r#"{
            "title": "6.1.7.0",
            "common": "6.3.2.1",
            "resources": {
                "map": {
                    "621": "6.3.4.0",
                    "622": "6.3.4.0"
                }
            }
        }"#;

        let info = parse_version_info(raw).unwrap();
        assert_eq!(info.get("common").map(String::as_str), Some("6.3.2.1"));
        assert_eq!(info.get("resources.map.621").map(String::as_str), Some("6.3.4.0"));
        assert_eq!(info.len(), 4);
    }

    #[test]
    fn parses_legacy_flat_version_entries() {
        let raw = r#"{"common": "6.3.0.0", "port": "6.1.0.4"}"#;
        let info = parse_version_info(raw).unwrap();
        assert_eq!(info.get("common").map(String::as_str), Some("6.3.0.0"));
        assert_eq!(info.get("port").map(String::as_str), Some("6.1.0.4"));
    }
}
