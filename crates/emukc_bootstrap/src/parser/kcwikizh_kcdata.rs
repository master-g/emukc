use std::{
	fs::{self, DirEntry},
	path::Path,
};

use emukc_model::prelude::*;
use serde::{Deserialize, Serialize};

use super::error::ParseError;

fn parse_ship_info(
	src: impl AsRef<Path>,
	manifest: &ApiManifest,
) -> Result<Kc3rdShipExtraInfoMap, ParseError> {
	#[derive(Debug, Serialize, Deserialize)]
	struct KcWikiZhShip {
		id: i64,
		book_sinfo: String,
		cnum: i64,
		can_drop: bool,
		can_construct: bool,
	}

	#[derive(Debug, Serialize, Deserialize)]
	struct KcWikiZhShipYaml {
		data: KcWikiZhShip,
	}

	// paring ships
	let ship_dir = src.as_ref().to_path_buf().join("_ship");
	// walk _ship dir and parse all the files
	let dir = fs::read_dir(&ship_dir)?;
	let yaml_files: Vec<DirEntry> =
		dir.filter_map(Result::ok).filter(|entry| entry.path().is_file()).collect();

	let mut map: Kc3rdShipExtraInfoMap = Kc3rdShipExtraInfoMap::new();

	trace!("parsing ship files");
	for entry in yaml_files {
		let path = entry.path();
		let raw = fs::read_to_string(path)?;
		for doc in serde_yaml::Deserializer::from_str(&raw) {
			let ship = KcWikiZhShipYaml::deserialize(doc);
			if let Ok(ship) = ship {
				let mst = manifest.api_mst_ship.iter().find(|mst| mst.api_id == ship.data.id);
				match mst {
					Some(mst) => {
						if mst.api_aftershipid.is_none() {
							debug!("ship id {} is not an ally ship", ship.data.id);
							continue;
						}
					}
					None => {
						debug!("ship id {} not found in manifest", ship.data.id);
						continue;
					}
				}

				map.insert(
					ship.data.id,
					Kc3rdShipExtraInfo {
						api_id: ship.data.id,
						info: ship.data.book_sinfo.clone(),
						droppable: ship.data.can_drop,
						buildable: ship.data.can_construct,
					},
				);
			}
		}
	}
	trace!("{} ships parsed", map.len());

	Ok(map)
}

fn parse_ship_class(
	src: impl AsRef<Path>,
	manifest: &ApiManifest,
) -> Result<Kc3rdShipClassNameMap, ParseError> {
	#[derive(Debug, Serialize, Deserialize)]
	struct KcWikiZhShipClass {
		id: i64,
		name: String,
	}

	#[derive(Debug, Serialize, Deserialize)]
	struct KcWikiZhShipClassYaml {
		data: KcWikiZhShipClass,
	}

	// paring ship class
	let shipclass_dir = src.as_ref().to_path_buf().join("_shipclass");
	// walk _shipclass dir and parse all the files
	let dir = fs::read_dir(shipclass_dir)?;
	let yaml_files: Vec<DirEntry> =
		dir.filter_map(Result::ok).filter(|entry| entry.path().is_file()).collect();

	let mut map = Kc3rdShipClassNameMap::new();

	trace!("parsing ship class files");
	for entry in yaml_files {
		let path = entry.path();
		let raw = fs::read_to_string(path)?;
		for doc in serde_yaml::Deserializer::from_str(&raw) {
			let ship = KcWikiZhShipClassYaml::deserialize(doc);
			if let Ok(ship) = ship {
				let mst = manifest.api_mst_ship.iter().find(|m| m.api_ctype == ship.data.id);
				if mst.is_none() {
					warn!("ship class {:?} not found in manifest", ship.data);
				} else {
					map.insert(
						ship.data.id,
						Kc3rdShipClassNameInfo {
							api_id: ship.data.id,
							name: ship.data.name.clone(),
						},
					);
				}
			}
		}
	}
	trace!("{} ship class parsed", map.len());

	Ok(map)
}

/// Parse the kcwikizh kcdata.
///
/// # Arguments
///
/// * `src` - The path to the kcwikizh kcdata, must be directory, i.e. unzipped.
/// * `manifest` - The API manifest.
///
/// # Returns
///
/// A tuple of the ship extra info map, ship class name map, and slotitem extra info map.
pub fn parse(
	src: impl AsRef<Path>,
	manifest: &ApiManifest,
) -> Result<(Kc3rdShipExtraInfoMap, Kc3rdShipClassNameMap), ParseError> {
	let src = src.as_ref();
	trace!("parsing kcwikizh kcdata: {:?}", src);

	Ok((parse_ship_info(src, manifest)?, parse_ship_class(src, manifest)?))
}
