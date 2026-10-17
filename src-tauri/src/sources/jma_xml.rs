//! 気象庁防災情報XML(PULL型)。地震情報のうち「震源・震度に関する情報」(VXSE53)と「震度速報」(VXSE51)を扱う。
//! フィード: https://www.data.jma.go.jp/developer/xml/feed/eqvol.xml(高頻度) / eqvol_l.xml(長期、約1週間分)
//! 統合キーは ArrivalTime(P2P地震情報の earthquake.time と一致することを実データで確認済み)。

use anyhow::{bail, Context, Result};
use reqwest::header::{ETAG, IF_NONE_MATCH};
use roxmltree::{Document, Node};

use crate::model::{Point, Report};

pub const SOURCE: &str = "jma";
pub const FEED_SHORT: &str = "https://www.data.jma.go.jp/developer/xml/feed/eqvol.xml";
pub const FEED_LONG: &str = "https://www.data.jma.go.jp/developer/xml/feed/eqvol_l.xml";

/// 取り込み対象の電文種別(ファイル名中の電文コード)
const KINDS: [&str; 2] = ["_VXSE53_", "_VXSE51_"];
/// 気象庁XMLの優先度。P2P(DetailScale=3)より確からしい。
const PRIORITY: u8 = 4;
/// 震度速報(震源・震度の詳細を含まない)。P2Pの震度速報(1)より確からしく、詳細(3)には劣る。
const PRIORITY_PROMPT: u8 = 2;

/// フィードの1エントリ(取り込み対象のみ)
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// ファイル名(拡張子なし)。DBの source_id に使う
    pub id: String,
    pub url: String,
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

fn path<'a, 'i>(n: Node<'a, 'i>, names: &[&str]) -> Option<Node<'a, 'i>> {
    names.iter().try_fold(n, |n, name| child(n, name))
}

fn text<'a>(n: Option<Node<'a, '_>>) -> Option<&'a str> {
    n.and_then(|n| n.text()).map(str::trim).filter(|t| !t.is_empty())
}

/// フィードから「震源・震度に関する情報」「震度速報」のエントリを取り出す。
pub fn parse_feed(xml: &str) -> Result<Vec<Entry>> {
    let doc = Document::parse(xml)?;
    let mut out = vec![];
    for entry in doc.descendants().filter(|n| n.is_element() && n.tag_name().name() == "entry") {
        let Some(url) = child(entry, "link").and_then(|l| l.attribute("href")) else {
            continue;
        };
        if !KINDS.iter().any(|k| url.contains(k)) {
            continue;
        }
        let id = url.rsplit('/').next().unwrap_or(url).trim_end_matches(".xml");
        out.push(Entry { id: id.to_string(), url: url.to_string() });
    }
    Ok(out)
}

/// ETagを使ってフィードを取得する。未更新(304)なら None。
pub async fn fetch_feed(
    client: &reqwest::Client,
    url: &str,
    etag: Option<&str>,
) -> Result<Option<(String, Option<String>)>> {
    let mut req = client.get(url);
    if let Some(e) = etag {
        req = req.header(IF_NONE_MATCH, e);
    }
    let resp = req.send().await?;
    if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(None);
    }
    let resp = resp.error_for_status()?;
    let new_etag = resp.headers().get(ETAG).and_then(|v| v.to_str().ok()).map(str::to_string);
    Ok(Some((resp.text().await?, new_etag)))
}

pub async fn fetch_report(client: &reqwest::Client, url: &str) -> Result<String> {
    Ok(client.get(url).send().await?.error_for_status()?.text().await?)
}

/// "+34.1+135.1-10000/" → (緯度, 経度, 深さkm)。深さは無いことがある。
fn parse_coordinate(s: &str) -> Option<(f64, f64, Option<f64>)> {
    let s = s.trim().trim_end_matches('/');
    // 符号の位置で区切る
    let mut parts: Vec<&str> = vec![];
    let mut start = 0;
    for (i, c) in s.char_indices().skip(1) {
        if c == '+' || c == '-' {
            parts.push(&s[start..i]);
            start = i;
        }
    }
    parts.push(&s[start..]);
    let n = |p: &str| p.parse::<f64>().ok();
    let lat = n(parts.first()?)?;
    let lon = n(parts.get(1)?)?;
    // 標高(m)。震源の深さは負の値で表される
    let depth = parts.get(2).and_then(|p| n(p)).map(|v| (-v / 1000.0).max(0.0));
    Some((lat, lon, depth))
}

fn scale(s: &str) -> Option<String> {
    Some(match s {
        "1" | "2" | "3" | "4" | "7" => s.to_string(),
        "5-" | "5+" | "6-" | "6+" => s.to_string(),
        "!5-" => "5-over".to_string(),
        _ => return None,
    })
}

/// 震度速報(VXSE51)か、震源・震度に関する情報(VXSE53)かを見分けて正規化する。
/// 取消・訓練・試験・対象外は None。
pub fn normalize(xml: &str) -> Result<Option<Report>> {
    let doc = Document::parse(xml)?;
    let root = doc.root_element();
    let is_prompt = child(root, "Head").and_then(|h| text(child(h, "InfoKind"))) == Some("震度速報");
    if is_prompt {
        return normalize_prompt(root);
    }
    let control = child(root, "Control").context("no Control")?;
    if text(child(control, "Status")) != Some("通常") {
        return Ok(None);
    }
    let head = child(root, "Head").context("no Head")?;
    if text(child(head, "InfoType")) == Some("取消") {
        return Ok(None);
    }
    let body = child(root, "Body").context("no Body")?;
    let eq = child(body, "Earthquake").context("no Earthquake")?;
    let arrival = text(child(eq, "ArrivalTime")).context("no ArrivalTime")?;
    if bail_if_invalid(arrival).is_err() {
        bail!("unexpected time format: {arrival}");
    }
    let area = path(eq, &["Hypocenter", "Area"]);
    let coord = text(area.and_then(|a| child(a, "Coordinate"))).and_then(parse_coordinate);
    let magnitude = text(child(eq, "Magnitude")).and_then(|m| m.parse::<f64>().ok()).filter(|m| m.is_finite());

    let mut max_scale = None;
    let mut points = vec![];
    if let Some(obs) = path(body, &["Intensity", "Observation"]) {
        max_scale = text(child(obs, "MaxInt")).and_then(scale);
        for pref in obs.children().filter(|n| n.is_element() && n.tag_name().name() == "Pref") {
            let pref_name = text(child(pref, "Name")).unwrap_or_default();
            for city in pref
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "City")
            {
                let code = text(child(city, "Code")).map(str::to_string);
                for st in city
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "IntensityStation")
                {
                    let (Some(name), Some(sc)) =
                        (text(child(st, "Name")), text(child(st, "Int")).and_then(scale))
                    else {
                        continue;
                    };
                    points.push(Point {
                        pref: pref_name.to_string(),
                        // ＊は気象庁以外の観測点を示す注記なので表示名からは除く
                        addr: name.trim_end_matches('＊').to_string(),
                        scale: sc,
                        is_area: false,
                        city_code: code.clone(),
                    });
                }
            }
        }
    }

    Ok(Some(Report {
        key_time: arrival.to_string(),
        origin_time: text(child(eq, "OriginTime")).map(str::to_string),
        issue_time: text(child(head, "ReportDateTime")).unwrap_or(arrival).to_string(),
        priority: PRIORITY,
        name: text(area.and_then(|a| child(a, "Name"))).map(str::to_string),
        latitude: coord.map(|c| c.0),
        longitude: coord.map(|c| c.1),
        depth_km: coord.and_then(|c| c.2),
        magnitude,
        max_scale,
        domestic_tsunami: None,
        foreign_tsunami: None,
        points,
    }))
}

/// 震度速報(VXSE51)。震源・マグニチュードは含まず、細分区域ごとの震度だけが入る。
/// 統合キーは EventID(検知時刻 YYYYMMDDHHMMSS)の分までを使う(VXSE53の ArrivalTime・P2Pの時刻と分まで一致)。
fn normalize_prompt(root: Node) -> Result<Option<Report>> {
    let control = child(root, "Control").context("no Control")?;
    let head = child(root, "Head").context("no Head")?;
    if text(child(control, "Status")) != Some("通常") || text(child(head, "InfoType")) == Some("取消") {
        return Ok(None);
    }
    let id = text(child(head, "EventID")).context("no EventID")?;
    if id.len() < 12 || !id[..12].bytes().all(|b| b.is_ascii_digit()) {
        bail!("unexpected EventID: {id}");
    }
    let key = format!(
        "{}-{}-{}T{}:{}:00+09:00",
        &id[..4], &id[4..6], &id[6..8], &id[8..10], &id[10..12]
    );
    let body = child(root, "Body").context("no Body")?;
    let obs = path(body, &["Intensity", "Observation"]).context("no Observation")?;
    let mut points = vec![];
    for pref in obs.children().filter(|n| n.is_element() && n.tag_name().name() == "Pref") {
        let pref_name = text(child(pref, "Name")).unwrap_or_default();
        for area in pref.children().filter(|n| n.is_element() && n.tag_name().name() == "Area") {
            let (Some(name), Some(sc)) = (text(child(area, "Name")), text(child(area, "MaxInt")).and_then(scale))
            else {
                continue;
            };
            points.push(Point {
                pref: pref_name.to_string(),
                addr: name.to_string(),
                scale: sc,
                is_area: true,
                city_code: None,
            });
        }
    }
    Ok(Some(Report {
        key_time: key.clone(),
        origin_time: None,
        issue_time: text(child(head, "ReportDateTime")).unwrap_or(&key).to_string(),
        priority: PRIORITY_PROMPT,
        max_scale: text(child(obs, "MaxInt")).and_then(scale),
        points,
        ..Default::default()
    }))
}

/// 統合キーとして使う時刻は P2P 側と同じ "YYYY-MM-DDTHH:MM:SS+09:00" でなければならない。
fn bail_if_invalid(t: &str) -> Result<()> {
    let ok = t.len() == 25
        && t.ends_with("+09:00")
        && t.as_bytes()[10] == b'T'
        && t[..19].chars().filter(|c| c.is_ascii_digit()).count() == 14;
    if ok { Ok(()) } else { bail!("bad time") }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = include_str!("../../tests/fixtures/jma_vxse53_small.xml");
    const LARGE: &str = include_str!("../../tests/fixtures/jma_vxse53_large.xml");

    #[test]
    fn feed_keeps_only_hypocenter_and_intensity_reports() {
        let e = parse_feed(include_str!("../../tests/fixtures/jma_feed_eqvol_l.xml")).unwrap();
        assert_eq!(e.len(), 29);
        assert!(e.iter().all(|e| e.id.contains("VXSE53") && e.url.ends_with(".xml")));
        assert_eq!(e[0].id, "20261017025652_0_VXSE53_270000");
    }

    #[test]
    fn parses_intensity_prompt_report() {
        let r = normalize(include_str!("../../tests/fixtures/jma_vxse51_sample.xml")).unwrap().unwrap();
        assert_eq!(r.key_time, "2011-03-11T14:46:00+09:00");
        assert_eq!(r.priority, 2);
        assert_eq!(r.max_scale.as_deref(), Some("6+"));
        assert!(r.latitude.is_none() && r.name.is_none());
        assert_eq!(r.points.len(), 17);
        assert!(r.points.iter().all(|p| p.is_area));
        let p = r.points.iter().find(|p| p.addr == "宮城県北部").unwrap();
        assert_eq!((p.pref.as_str(), p.scale.as_str()), ("宮城県", "6+"));
    }

    #[test]
    fn feed_accepts_prompt_entries() {
        let feed = r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><link type="application/xml" href="https://x/a/20261017115600_0_VXSE51_270000.xml"/></entry><entry><link href="https://x/a/20261017_VXSE52_1.xml"/></entry></feed>"#;
        let e = parse_feed(feed).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].id, "20261017115600_0_VXSE51_270000");
    }

    #[test]
    fn parses_small_report() {
        let r = normalize(SMALL).unwrap().unwrap();
        assert_eq!(r.key_time, "2026-10-17T11:54:00+09:00");
        assert_eq!(r.origin_time.as_deref(), Some("2026-10-17T11:54:00+09:00"));
        assert_eq!(r.name.as_deref(), Some("紀伊水道"));
        assert_eq!((r.latitude, r.longitude, r.depth_km), (Some(34.1), Some(135.1), Some(10.0)));
        assert_eq!(r.magnitude, Some(2.6));
        assert_eq!(r.max_scale.as_deref(), Some("1"));
        assert_eq!(r.priority, 4);
        assert_eq!(r.points.len(), 3);
        let p = &r.points[0];
        assert_eq!((p.pref.as_str(), p.addr.as_str(), p.scale.as_str()), ("和歌山県", "海南市下津", "1"));
        assert_eq!(p.city_code.as_deref(), Some("3020200"));
    }

    #[test]
    fn parses_large_report() {
        let r = normalize(LARGE).unwrap().unwrap();
        assert!(r.points.len() > 20);
        assert!(r.points.iter().all(|p| p.city_code.is_some() && !p.addr.ends_with('＊')));
    }

    /// 発生時刻と検知時刻が異なる遠地の地震: 統合キーは検知時刻(P2Pと一致)
    #[test]
    fn key_is_arrival_time_and_origin_time_is_kept() {
        let r = normalize(include_str!("../../tests/fixtures/jma_vxse53_offset_3min_foreign.xml"))
            .unwrap()
            .unwrap();
        assert_eq!(r.key_time, "2026-10-16T12:38:00+09:00");
        assert_eq!(r.origin_time.as_deref(), Some("2026-10-16T12:35:00+09:00"));
        assert_eq!(r.depth_km, Some(160.0));
    }

    #[test]
    fn cancelled_and_non_normal_reports_are_skipped() {
        assert!(normalize(&SMALL.replace("<InfoType>発表</InfoType>", "<InfoType>取消</InfoType>")).unwrap().is_none());
        assert!(normalize(&SMALL.replace("<Status>通常</Status>", "<Status>訓練</Status>")).unwrap().is_none());
    }

    #[test]
    fn coordinate_parsing() {
        assert_eq!(parse_coordinate("+34.1+135.1-10000/"), Some((34.1, 135.1, Some(10.0))));
        assert_eq!(parse_coordinate("+34.1+135.1/"), Some((34.1, 135.1, None)));
        assert_eq!(parse_coordinate("-5.5-150.25-80000/"), Some((-5.5, -150.25, Some(80.0))));
        assert_eq!(parse_coordinate("+34.1+135.1+0/"), Some((34.1, 135.1, Some(0.0))));
        assert_eq!(parse_coordinate(""), None);
    }

    #[test]
    fn unknown_magnitude_becomes_none() {
        let r = normalize(&SMALL.replace(">2.6</jmx_eb:Magnitude>", ">NaN</jmx_eb:Magnitude>")).unwrap().unwrap();
        assert_eq!(r.magnitude, None);
    }
}
