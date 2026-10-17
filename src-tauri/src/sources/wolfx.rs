//! wolfx JMA緊急地震速報 (wss://ws-api.wolfx.jp/jma_eew)

use anyhow::Result;
use serde::Deserialize;

use crate::model::{Eew, ForecastArea};
use crate::time::jst_to_rfc3339;

pub const SOURCE: &str = "wolfx";
pub const URL: &str = "wss://ws-api.wolfx.jp/jma_eew";

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(rename = "EventID")]
    event_id: Option<String>,
    #[serde(rename = "Serial", default)]
    serial: u32,
    #[serde(rename = "AnnouncedTime", default)]
    announced: String,
    #[serde(rename = "OriginTime", default)]
    origin: String,
    #[serde(rename = "Hypocenter")]
    hypocenter: Option<String>,
    #[serde(rename = "Latitude")]
    latitude: Option<f64>,
    #[serde(rename = "Longitude")]
    longitude: Option<f64>,
    #[serde(rename = "Depth")]
    depth: Option<f64>,
    #[serde(rename = "Magnitude")]
    magnitude: Option<f64>,
    #[serde(rename = "MaxIntensity")]
    max_intensity: Option<String>,
    #[serde(rename = "WarnArea", default)]
    warn_area: Vec<WarnArea>,
    #[serde(rename = "isTraining", default)]
    is_training: bool,
    #[serde(rename = "isWarn", default)]
    is_warn: bool,
    #[serde(rename = "isFinal", default)]
    is_final: bool,
    #[serde(rename = "isCancel", default)]
    is_cancel: bool,
}

#[derive(Debug, Deserialize)]
struct WarnArea {
    #[serde(rename = "Chiiki", default)]
    name: String,
    #[serde(rename = "Shindo1")]
    from: Option<String>,
    #[serde(rename = "Shindo2")]
    to: Option<String>,
    #[serde(rename = "Type", default)]
    kind: String,
    #[serde(rename = "Arrive", default)]
    arrive: bool,
}

/// "5弱" → "5-", "5強" → "5+", "以上"/"over" → "over"。不明は None。
pub fn normalize_scale(s: &str) -> Option<String> {
    Some(match s.trim() {
        "1" | "2" | "3" | "4" | "7" => s.trim().to_string(),
        "5弱" | "5-" => "5-".into(),
        "5強" | "5+" => "5+".into(),
        "6弱" | "6-" => "6-".into(),
        "6強" | "6+" => "6+".into(),
        "over" | "以上" => "over".into(),
        _ => return None,
    })
}

/// 位置・規模が未設定のとき 0 が入るため欠損として扱う
fn nonzero(v: Option<f64>) -> Option<f64> {
    v.filter(|v| *v != 0.0)
}

/// 受信テキストを解釈する。EEW以外(heartbeat/pong等)は None。
pub fn parse(text: &str) -> Result<Option<Eew>> {
    let m: Message = serde_json::from_str(text)?;
    if m.kind != "jma_eew" && !m.kind.is_empty() {
        return Ok(None);
    }
    let Some(event_id) = m.event_id else {
        return Ok(None);
    };
    Ok(Some(Eew {
        source: SOURCE.into(),
        event_id,
        serial: m.serial,
        origin_time: jst_to_rfc3339(&m.origin)?,
        report_time: jst_to_rfc3339(&m.announced)?,
        is_warning: m.is_warn,
        is_final: m.is_final,
        is_cancel: m.is_cancel,
        is_training: m.is_training,
        hypocenter: m.hypocenter.filter(|s| !s.is_empty()),
        latitude: nonzero(m.latitude),
        longitude: nonzero(m.longitude),
        depth_km: m.depth.filter(|d| *d > 0.0),
        magnitude: nonzero(m.magnitude),
        max_scale: m.max_intensity.as_deref().and_then(normalize_scale),
        areas: m
            .warn_area
            .into_iter()
            .map(|a| ForecastArea {
                name: a.name,
                scale_from: a.from.as_deref().and_then(normalize_scale),
                scale_to: a.to.as_deref().and_then(normalize_scale),
                is_warning: a.kind == "警報",
                arrived: a.arrive,
            })
            .collect(),
        received_at_ms: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_heartbeat_and_pong() {
        assert!(parse(r#"{"type":"heartbeat","ver":24,"id":"1","timestamp":1}"#).unwrap().is_none());
        assert!(parse(r#"{"type":"pong","timestamp":1}"#).unwrap().is_none());
    }

    #[test]
    fn parses_real_cancel_report() {
        let e = parse(include_str!("../../tests/fixtures/wolfx_jma_eew_latest.json"))
            .unwrap()
            .unwrap();
        assert!(e.is_cancel && !e.is_warning);
        assert_eq!(e.serial, 37);
        assert_eq!(e.origin_time, "2026-10-16T12:38:10+09:00");
        assert!(e.hypocenter.is_none() && e.latitude.is_none() && e.magnitude.is_none());
        assert!(e.max_scale.is_none());
    }

    /// フィールド定義(wolfx公式ドキュメント)から作った合成データ
    #[test]
    fn parses_synthetic_warning() {
        let e = parse(include_str!("../../tests/fixtures/wolfx_jma_eew_synthetic_warning.json"))
            .unwrap()
            .unwrap();
        assert!(e.is_warning && !e.is_cancel);
        assert_eq!(e.hypocenter.as_deref(), Some("石川県能登地方"));
        assert_eq!(e.max_scale.as_deref(), Some("5+"));
        assert_eq!(e.magnitude, Some(6.6));
        assert_eq!(e.areas.len(), 2);
        assert_eq!(e.areas[0].scale_from.as_deref(), Some("5-"));
        assert_eq!(e.areas[0].scale_to.as_deref(), Some("5+"));
        assert!(e.areas[0].is_warning && !e.areas[1].is_warning);
    }

    #[test]
    fn scale_normalization() {
        assert_eq!(normalize_scale("5弱").as_deref(), Some("5-"));
        assert_eq!(normalize_scale("不明"), None);
        assert_eq!(normalize_scale("over").as_deref(), Some("over"));
    }
}
