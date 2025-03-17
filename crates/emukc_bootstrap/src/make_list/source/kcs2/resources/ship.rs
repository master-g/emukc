use std::sync::{Arc, LazyLock};

use emukc_cache::{IntoVersion, Kache, KacheError};
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
	make_list::{CacheList, CacheListMakeStrategy, batch_check_exists},
	prelude::CacheListMakingError,
};

pub(super) async fn make(
	mst: &ApiManifest,
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	make_non_graph(mst, list);
	make_friend_graph(mst, list);
	match strategy {
		CacheListMakeStrategy::Default => {
			make_friend_event_graph(mst, list);
			make_enemy_graph(mst, list);
			make_ship_special(mst, list);
			make_sp_remodel(mst, list);
			make_ship_type(mst, list);
		}
		CacheListMakeStrategy::Greedy(concurrent) => {
			make_ship_special_greedy(mst, cache, concurrent, list).await?;
			make_friend_event_graph_greedy(mst, cache, concurrent, list).await?;
			make_enemy_graph_greedy(mst, cache, concurrent, list).await?;
			make_sp_remodel_greedy(mst, cache, concurrent, list).await?;
			make_ship_type_greedy(mst, cache, concurrent, list).await?
		}
	};

	// make_friend_event_graph_greedy(mst, cache, list).await?;
	// make_enemy_graph(mst, cache, list).await?;

	Ok(())
}

fn make_non_graph(mst: &ApiManifest, list: &mut CacheList) {
	for ship in mst.api_mst_ship.iter() {
		let categories = if ship.api_aftershipid.is_none() {
			vec!["banner", "banner3", "banner3_g_dmg"]
		} else {
			vec![
				"album_status",
				"banner",
				"banner2",
				"banner2_dmg",
				"banner2_g_dmg",
				"banner_dmg",
				"banner_g_dmg",
				"card",
				"card_dmg",
				"power_up",
				"remodel",
				"remodel_dmg",
				"supply_character",
				"supply_character_dmg",
			]
		};

		let ship_id = format!("{0:04}", ship.api_id);

		let graph = mst.api_mst_shipgraph.iter().find(|v| v.api_id == ship.api_id);
		let version = graph.map(|v| v.api_version.first()).flatten();

		for category in categories {
			list.add(
				format!(
					"kcs2/resources/ship/{category}/{ship_id}_{}.png",
					SuffixUtils::create(&ship_id, format!("ship_{category}").as_str())
				),
				version,
			);
		}
	}
}

fn make_friend_graph(mst: &ApiManifest, list: &mut CacheList) {
	for graph in mst.api_mst_shipgraph.iter() {
		let version = graph.api_version.first().into_version();
		if version.is_none() {
			continue;
		}

		if graph.api_sortno.is_none() {
			continue;
		}

		let ship_id = format!("{0:04}", graph.api_id);

		for category in ["full", "full_dmg"] {
			list.add(
				format!(
					"kcs2/resources/ship/{category}/{ship_id}_{}_{}.png",
					SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
					graph.api_filename
				),
				version.as_ref(),
			);
		}

		for category in ["character_full", "character_full_dmg", "character_up", "character_up_dmg"]
		{
			list.add(
				format!(
					"kcs2/resources/ship/{category}/{ship_id}_{}.png",
					SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
				),
				version.as_ref(),
			);
		}
	}
}

#[allow(unused)]
async fn make_friend_event_graph_greedy(
	mst: &ApiManifest,
	kache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), KacheError> {
	let checks: Vec<(String, String)> = mst
		.api_mst_shipgraph
		.iter()
		.filter(|v| v.api_id > 5000)
		.flat_map(|v| {
			let ship_id = format!("{0:04}", v.api_id);
			["character_full", "character_full_dmg", "character_up", "character_up_dmg"].map(
				|category| {
					(
						format!(
							"kcs2/resources/ship/{category}/{ship_id}_{}.png",
							SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
						),
						v.api_version.first().cloned().unwrap_or_default(),
					)
				},
			)
		})
		.collect();

	let c = Arc::new(kache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, v), exists) in check_result {
		if exists {
			println!("{}, {}", p, v);
			list.add(p, v);
		}
	}

	Ok(())
}

struct ShipEventHoles {
	full: Vec<i64>,
	full_dmg: Vec<i64>,
	up: Vec<i64>,
	up_dmg: Vec<i64>,
}

static EVENT_SHIP_HOLES: LazyLock<ShipEventHoles> = LazyLock::new(|| ShipEventHoles {
	full: vec![
		5358, 5514, 5526, 5527, 5530, 5531, 5532, 5534, 5536, 5848, 5849, 5850, 5851, 5852, 5853,
		5963, 5964,
	],
	full_dmg: vec![
		5026, 5027, 5275, 5276, 5277, 5278, 5279, 5280, 5281, 5282, 5283, 5284, 5285, 5286, 5287,
		5288, 5289, 5290, 5291, 5292, 5293, 5294, 5295, 5296, 5297, 5298, 5299, 5300, 5301, 5302,
		5303, 5304, 5305, 5306, 5358, 5514, 5526, 5527, 5530, 5531, 5532, 5534, 5536, 5667, 5668,
		5669, 5848, 5849, 5850, 5851, 5852, 5853, 5963, 5964,
	],
	up: vec![
		5358, 5514, 5526, 5527, 5530, 5531, 5532, 5534, 5536, 5848, 5849, 5850, 5851, 5852, 5853,
		5963, 5964,
	],
	up_dmg: vec![
		5026, 5027, 5275, 5276, 5277, 5278, 5279, 5280, 5281, 5282, 5283, 5284, 5285, 5286, 5287,
		5288, 5289, 5290, 5291, 5292, 5293, 5294, 5295, 5296, 5297, 5298, 5299, 5300, 5301, 5302,
		5303, 5304, 5305, 5306, 5358, 5514, 5526, 5527, 5530, 5531, 5532, 5534, 5536, 5667, 5668,
		5669, 5848, 5849, 5850, 5851, 5852, 5853, 5963, 5964,
	],
});

fn make_friend_event_graph(mst: &ApiManifest, list: &mut CacheList) {
	let holes = &EVENT_SHIP_HOLES;
	for graph in mst.api_mst_shipgraph.iter() {
		if graph.api_id < 5000 {
			continue;
		}

		let ship_id = format!("{0:04}", graph.api_id);

		let mut categories: Vec<String> = Vec::new();

		if !holes.full.contains(&graph.api_id) {
			categories.push("character_full".to_string());
		}

		if !holes.full_dmg.contains(&graph.api_id) {
			categories.push("character_full_dmg".to_string());
		}

		if !holes.up.contains(&graph.api_id) {
			categories.push("character_up".to_string());
		}

		if !holes.up_dmg.contains(&graph.api_id) {
			categories.push("character_up_dmg".to_string());
		}

		for category in categories.iter() {
			list.add(
				format!(
					"kcs2/resources/ship/{category}/{ship_id}_{}.png",
					SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
				),
				graph.api_version.first(),
			);
		}
	}
}

#[allow(unused)]
async fn make_enemy_graph_greedy(
	mst: &ApiManifest,
	kache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), KacheError> {
	let checks: Vec<(String, String)> = mst
		.api_mst_shipgraph
		.iter()
		.filter(|v| v.api_sortno.is_none() && v.api_id < 5000)
		.flat_map(|v| {
			let ship_id = format!("{0:04}", v.api_id);
			["full", "full_dmg"].map(|category| {
				(
					format!(
						"kcs2/resources/ship/{category}/{ship_id}_{}_{}.png",
						SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
						v.api_filename,
					),
					v.api_version.first().cloned().unwrap_or_default(),
				)
			})
		})
		.collect();

	let c = Arc::new(kache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, v), exists) in check_result {
		if exists {
			println!("{}, {}", p, v);
			list.add(p, v);
		}
	}

	Ok(())
}

static ENEMY_SHIP_HOLES: LazyLock<ShipEventHoles> = LazyLock::new(|| ShipEventHoles {
	full: vec![
		1563, 1568, 1569, 1580, 1593, 1596, 1846, 1847, 1848, 1849, 1850, 1851, 1852, 1853, 1854,
		1855, 1856, 1857, 1858, 1859, 1860, 1861, 1862, 1863, 1864, 1865, 1866, 1867, 1868, 1869,
		1870, 1871, 1872, 1873, 1874, 1875, 1876, 1877, 1878, 1879, 1880, 1881, 1882, 1883, 1884,
		1885, 1886, 1887, 1888, 1889, 1890, 1891, 1892, 1893, 1894, 1895, 1896, 1897, 1898, 1899,
		1900, 1901, 1902, 1903, 1904, 1905, 1906, 1907, 1908, 1909, 1910, 1911, 1912, 1913, 1914,
		1915, 1916, 1917, 1918, 1919, 1920, 1921, 1922, 1923, 1924, 1925, 1926, 1927, 1928, 1929,
		1930, 1931, 1932, 1933, 1934, 1935, 1936, 1937, 1938, 1939, 1940, 1941, 1942, 1943, 1944,
		1945, 1946, 1947, 1948, 1949, 1950, 1951, 1952, 1953, 1954, 1955, 1956, 1957, 1958, 1959,
		1960, 1961, 1962, 1963, 1964, 1965, 1966, 1967, 1968, 1969, 1970, 1971, 1972, 1973, 1974,
		1975, 1976, 1977, 1978, 1979, 1980, 1981, 1982, 1983, 1984, 1985, 1986, 1987, 1988, 1989,
		1990, 1991, 1992, 1993, 1994, 1995, 1996, 1997, 1998, 1999, 2000, 2001, 2002, 2003, 2004,
		2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019,
		2020, 2021, 2022, 2023, 2024, 2025, 2026, 2027, 2028, 2029, 2030, 2031, 2032, 2033, 2034,
		2035, 2036, 2037, 2038, 2039, 2040, 2041, 2042, 2043, 2044, 2045, 2046, 2047, 2048, 2049,
		2050, 2051, 2052, 2053, 2054, 2055, 2056, 2057, 2058, 2059, 2060, 2061, 2062, 2063, 2064,
		2065, 2066, 2067, 2068, 2069, 2070, 2071, 2072, 2073, 2074, 2075, 2076, 2077, 2078, 2079,
		2080, 2081, 2082, 2083, 2084, 2085, 2086, 2087, 2088, 2089, 2090, 2091, 2092, 2093, 2094,
		2095, 2096, 2097, 2098, 2099, 2100, 2101, 2102, 2103, 2104, 2105, 2106, 2107, 2108, 2109,
		2110, 2111, 2112, 2113, 2114, 2115, 2116, 2117, 2118, 2119, 2120, 2121, 2122, 2123, 2124,
		2125, 2126, 2127, 2128, 2129, 2130, 2131, 2132, 2133, 2134, 2135, 2136, 2137, 2138, 2139,
		2140, 2141, 2142, 2143, 2144, 2145, 2146, 2147, 2148, 2149, 2150, 2151, 2152, 2153, 2154,
		2155, 2156, 2157, 2158, 2159, 2160, 2161, 2162, 2163, 2164, 2165, 2166, 2167, 2168, 2169,
		2170, 2171, 2172, 2173, 2174, 2175, 2176, 2177, 2178, 2179, 2180, 2181, 2182, 2183, 2184,
		2185, 2186, 2187, 2188, 2189, 2190, 2191, 2192, 2193, 2194, 2196, 2197, 2198, 2199, 2200,
		2201, 2202, 2203, 2204, 2205, 2206, 2207, 2208, 2209, 2210, 2211, 2212, 2213, 2214, 2215,
		2216, 2217, 2218, 2219, 2220, 2221, 2222, 2223, 2224, 2225, 2226, 2227, 2228, 2229, 2230,
		2231, 2232, 2233, 2234, 2235, 2236, 2237, 2238, 2239, 2240, 2241, 2242, 2243, 2244, 2245,
		2246, 2247, 2248, 2249, 2250, 2251, 2252, 2253, 2254, 2255, 2256, 2257, 2258, 2259, 2260,
		2261, 2262, 2263, 2264, 2265, 2266, 2267, 2268, 2269, 2270, 2271, 2272, 2273, 2274, 2275,
		2276, 2277, 2278, 2279, 2280, 2281, 2282, 2283, 2284, 2285, 2286, 2287,
	],
	full_dmg: vec![
		1556, 1557, 1563, 1568, 1569, 1580, 1593, 1596, 1650, 1651, 1652, 1673, 1674, 1675, 1679,
		1680, 1681, 1682, 1683, 1684, 1685, 1686, 1687, 1688, 1689, 1690, 1691, 1692, 1846, 1847,
		1848, 1849, 1850, 1851, 1852, 1853, 1854, 1855, 1856, 1857, 1858, 1859, 1860, 1861, 1862,
		1863, 1864, 1865, 1866, 1867, 1868, 1869, 1870, 1871, 1872, 1873, 1874, 1875, 1876, 1877,
		1878, 1879, 1880, 1881, 1882, 1883, 1884, 1885, 1886, 1887, 1888, 1889, 1890, 1891, 1892,
		1893, 1894, 1895, 1896, 1897, 1898, 1899, 1900, 1901, 1902, 1903, 1904, 1905, 1906, 1907,
		1908, 1909, 1910, 1911, 1912, 1913, 1914, 1915, 1916, 1917, 1918, 1919, 1920, 1921, 1922,
		1923, 1924, 1925, 1926, 1927, 1928, 1929, 1930, 1931, 1932, 1933, 1934, 1935, 1936, 1937,
		1938, 1939, 1940, 1941, 1942, 1943, 1944, 1945, 1946, 1947, 1948, 1949, 1950, 1951, 1952,
		1953, 1954, 1955, 1956, 1957, 1958, 1959, 1960, 1961, 1962, 1963, 1964, 1965, 1966, 1967,
		1968, 1969, 1970, 1971, 1972, 1973, 1974, 1975, 1976, 1977, 1978, 1979, 1980, 1981, 1982,
		1983, 1984, 1985, 1986, 1987, 1988, 1989, 1990, 1991, 1992, 1993, 1994, 1995, 1996, 1997,
		1998, 1999, 2000, 2001, 2002, 2003, 2004, 2005, 2006, 2007, 2008, 2009, 2010, 2011, 2012,
		2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026, 2027,
		2028, 2029, 2030, 2031, 2032, 2033, 2034, 2035, 2036, 2037, 2038, 2039, 2040, 2041, 2042,
		2043, 2044, 2045, 2046, 2047, 2048, 2049, 2050, 2051, 2052, 2053, 2054, 2055, 2056, 2057,
		2058, 2059, 2060, 2061, 2062, 2063, 2064, 2065, 2066, 2067, 2068, 2069, 2070, 2071, 2072,
		2073, 2074, 2075, 2076, 2077, 2078, 2079, 2080, 2081, 2082, 2083, 2084, 2085, 2086, 2087,
		2088, 2089, 2090, 2091, 2092, 2093, 2094, 2095, 2096, 2097, 2098, 2099, 2100, 2101, 2102,
		2103, 2104, 2105, 2106, 2107, 2108, 2109, 2110, 2111, 2112, 2113, 2114, 2115, 2116, 2117,
		2118, 2119, 2120, 2121, 2122, 2123, 2124, 2125, 2126, 2127, 2128, 2129, 2130, 2131, 2132,
		2133, 2134, 2135, 2136, 2137, 2138, 2139, 2140, 2141, 2142, 2143, 2144, 2145, 2146, 2147,
		2148, 2149, 2150, 2151, 2152, 2153, 2154, 2155, 2156, 2157, 2158, 2159, 2160, 2161, 2162,
		2163, 2164, 2165, 2166, 2167, 2168, 2169, 2170, 2171, 2172, 2173, 2174, 2175, 2176, 2177,
		2178, 2179, 2180, 2181, 2182, 2183, 2184, 2185, 2186, 2187, 2188, 2189, 2190, 2191, 2192,
		2193, 2194, 2196, 2197, 2198, 2199, 2200, 2201, 2202, 2203, 2204, 2205, 2206, 2207, 2208,
		2209, 2210, 2211, 2212, 2213, 2214, 2215, 2216, 2217, 2218, 2219, 2220, 2221, 2222, 2223,
		2224, 2225, 2226, 2227, 2228, 2229, 2230, 2231, 2232, 2233, 2234, 2235, 2236, 2237, 2238,
		2239, 2240, 2241, 2242, 2243, 2244, 2245, 2246, 2247, 2248, 2249, 2250, 2251, 2252, 2253,
		2254, 2255, 2256, 2257, 2258, 2259, 2260, 2261, 2262, 2263, 2264, 2265, 2266, 2267, 2268,
		2269, 2270, 2271, 2272, 2273, 2274, 2275, 2276, 2277, 2278, 2279, 2280, 2281, 2282, 2283,
		2284, 2285, 2286, 2287,
	],
	up: vec![],
	up_dmg: vec![],
});

#[allow(unused)]
fn make_enemy_graph(mst: &ApiManifest, list: &mut CacheList) {
	let holes = &ENEMY_SHIP_HOLES;
	for graph in mst.api_mst_shipgraph.iter() {
		if graph.api_sortno.is_some() || graph.api_id >= 5000 {
			continue;
		}

		let ship_id = format!("{0:04}", graph.api_id);

		let mut categories: Vec<String> = Vec::new();

		if !holes.full.contains(&graph.api_id) {
			categories.push("full".to_string());
		}

		if !holes.full_dmg.contains(&graph.api_id) {
			categories.push("full_dmg".to_string());
		}

		for category in categories.iter() {
			list.add(
				format!(
					"kcs2/resources/ship/{category}/{ship_id}_{}_{}.png",
					SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
					graph.api_filename
				),
				graph.api_version.first(),
			);
		}
	}
}

static SPECIAL_SHIPS: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		0639, 0724, 0694, 0969, 0918, 0446, 0554, 0553, 0576, 0411, 0184, 0733, 0412, 0944, 0916,
		0634, 0577, 0592, 0572, 0949, 0392, 0571, 0635, 0573, 0640, 0447, 0541, 1496, 0659, 0601,
		0697, 0546, 0911, 0364, 0178, 0360, 0954, 0591, 0913, 0593,
	]
});

fn make_ship_special(mst: &ApiManifest, list: &mut CacheList) {
	SPECIAL_SHIPS.iter().filter_map(|id| mst.find_shipgraph(*id)).for_each(|graph| {
		let ship_id = format!("{0:04}", graph.api_id);
		let p = format!(
			"kcs2/resources/ship/special/{ship_id}_{}.png",
			SuffixUtils::create(&ship_id, format!("ship_special").as_str()),
		);
		list.add(p, graph.api_version.first());
	});
}

#[allow(unused)]
async fn make_ship_special_greedy(
	mst: &ApiManifest,
	kache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), KacheError> {
	let checks: Vec<(String, String)> = mst
		.api_mst_shipgraph
		.iter()
		.filter(|v| v.api_sortno.is_some())
		.map(|v| {
			let ship_id = format!("{0:04}", v.api_id);
			(
				format!(
					"kcs2/resources/ship/special/{ship_id}_{}.png",
					SuffixUtils::create(&ship_id, format!("ship_special").as_str()),
				),
				v.api_version.first().cloned().unwrap_or_default(),
			)
		})
		.collect();

	let c = Arc::new(kache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, v), exists) in check_result {
		if exists {
			println!("{}, {}", p, v);
			list.add(p, v);
		}
	}

	Ok(())
}

#[allow(unused)]
async fn make_sp_remodel_greedy(
	mst: &ApiManifest,
	kache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), KacheError> {
	let checks: Vec<(String, String)> = mst
		.api_mst_shipgraph
		.iter()
		.filter(|v| v.api_sortno.is_some())
		.flat_map(|v| {
			let ship_id = format!("{0:04}", v.api_id);
			let v = v.api_version.first().cloned().unwrap_or_default();
			vec![
				(
					format!("kcs2/resources/ship/sp_remodel/animation_key/{ship_id}_remodel.json",),
					v.to_owned(),
				),
				(
					format!(
						"kcs2/resources/ship/sp_remodel/full_x2/{ship_id}_{}.png",
						SuffixUtils::create(&ship_id, "ship_sp_remodel/full_x2")
					),
					v.to_owned(),
				),
				(
					format!(
						"kcs2/resources/ship/sp_remodel/silhouette/{ship_id}_{}.png",
						SuffixUtils::create(&ship_id, "ship_sp_remodel/full_x2")
					),
					v.to_owned(),
				),
				(
					format!(
						"kcs2/resources/ship/sp_remodel/text_class/{ship_id}_{}.png",
						SuffixUtils::create(&ship_id, "ship_sp_remodel/text_class")
					),
					v.to_owned(),
				),
				(
					format!(
						"kcs2/resources/ship/sp_remodel/text_name/{ship_id}_{}.png",
						SuffixUtils::create(&ship_id, "ship_sp_remodel/text_name")
					),
					v.to_owned(),
				),
				(
					format!(
						"kcs2/resources/ship/sp_remodel/text_remodel_mes/{ship_id}_{}.png",
						SuffixUtils::create(&ship_id, "ship_sp_remodel/text_remodel_mes")
					),
					v.to_owned(),
				),
			]
		})
		.collect();

	let c = Arc::new(kache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, v), exists) in check_result {
		if exists {
			println!("{}, {}", p, v);
			list.add(p, v);
		}
	}

	Ok(())
}

static SP_REMODEL_SHIPS: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		0501, 0502, 0506, 0507, 0587, 0588, 0591, 0592, 0593, 0594, 0599, 0610, 0622, 0629, 0630,
		0646, 0651, 0652, 0656, 0662, 0663, 0667, 0668, 0694, 0698, 0707, 0883, 0888, 0894, 0899,
		0911, 0916, 0951, 0954, 0955, 0956, 0959, 0960, 0961, 0968, 0969, 0975, 0981, 0986,
	]
});

static SP_REMODEL_MES: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		0073, 0121, 0136, 0145, 0149, 0150, 0151, 0152, 0202, 0204, 0215, 0228, 0277, 0278, 0285,
		0293, 0306, 0307, 0316, 0318, 0323, 0324, 0325, 0350, 0357, 0369, 0373, 0392, 0396, 0501,
		0502, 0579, 0588, 0593, 0594, 0610, 0628, 0651, 0663, 0667, 0698, 0883, 0894, 0911, 0954,
		0955, 0960,
	]
});

#[allow(unused)]
fn make_sp_remodel(mst: &ApiManifest, list: &mut CacheList) {
	for id in SP_REMODEL_SHIPS.iter() {
		let graph = match mst.find_shipgraph(*id) {
			Some(graph) => graph,
			None => continue,
		};

		let ship_id = format!("{0:04}", id);
		let v = graph.api_version.first();
		let full_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/full_x2");
		let silh_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/silhouette");
		let cls_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/text_class");
		let name_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/text_name");

		list.add(format!("kcs2/resources/ship/sp_remodel/animation_key/{ship_id}_remodel.json"), v)
			.add(format!("kcs2/resources/ship/sp_remodel/full_x2/{ship_id}_{}.png", full_key), v)
			.add(format!("kcs2/resources/ship/sp_remodel/silhouette/{ship_id}_{}.png", silh_key), v)
			.add(format!("kcs2/resources/ship/sp_remodel/text_class/{ship_id}_{}.png", cls_key), v)
			.add(format!("kcs2/resources/ship/sp_remodel/text_name/{ship_id}_{}.png", name_key), v);
	}

	for id in SP_REMODEL_MES.iter() {
		let graph = match mst.find_shipgraph(*id) {
			Some(graph) => graph,
			None => continue,
		};

		let ship_id = format!("{0:04}", id);
		let v = graph.api_version.first();
		let mes_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/text_remodel_mes");
		list.add(
			format!("kcs2/resources/ship/sp_remodel/text_remodel_mes/{ship_id}_{}.png", mes_key),
			v,
		);
	}
}

#[allow(unused)]
async fn make_ship_type_greedy(
	mst: &ApiManifest,
	kache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), KacheError> {
	let checks: Vec<(String, String)> = mst
		.api_mst_stype
		.iter()
		.flat_map(|v| {
			[
				(format!("kcs2/resources/stype/etext/{0:03}.png", v.api_id,), "".to_string()),
				(format!("kcs2/resources/stype/etext/sp{0:03}.png", v.api_id,), "".to_string()),
			]
		})
		.collect();

	let c = Arc::new(kache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, v), exists) in check_result {
		if exists {
			println!("{}, {}", p, v);
			list.add(p, v);
		}
	}

	Ok(())
}

const SHIP_SP_TYPE_MAX: usize = 8;

fn make_ship_type(mst: &ApiManifest, list: &mut CacheList) {
	for stype in mst.api_mst_stype.iter() {
		if stype.api_id == 8 || stype.api_id == 15 {
			continue;
		}

		let stype_id = format!("{0:03}", stype.api_id);
		let etext = format!("kcs2/resources/stype/etext/{}.png", stype_id);

		list.add(etext, "");
	}

	for i in 1..=SHIP_SP_TYPE_MAX {
		let stype_id = format!("{0:03}", i);
		let etext = format!("kcs2/resources/stype/etext/sp{}.png", stype_id);

		list.add(etext, "");
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_full() {
		let ship_id = format!("{0:04}", 1);
		let key = SuffixUtils::create(&ship_id, format!("ship_full_dmg").as_str());

		assert_eq!(key, "6245");
	}

	#[test]
	fn test_sp_remodel() {
		// vec![
		// 			"/animation_key/0502_remodel.json",
		// 			"/full_x2/0502_8686.png",
		// 			"/silhouette/0502_8686.png",
		// 			"/text_class/0502_8209.png",
		// 			"/text_name/0502_4089.png",
		// 		];
		let ship_id = "0121";
		for category in ["full_x2", "silhouette", "text_class", "text_name", "text_remodel_mes"] {
			println!(
				"{}",
				SuffixUtils::create(ship_id, format!("ship_sp_remodel/{}", category).as_str())
			);
		}
	}

	#[test]
	fn test_extra() {
		let ship_id = format!("{0:04}", 5808);
		for category in ["character_full", "character_full_dmg", "character_up", "character_up_dmg"]
		{
			let key = SuffixUtils::create(&ship_id, format!("ship_{}", category).as_str());
			println!(
				"http://w01y.kancolle-server.com/kcs2/resources/ship/{category}/{ship_id}_{}.png",
				key
			);
		}
	}
}
