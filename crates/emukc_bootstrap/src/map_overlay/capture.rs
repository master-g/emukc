use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::real_map_start_asset::RealMapStartAsset;

use super::MapOverlayBuildError;

#[derive(Debug, Clone, Deserialize)]
struct ResponseSaverRecord {
    path: String,
    body: serde_json::Value,
    #[serde(rename = "postBody")]
    post_body: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct ApiDataEnvelopeRecord {
    api_result: i64,
    #[serde(default)]
    api_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedMapCell {
    pub(crate) cell_no: i64,
    pub(super) master_cell_id: i64,
    pub(crate) color_no: i64,
    pub(super) distance: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedMapStart {
    pub(crate) map_id: i64,
    pub(crate) boss_cell_no: i64,
    pub(super) request_path: Option<String>,
    pub(crate) cells: Vec<CapturedMapCell>,
}

pub(super) fn collect_json_files(root: &Path) -> Result<Vec<PathBuf>, MapOverlayBuildError> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let entries = fs::read_dir(&path).map_err(|source| MapOverlayBuildError::Io {
            path: path.clone(),
            source,
        })?;

        for entry in entries {
            let entry = entry.map_err(|source| MapOverlayBuildError::Io {
                path: path.clone(),
                source,
            })?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
            } else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                files.push(entry_path);
            }
        }
    }

    files.sort();
    Ok(files)
}

pub(super) fn load_response_saver_capture(
    path: &Path,
) -> Result<(String, Result<CapturedMapStart, String>), MapOverlayBuildError> {
    let source = path.display().to_string();
    let record = load_response_saver_record(path)?;
    let capture = extract_map_start_capture_from_response_saver(&record);
    Ok((source, capture))
}

pub(crate) fn load_embedded_real_map_start_capture(
    asset: &RealMapStartAsset,
) -> Result<(String, Result<CapturedMapStart, String>), MapOverlayBuildError> {
    let record =
        serde_json::from_str::<ApiDataEnvelopeRecord>(asset.raw_json()).map_err(|source| {
            MapOverlayBuildError::Json {
                path: PathBuf::from(asset.name),
                source,
            }
        })?;
    Ok((asset.name.to_string(), extract_map_start_capture_from_api_data(&record)))
}

fn load_response_saver_record(path: &Path) -> Result<ResponseSaverRecord, MapOverlayBuildError> {
    let raw = fs::read_to_string(path).map_err(|source| MapOverlayBuildError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| MapOverlayBuildError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn extract_map_start_capture_from_response_saver(
    record: &ResponseSaverRecord,
) -> Result<CapturedMapStart, String> {
    if record.path != "/kcsapi/api_req_map/start" {
        return Err(format!("unsupported_path:{}", record.path));
    }

    let maparea_id = parse_i64(
        record
            .post_body
            .as_ref()
            .and_then(|body| body.get("api_maparea_id"))
            .or_else(|| record.body.get("api_maparea_id")),
    )
    .ok_or_else(|| "missing_maparea_id".to_string())?;
    let mapinfo_no = parse_i64(
        record
            .post_body
            .as_ref()
            .and_then(|body| body.get("api_mapinfo_no"))
            .or_else(|| record.body.get("api_mapinfo_no")),
    )
    .ok_or_else(|| "missing_mapinfo_no".to_string())?;
    let cells = record
        .body
        .get("api_cell_data")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "missing_api_cell_data".to_string())?
        .iter()
        .map(extract_map_cell)
        .collect::<Result<Vec<_>, _>>()?;

    validate_cells(&cells)?;
    Ok(CapturedMapStart {
        map_id: maparea_id * 10 + mapinfo_no,
        boss_cell_no: 0,
        request_path: Some(record.path.clone()),
        cells,
    })
}

fn extract_map_start_capture_from_api_data(
    record: &ApiDataEnvelopeRecord,
) -> Result<CapturedMapStart, String> {
    if record.api_result != 1 {
        return Err(format!("invalid_api_result:{}", record.api_result));
    }
    let api_data = record.api_data.as_ref().ok_or_else(|| "missing_api_data".to_string())?;

    let maparea_id = parse_i64(api_data.get("api_maparea_id"))
        .ok_or_else(|| "missing_maparea_id".to_string())?;
    let mapinfo_no = parse_i64(api_data.get("api_mapinfo_no"))
        .ok_or_else(|| "missing_mapinfo_no".to_string())?;
    let boss_cell_no = parse_i64(api_data.get("api_bosscell_no")).unwrap_or(0);
    let cells = api_data
        .get("api_cell_data")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "missing_api_cell_data".to_string())?
        .iter()
        .map(extract_map_cell)
        .collect::<Result<Vec<_>, _>>()?;

    validate_cells(&cells)?;
    Ok(CapturedMapStart {
        map_id: maparea_id * 10 + mapinfo_no,
        boss_cell_no,
        request_path: Some("/kcsapi/api_req_map/start".to_string()),
        cells,
    })
}

fn validate_cells(cells: &[CapturedMapCell]) -> Result<(), String> {
    if cells.is_empty() {
        return Err("empty_api_cell_data".to_string());
    }
    let mut seen = BTreeSet::new();
    for cell in cells {
        if !seen.insert(cell.cell_no) {
            return Err(format!("duplicate_cell_no:{}", cell.cell_no));
        }
    }
    Ok(())
}

fn extract_map_cell(value: &serde_json::Value) -> Result<CapturedMapCell, String> {
    let cell_no = parse_i64(value.get("api_no")).ok_or_else(|| "missing_cell_no".to_string())?;
    let master_cell_id =
        parse_i64(value.get("api_id")).ok_or_else(|| format!("missing_api_id:{cell_no}"))?;
    if master_cell_id <= 0 {
        return Err(format!("invalid_api_id:{cell_no}:{master_cell_id}"));
    }

    Ok(CapturedMapCell {
        cell_no,
        master_cell_id,
        color_no: parse_i64(value.get("api_color_no")).unwrap_or(0),
        distance: parse_i64(value.get("api_distance")),
    })
}

fn parse_i64(value: Option<&serde_json::Value>) -> Option<i64> {
    match value? {
        serde_json::Value::Number(number) => number.as_i64(),
        serde_json::Value::String(string) => string.parse::<i64>().ok(),
        _ => None,
    }
}
