use serde::Serialize;

use crate::net::prelude::*;
use crate::net::router::kcs2::GIT_HASH;

/// Event-map combined-fleet UI capability (plan KTD7): emukc supports
/// combined types 0..=3 including transport, so emit 2 (all selectable).
const API_EVENT_OBJECT_M_FLAG: i64 = 2;

#[derive(Serialize)]
struct Resp {
    api_material: Vec<KcApiMaterialElement>,
    api_deck_port: Vec<KcApiDeckPort>,
    api_ndock: Vec<KcApiNDock>,
    api_ship: Vec<KcApiShip>,
    api_basic: KcApiUserBasic,
    api_log: Vec<KcApiLogElement>,
    api_combined_flag: i64,
    api_p_bgm_id: i64,
    api_event_object: KcApiEventObject,
    api_parallel_quest_count: i64,
    api_dest_ship_slot: i64,
    // api_plane_info: Vec<KcApiPlaneInfo>,
    // api_furniture_affect_items: Vec<i64>,
    api_c_flags: Vec<i64>,
    api_c_flag2: i64,
}

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let view = state.port_view(pid).await?;

    Ok(KcApiResponse::success(&project(view)))
}

fn project(view: PortView) -> Resp {
    let ver = format!("Welcome to EmuKC {}-{}", VERSION, GIT_HASH.to_uppercase());
    let api_log = vec![KcApiLogElement {
        api_no: 0,
        api_type: "10".to_string(),
        api_state: "0".to_string(),
        api_message: ver,
    }];

    // log type
    // 1: ndock
    // 2: factory
    // 3: expedition
    // 4: provision
    // 5: practice
    // 6: medal
    // 7: sortie
    // 8: quest
    // 9: apply
    // 10: promotion
    // 11: picturebook
    // 12: complete
    // 13: n/a
    // 14: sortie
    // 15: remodel

    Resp {
        api_material: view.materials.into(),
        api_deck_port: view.fleets.into_iter().map(std::convert::Into::into).collect(),
        api_dest_ship_slot: 1,
        api_ndock: view.ndocks.into_iter().map(std::convert::Into::into).collect(),
        api_ship: view.ships,
        api_parallel_quest_count: view.basic.api_max_quests,
        api_basic: view.basic,
        api_log,
        api_p_bgm_id: view.port_bgm_id,
        api_event_object: KcApiEventObject {
            api_m_flag: API_EVENT_OBJECT_M_FLAG,
            api_c_num: None,
            api_m_flag2: None,
        },
        api_c_flags: vec![0], // event functional flags
        api_c_flag2: 0,       // mini event item usage lock flag
        api_combined_flag: view.combined_type,
    }
}
