//! P2P地震情報 WebSocket (wss://api.p2pquake.net/v2/ws) の緊急地震速報(code 556, 警報のみ)

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::model::{is_warning_scale, scale_rank, Eew, ForecastArea};
use crate::sources::p2p::scale_label;
use crate::time::jst_to_rfc3339;

pub const SOURCE: &str = "p2pquake";
pub const URL: &str = "wss://api.p2pquake.net/v2/ws";

#[derive(Debug, Deserialize)]
struct Message {
    code: i32,
    #[serde(default)]
    cancelled: bool,
    #[serde(default)]
    test: bool,
    earthquake: Option<Earthquake>,
    issue: Issue,
    #[serde(default)]
    areas: Option<Vec<Area>>,
}

#[derive(Debug, Deserialize)]
struct Earthquake {
    #[serde(rename = "originTime")]
    origin_time: String,
    hypocenter: Option<Hypocenter>,
}

#[derive(Debug, Deserialize)]
struct Hypocenter {
    name: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    depth: Option<f64>,
    magnitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct Issue {
    #[serde(rename = "eventId")]
    event_id: String,
    serial: String,
    time: String,
}

#[derive(Debug, Deserialize)]
struct Area {
    name: String,
    #[serde(rename = "scaleFrom")]
    from: i32,
    #[serde(rename = "scaleTo")]
    to: i32,
    /// 10: 主要動は未到達と予測 / 11: 既に到達と予測
    #[serde(rename = "kindCode", default)]
    kind_code: String,
}

/// 0 → "0"、99 → "over"(以上)、それ以外は震度コード
fn area_scale(code: i32) -> Option<String> {
    match code {
        0 => Some("0".into()),
        99 => Some("over".into()),
        c => scale_label(c).map(str::to_string),
    }
}

/// 受信テキストを解釈する。緊急地震速報(556)以外は None。
pub fn parse(text: &str) -> Result<Option<Eew>> {
    let m: Message = serde_json::from_str(text)?;
    if m.code != 556 {
        return Ok(None);
    }
    let report_time = jst_to_rfc3339(&m.issue.time)?;
    let h = m.earthquake.as_ref().and_then(|e| e.hypocenter.as_ref());
    let valid = |v: Option<f64>, min: f64| v.filter(|v| *v >= min);
    let areas: Vec<ForecastArea> = m
        .areas
        .unwrap_or_default()
        .into_iter()
        .map(|a| {
            let to = area_scale(a.to);
            ForecastArea {
                name: a.name,
                scale_from: area_scale(a.from),
                is_warning: is_warning_scale(to.as_deref()),
                scale_to: to,
                arrived: a.kind_code == "11",
            }
        })
        .collect();
    let max_scale = areas
        .iter()
        .filter_map(|a| a.scale_to.clone())
        .filter(|s| scale_rank(s).is_some())
        .max_by_key(|s| scale_rank(s));
    Ok(Some(Eew {
        source: SOURCE.into(),
        event_id: m.issue.event_id,
        serial: m.issue.serial.parse().context("invalid serial")?,
        origin_time: match &m.earthquake {
            Some(e) => jst_to_rfc3339(&e.origin_time)?,
            None => report_time.clone(),
        },
        report_time,
        // 556は警報のみ配信される
        is_warning: !m.cancelled,
        is_final: false,
        is_cancel: m.cancelled,
        is_training: m.test,
        hypocenter: h.and_then(|h| h.name.clone()).filter(|n| !n.is_empty()),
        latitude: h.and_then(|h| valid(h.latitude, -90.0)),
        longitude: h.and_then(|h| valid(h.longitude, -180.0)),
        depth_km: h.and_then(|h| valid(h.depth, 0.0)),
        magnitude: h.and_then(|h| valid(h.magnitude, 0.0)),
        max_scale,
        areas,
        received_at_ms: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<serde_json::Value> {
        serde_json::from_str(include_str!("../../tests/fixtures/p2p_history_556.json")).unwrap()
    }

    #[test]
    fn ignores_other_codes() {
        assert!(parse(r#"{"code":555,"areas":[],"issue":{"eventId":"x","serial":"1","time":"2026/01/01 00:00:00"}}"#)
            .unwrap()
            .is_none());
    }

    #[test]
    fn parses_real_warning() {
        let v = fixture().into_iter().find(|v| v["issue"]["serial"] == "2" && !v["cancelled"].as_bool().unwrap()).unwrap();
        let e = parse(&v.to_string()).unwrap().unwrap();
        assert_eq!(e.event_id, "20261016123810");
        assert_eq!(e.serial, 2);
        assert!(e.is_warning && !e.is_cancel);
        assert_eq!(e.hypocenter.as_deref(), Some("千島列島"));
        assert_eq!(e.origin_time, "2026-10-16T12:36:27+09:00");
        assert_eq!(e.magnitude, Some(9.9));
        assert_eq!(e.max_scale.as_deref(), Some("7"));
        assert_eq!(e.areas.len(), 105);
        assert!(e.areas.iter().any(|a| a.arrived));
        assert!(e.areas.iter().any(|a| !a.is_warning));
    }

    #[test]
    fn parses_real_cancel() {
        let v = fixture().into_iter().find(|v| v["cancelled"].as_bool().unwrap()).unwrap();
        let e = parse(&v.to_string()).unwrap().unwrap();
        assert!(e.is_cancel && !e.is_warning);
        assert_eq!(e.serial, 2);
        assert!(e.hypocenter.is_none() && e.areas.is_empty());
        assert_eq!(e.origin_time, e.report_time);
    }

    #[test]
    fn all_fixture_entries_parse() {
        for v in fixture() {
            parse(&v.to_string()).unwrap().unwrap();
        }
    }
}
