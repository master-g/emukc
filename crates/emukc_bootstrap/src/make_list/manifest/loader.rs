use std::path::PathBuf;

use super::types::{MANIFEST_VERSION, ResourceManifest};
use crate::prelude::CacheListMakingError;

const MANIFEST_FILE: &str = "resource_manifest.json";

fn manifest_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("assets");
    p.push(MANIFEST_FILE);
    p
}

pub(crate) fn load_resource_manifest() -> Result<ResourceManifest, CacheListMakingError> {
    let path = manifest_path();

    if !path.exists() {
        return Err(CacheListMakingError::Other(format!(
            "Resource manifest not found: {:?}\nRun `bun run decode -- --sync-resource-manifest` to generate it.",
            path
        )));
    }

    let raw = std::fs::read_to_string(&path).map_err(|e| {
        CacheListMakingError::Other(format!("Failed to read resource manifest: {e}"))
    })?;

    let manifest: ResourceManifest = serde_json::from_str(&raw).map_err(|e| {
        CacheListMakingError::Other(format!("Failed to parse resource manifest: {e}"))
    })?;

    if manifest.version != MANIFEST_VERSION {
        warn!(
            "Resource manifest version mismatch: expected {}, got {}. Proceeding anyway.",
            MANIFEST_VERSION, manifest.version
        );
    }

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_real_manifest() {
        let result = load_resource_manifest();
        assert!(result.is_ok(), "Should load the real resource_manifest.json");
        let manifest = result.unwrap();
        assert_eq!(manifest.version, 1);
        assert!(!manifest.entries.is_empty());
    }
}
