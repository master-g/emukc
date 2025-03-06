//! crawl kcs2 resources se

use std::sync::LazyLock;

use emukc_cache::kache;

static SE: LazyLock<Vec<u32>> = LazyLock::new(|| {
	vec![
		101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118,
		120, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217,
		218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 240, 241, 242, 243,
		244, 245, 246, 247, 248, 249, 250, 252, 253, 254, 255, 256, 257, 258, 301, 302, 303, 304,
		305, 306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320, 321, 322,
		323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333,
	]
});

static AREA_SALLY: LazyLock<&[&str]> =
	LazyLock::new(|| &["001", "002", "004", "005", "006", "007", "057", "057_2", "058"]);

static AREA_AIR_UNIT: LazyLock<&[&str]> = LazyLock::new(|| &["006", "007", "058"]);

static TUTORIAL_VOICE: LazyLock<&[&str]> = LazyLock::new(|| {
	&[
		"021", "022", "023_a", "024", "025", "026_a", "027", "028", "029", "030", "031", "032_a",
		"033", "034", "035",
	]
});

pub(super) async fn crawl(cache: &kache::Kache) -> Result<(), kache::Error> {
	for se in SE.iter() {
		cache.get(format!("kcs2/resources/se/{}.mp3", se).as_str(), None).await?;
	}

	for sally in AREA_SALLY.iter() {
		cache.get(format!("kcs2/resources/area/sally/{}.png", sally).as_str(), None).await?;
	}

	for air_unit in AREA_AIR_UNIT.iter() {
		cache.get(format!("kcs2/resources/area/airunit/{}.png", air_unit).as_str(), None).await?;
	}

	for voice in TUTORIAL_VOICE.iter() {
		cache.get(format!("kcs2/resources/voice/tutorial/{}.mp3", voice).as_str(), None).await?;
	}

	for i in 1..=103 {
		cache.get(format!("kcs2/resources/voice/titlecall_1/{0:03}.mp3", i).as_str(), None).await?;
	}
	for i in 1..=64 {
		cache.get(format!("kcs2/resources/voice/titlecall_2/{0:03}.mp3", i).as_str(), None).await?;
	}

	Ok(())
}
