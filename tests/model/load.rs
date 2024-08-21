//! A test for loading a JSON file into a model.

use std::str::FromStr;

use emukc::prelude::*;

fn main() {
	let json_path = ".data/temp/start2.json";
	let raw = std::fs::read_to_string(json_path).unwrap();
	let manifest: ApiManifest = ApiManifest::from_str(&raw).unwrap();

	let pretty_ship_json = serde_json::to_string_pretty(&manifest.api_mst_ship).unwrap();
	println!("{}", pretty_ship_json);
}
