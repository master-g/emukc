//! A test for loading a JSON file into a model.

use std::{io::BufRead, path::Path, str::FromStr};

use emukc::prelude::*;

fn main() {
	let json_path = ".data/temp/start2.json";
	let raw = std::fs::read_to_string(json_path).unwrap();
	let manifest: ApiManifest = ApiManifest::from_str(&raw).unwrap();

	// let graphs = load_ship_event_graphs(".data/temp/event_ship_graph.txt");
	let graphs = load_ship_event_graphs(".data/temp/enemy_ship_graph.txt");
	// let holes = find_holes(&manifest, &graphs, |graph| graph.api_id >= 5000);
	let holes =
		find_holes(&manifest, &graphs, |graph| graph.api_sortno.is_none() && graph.api_id < 5000);

	println!("{:?}", holes);
}

struct GraphMaps {
	full: Vec<i64>,
	full_dmg: Vec<i64>,
	up: Vec<i64>,
	up_dmg: Vec<i64>,
}

fn load_ship_event_graphs(path: impl AsRef<Path>) -> GraphMaps {
	let txt_file = std::fs::OpenOptions::new().read(true).open(path.as_ref()).unwrap();
	let reader = std::io::BufReader::new(txt_file);

	let mut result = GraphMaps {
		full: Vec::new(),
		full_dmg: Vec::new(),
		up: Vec::new(),
		up_dmg: Vec::new(),
	};

	for line in reader.lines() {
		let line = line.unwrap();
		let parts: Vec<&str> = line.split('/').collect();
		let category = parts[0];
		let id: i64 = parts[1].parse().unwrap();

		match category {
			"full" => {
				result.full.push(id);
			}
			"full_dmg" => {
				result.full_dmg.push(id);
			}
			"up" => {
				result.up.push(id);
			}
			"up_dmg" => {
				result.up_dmg.push(id);
			}
			_ => {
				panic!("Unknown category: {}", category);
			}
		}
	}

	result
}

#[derive(Debug)]
struct Holes {
	full: Vec<i64>,
	full_dmg: Vec<i64>,
	up: Vec<i64>,
	up_dmg: Vec<i64>,
}

fn find_holes<F>(mst: &ApiManifest, maps: &GraphMaps, cond: F) -> Holes
where
	F: Fn(&ApiMstShipgraph) -> bool,
{
	let mut result = Holes {
		full: Vec::new(),
		full_dmg: Vec::new(),
		up: Vec::new(),
		up_dmg: Vec::new(),
	};

	for graph in mst.api_mst_shipgraph.iter() {
		if !cond(graph) {
			continue;
		}

		if !maps.full.contains(&graph.api_id) {
			result.full.push(graph.api_id);
		}

		if !maps.full_dmg.contains(&graph.api_id) {
			result.full_dmg.push(graph.api_id);
		}

		if !maps.up.contains(&graph.api_id) {
			result.up.push(graph.api_id);
		}

		if !maps.up_dmg.contains(&graph.api_id) {
			result.up_dmg.push(graph.api_id);
		}
	}

	result
}
