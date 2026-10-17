//! P2P地震情報 (https://www.p2pquake.net/develop/json_api_v2/) 地震情報(code 551)

use anyhow::Result;
use serde::Deserialize;

use crate::model::{Point, Report};
use crate::time::jst_to_rfc3339 as to_rfc3339;

const HISTORY_URL: &str = "https://api.p2pquake.net/v2/history";
pub const SOURCE: &str = "p2pquake";

#[derive(Debug, Deserialize)]
pub struct Quake {
    pub id: String,
    pub earthquake: Eq,
    pub issue: Issue,
    #[serde(default)]
    pub points: Vec<P2pPoint>,
}

#[derive(Debug, Deserialize)]
pub struct Eq {
    pub time: String,
    #[serde(default)]
    pub hypocenter: Hypocenter,
    #[serde(rename = "maxScale", default)]
    pub max_scale: Option<i32>,
    #[serde(rename = "domesticTsunami")]
    pub domestic_tsunami: Option<String>,
    #[serde(rename = "foreignTsunami")]
    pub foreign_tsunami: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Hypocenter {
    pub name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub depth: Option<f64>,
    pub magnitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub time: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct P2pPoint {
    pub pref: String,
    pub addr: String,
    #[serde(rename = "isArea")]
    pub is_area: bool,
    pub scale: i32,
}

pub fn scale_label(code: i32) -> Option<&'static str> {
    Some(match code {
        10 => "1",
        20 => "2",
        30 => "3",
        40 => "4",
        45 => "5-",
        46 => "5-over",
        50 => "5+",
        55 => "6-",
        60 => "6+",
        70 => "7",
        _ => return None,
    })
}

fn priority(kind: &str) -> u8 {
    match kind {
        "ScalePrompt" => 1,
        "Destination" | "Foreign" => 2,
        "DetailScale" => 3,
        _ => 0,
    }
}

pub fn normalize(q: &Quake) -> Result<Report> {
    let h = &q.earthquake.hypocenter;
    // 欠損は -1 / -200 / 空文字で表される
    let valid = |v: Option<f64>, min: f64| v.filter(|v| *v >= min);
    Ok(Report {
        key_time: to_rfc3339(&q.earthquake.time)?,
        origin_time: None,
        issue_time: to_rfc3339(&q.issue.time)?,
        priority: priority(&q.issue.kind),
        name: h.name.clone().filter(|n| !n.is_empty()),
        latitude: valid(h.latitude, -90.0),
        longitude: valid(h.longitude, -180.0),
        depth_km: valid(h.depth, 0.0),
        magnitude: valid(h.magnitude, 0.0),
        max_scale: q
            .earthquake
            .max_scale
            .and_then(scale_label)
            .map(str::to_string),
        domestic_tsunami: q.earthquake.domestic_tsunami.clone(),
        foreign_tsunami: q.earthquake.foreign_tsunami.clone(),
        points: q
            .points
            .iter()
            .filter_map(|p| {
                Some(Point {
                    pref: p.pref.clone(),
                    addr: p.addr.clone(),
                    scale: scale_label(p.scale)?.to_string(),
                    is_area: p.is_area,
                    city_code: None,
                })
            })
            .collect(),
    })
}

/// 履歴を取得する。生JSON文字列も返す(DB保存用)。
pub async fn fetch_history(
    client: &reqwest::Client,
    offset: u32,
    limit: u32,
) -> Result<Vec<(Quake, String)>> {
    let values: Vec<serde_json::Value> = client
        .get(HISTORY_URL)
        .query(&[("codes", "551")])
        .query(&[("limit", limit), ("offset", offset)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    values
        .into_iter()
        .map(|v| Ok((serde_json::from_value(v.clone())?, v.to_string())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::merge;

    fn fixture() -> Vec<Quake> {
        let s = include_str!("../../tests/fixtures/p2p_history_551.json");
        serde_json::from_str(s).expect("fixture parses")
    }

    #[test]
    fn parses_and_normalizes_all_fixture_entries() {
        for q in fixture() {
            normalize(&q).unwrap();
        }
    }

    #[test]
    fn missing_values_become_none() {
        let q = fixture()
            .into_iter()
            .find(|q| q.issue.kind == "ScalePrompt")
            .unwrap();
        let r = normalize(&q).unwrap();
        assert!(r.latitude.is_none() && r.depth_km.is_none() && r.name.is_none());
        assert!(r.magnitude.is_none());
        assert!(r.max_scale.is_some());
    }

    #[test]
    fn merges_prompt_destination_detail_of_same_event() {
        let mut by_time: std::collections::HashMap<String, Vec<Report>> = Default::default();
        for q in fixture() {
            let r = normalize(&q).unwrap();
            by_time.entry(r.key_time.clone()).or_default().push(r);
        }
        let (t, rs) = by_time.into_iter().find(|(_, rs)| rs.len() >= 3).unwrap();
        let eq = merge(&t, rs);
        assert!(eq.name.is_some() && eq.magnitude.is_some() && eq.max_scale.is_some());
    }
}
