//! crawl kcs2 resources se

use std::sync::LazyLock;

use crate::{make_list::CacheList, prelude::CacheListMakingError};

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

pub(super) static AREA_AIR_UNIT: LazyLock<&[&str]> = LazyLock::new(|| &["006", "007", "058"]);

static TUTORIAL_VOICE: LazyLock<&[&str]> = LazyLock::new(|| {
    &[
        "021", "022", "023_a", "024", "025", "026_a", "027", "028", "029", "030", "031", "032_a",
        "033", "034", "035",
    ]
});

static WORLD_SELECT: LazyLock<&[&str]> = LazyLock::new(|| {
    &[
        "bg.jpg",
        "error.png",
        "gauge20.png",
        "gauge20_gray.png",
        "limit.png",
        "title20_icon.png",
        "title20_select.png",
    ]
});

pub(super) fn is_default_se_id(id: i64) -> bool {
    let Ok(id) = u32::try_from(id) else {
        return false;
    };
    SE.contains(&id)
}

pub(super) fn is_default_tutorial_voice_stem(stem: &str) -> bool {
    TUTORIAL_VOICE.contains(&stem)
}

pub(super) fn is_default_voice_file(file: &str) -> bool {
    let Some((category, name)) = file.split_once('/') else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".mp3") else {
        return false;
    };
    if category == "tutorial" {
        return is_default_tutorial_voice_stem(stem);
    }
    let Ok(id) = stem.parse::<u32>() else {
        return false;
    };

    match category {
        "titlecall_1" => (1..=103).contains(&id),
        "titlecall_2" => (1..=64).contains(&id),
        _ => false,
    }
}

pub(super) fn is_default_world_select_file(file: &str) -> bool {
    if WORLD_SELECT.contains(&file) || file == "btn_chinjyufu_on.png" {
        return true;
    }

    let Some(stem) = file.strip_prefix("btn_chinjyufu") else {
        return false;
    };
    let stem = stem.strip_suffix(".png").unwrap_or(stem);
    let stem = stem.strip_suffix("_off").unwrap_or(stem);
    stem.parse::<u32>().is_ok_and(|id| (1..=20).contains(&id))
}

pub(super) async fn make(list: &mut CacheList) -> Result<(), CacheListMakingError> {
    for se in SE.iter() {
        list.add_unversioned(format!("kcs2/resources/se/{se}.mp3"));
    }

    for sally in AREA_SALLY.iter() {
        list.add_unversioned(format!("kcs2/resources/area/sally/{sally}.png"));
    }

    for air_unit in AREA_AIR_UNIT.iter() {
        list.add_unversioned(format!("kcs2/resources/area/airunit/{air_unit}.png"));
    }

    for voice in TUTORIAL_VOICE.iter() {
        list.add_unversioned(format!("kcs2/resources/voice/tutorial/{voice}.mp3"));
    }

    for i in 1..=103 {
        list.add_unversioned(format!("kcs2/resources/voice/titlecall_1/{i:03}.mp3"));
    }
    for i in 1..=64 {
        list.add_unversioned(format!("kcs2/resources/voice/titlecall_2/{i:03}.mp3"));
    }

    for i in 1..=20 {
        list.add_unversioned(format!("kcs2/resources/worldselect/btn_chinjyufu{i}.png"));
        list.add_unversioned(format!("kcs2/resources/worldselect/btn_chinjyufu{i}_off.png"));
    }
    list.add_unversioned("kcs2/resources/worldselect/btn_chinjyufu_on.png".to_string());

    for res in WORLD_SELECT.iter() {
        list.add_unversioned(format!("kcs2/resources/worldselect/{res}"));
    }

    Ok(())
}
