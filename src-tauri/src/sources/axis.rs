//! AXIS (https://axis.prioris.jp) 緊急地震速報。要アクセストークン。
//! 接続手順は参考実装(minittupoyo/axis_eew)、電文形式は公式ドキュメント
//! (/docs/message/sample/eew)による。実験的配信のため形式は予告なく変わり得る。

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;

use crate::model::{is_warning_scale, Eew, ForecastArea};
use crate::sources::ws::Stream;
use crate::sources::wolfx::normalize_scale;

pub const SOURCE: &str = "axis";
const SERVER_LIST_URL: &str = "https://axis.prioris.jp/api/server/list/";

#[derive(Debug, Deserialize)]
struct ServerList {
    servers: Vec<String>,
}

/// サーバー一覧から先頭のサーバーを選び、Bearer認証でWebSocketに接続する。
pub async fn connect(token: &str) -> Result<Stream> {
    let list: ServerList = reqwest::Client::new()
        .get(SERVER_LIST_URL)
        .bearer_auth(token)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await?
        .error_for_status()
        .context("AXIS server list (トークンを確認してください)")?
        .json()
        .await?;
    let base = list.servers.first().context("AXIS server list is empty")?;
    let mut req = format!("{}/socket", base.trim_end_matches('/')).into_client_request()?;
    req.headers_mut()
        .insert(AUTHORIZATION, format!("Bearer {token}").parse()?);
    Ok(connect_async(req).await?.0)
}

#[derive(Debug, Deserialize)]
struct Envelope {
    channel: String,
    message: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[serde(rename = "Title", default)]
    title: String,
    #[serde(rename = "OriginDateTime")]
    origin: String,
    #[serde(rename = "ReportDateTime")]
    report: String,
    #[serde(rename = "EventID")]
    event_id: String,
    #[serde(rename = "Serial")]
    serial: u32,
    #[serde(rename = "Hypocenter")]
    hypocenter: Option<Hypocenter>,
    #[serde(rename = "Intensity")]
    intensity: Option<String>,
    #[serde(rename = "Magnitude")]
    magnitude: Option<String>,
    #[serde(rename = "Flag")]
    flag: Flag,
    #[serde(rename = "Forecast", default)]
    forecast: Vec<Forecast>,
}

#[derive(Debug, Deserialize)]
struct Hypocenter {
    #[serde(rename = "Name")]
    name: Option<String>,
    /// [経度, 緯度]
    #[serde(rename = "Coordinate", default)]
    coordinate: Vec<f64>,
    /// 例: "50km"
    #[serde(rename = "Depth")]
    depth: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Flag {
    #[serde(default)]
    is_final: bool,
    #[serde(default)]
    is_cancel: bool,
    #[serde(default)]
    is_training: bool,
}

#[derive(Debug, Deserialize)]
struct Forecast {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Intensity")]
    intensity: ForecastIntensity,
}

#[derive(Debug, Deserialize)]
struct ForecastIntensity {
    #[serde(rename = "From")]
    from: Option<String>,
    #[serde(rename = "To")]
    to: Option<String>,
}

/// "50km" → 50.0。数値でなければ None。
fn parse_depth(s: &str) -> Option<f64> {
    s.trim().trim_end_matches("km").trim().parse().ok()
}

/// 受信テキストを解釈する。"hello"/"hb"やEEW以外のチャンネルは None。
pub fn parse(text: &str) -> Result<Option<Eew>> {
    if !text.trim_start().starts_with('{') {
        return Ok(None);
    }
    let env: Envelope = serde_json::from_str(text)?;
    if env.channel != "eew" {
        return Ok(None);
    }
    let m: Message = serde_json::from_value(env.message)?;
    let is_warning_title = m.title.contains("警報");
    let h = m.hypocenter.as_ref();
    Ok(Some(Eew {
        source: SOURCE.into(),
        event_id: m.event_id,
        serial: m.serial,
        origin_time: m.origin,
        report_time: m.report,
        is_warning: is_warning_title,
        is_final: m.flag.is_final,
        is_cancel: m.flag.is_cancel,
        is_training: m.flag.is_training,
        hypocenter: h.and_then(|h| h.name.clone()).filter(|n| !n.is_empty()),
        longitude: h.and_then(|h| h.coordinate.first().copied()),
        latitude: h.and_then(|h| h.coordinate.get(1).copied()),
        depth_km: h.and_then(|h| h.depth.as_deref()).and_then(parse_depth),
        magnitude: m.magnitude.and_then(|s| s.trim().parse().ok()),
        max_scale: m.intensity.as_deref().and_then(normalize_scale),
        areas: m
            .forecast
            .into_iter()
            .map(|f| {
                let to = f.intensity.to.as_deref().and_then(normalize_scale);
                ForecastArea {
                    name: f.name,
                    scale_from: f.intensity.from.as_deref().and_then(normalize_scale),
                    is_warning: is_warning_title && is_warning_scale(to.as_deref()),
                    scale_to: to,
                    arrived: false,
                }
            })
            .collect(),
        received_at_ms: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_hello_heartbeat_and_other_channels() {
        assert!(parse("hello").unwrap().is_none());
        assert!(parse("hb").unwrap().is_none());
        assert!(parse(r#"{"channel":"other","message":{}}"#).unwrap().is_none());
    }

    /// 公式ドキュメントのサンプル電文
    #[test]
    fn parses_documented_sample() {
        let e = parse(include_str!("../../tests/fixtures/axis_doc_sample_eew.json"))
            .unwrap()
            .unwrap();
        assert_eq!(e.event_id, "20220617005133");
        assert_eq!(e.serial, 9);
        assert!(!e.is_warning && e.is_final && !e.is_cancel);
        assert_eq!(e.hypocenter.as_deref(), Some("徳島県南部"));
        assert_eq!((e.longitude, e.latitude), (Some(134.6), Some(33.9)));
        assert_eq!(e.depth_km, Some(50.0));
        assert_eq!(e.magnitude, Some(5.0));
        assert_eq!(e.max_scale.as_deref(), Some("4"));
        assert_eq!(e.origin_time, "2022-06-17T00:51:25+09:00");
        assert_eq!(e.areas.len(), 2);
        assert_eq!(e.areas[0].name, "香川県東部");
        assert!(!e.areas[0].is_warning);
    }

    #[test]
    fn unknown_values_become_none() {
        let t = r#"{"channel":"eew","message":{"Title":"緊急地震速報（警報）","OriginDateTime":"2026-01-01T00:00:00+09:00","ReportDateTime":"2026-01-01T00:00:10+09:00","EventID":"1","Serial":1,"Hypocenter":{"Name":"","Coordinate":[],"Depth":"不明"},"Intensity":"不明","Magnitude":"不明","Flag":{"is_final":false,"is_cancel":false,"is_training":false},"Forecast":[]}}"#;
        let e = parse(t).unwrap().unwrap();
        assert!(e.is_warning);
        assert!(e.hypocenter.is_none() && e.latitude.is_none() && e.depth_km.is_none());
        assert!(e.magnitude.is_none() && e.max_scale.is_none());
    }
}
