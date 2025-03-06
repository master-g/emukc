use emukc_cache::kache;
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::{ApiManifest, ApiMstFurniture};

pub(super) async fn crawl(mst: &ApiManifest, cache: &kache::Kache) -> Result<(), kache::Error> {
	for entry in mst.api_mst_furniture.iter() {
		if entry.api_active_flag == 1 {
			fetch_scripts(entry, cache).await?;
			fetch_movable(entry, cache).await?;
		} else {
			fetch_normal(entry, cache).await?;
		}
	}

	Ok(())
}

async fn fetch_scripts(entry: &ApiMstFurniture, cache: &kache::Kache) -> Result<(), kache::Error> {
	let id = format!("{0:03}", entry.api_id);
	let key = SuffixUtils::create(&id, "furniture_scripts");
	let version = if entry.api_version > 1 {
		Some(entry.api_version.to_string())
	} else {
		None
	};
	let _ = cache
		.get(
			format!("kcs2/resources/furniture/scripts/{id}_{key}.json").as_str(),
			version.as_deref(),
		)
		.await?;

	Ok(())
}

async fn fetch_movable(entry: &ApiMstFurniture, cache: &kache::Kache) -> Result<(), kache::Error> {
	let id = format!("{0:03}", entry.api_id);
	let key = SuffixUtils::create(&id, "furniture_movable");
	let version = if entry.api_version > 1 {
		Some(entry.api_version.to_string())
	} else {
		None
	};
	for ext in ["json", "png"] {
		let _ = cache
			.get(
				format!("kcs2/resources/furniture/movable/{id}_{key}.{ext}").as_str(),
				version.as_deref(),
			)
			.await?;
	}

	Ok(())
}

async fn fetch_normal(entry: &ApiMstFurniture, cache: &kache::Kache) -> Result<(), kache::Error> {
	let id = format!("{0:03}", entry.api_id);
	let key = SuffixUtils::create(&id, "furniture_normal");
	let version = if entry.api_version > 1 {
		Some(entry.api_version.to_string())
	} else {
		None
	};
	let _ = cache
		.get(format!("kcs2/resources/furniture/normal/{id}_{key}.png").as_str(), version.as_deref())
		.await?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_furniture_script_magic() {
		let id = "006";
		let key = SuffixUtils::create(id, "furniture_scripts");
		assert_eq!(key, "8280");
	}

	#[test]
	fn test_furniture_movable() {
		let id = "006";
		let key = SuffixUtils::create(id, "furniture_movable");
		assert_eq!(key, "3938");
	}

	#[test]
	fn test_furniture_normal() {
		let id = "007";
		let key = SuffixUtils::create(id, "furniture_normal");
		assert_eq!(key, "5950");
	}
}
