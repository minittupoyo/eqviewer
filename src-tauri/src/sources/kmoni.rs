//! 強震モニタ(防災科研 kmoni.bosai.go.jp)のリアルタイム震度画像。
//!
//! 画像(352x400, GIF)は観測点1つを3x3pxの正方形で描き、色が震度に対応する。
//! 観測点の緯度経度と画像上の位置は、ingen084/kyoshin-monitor-observation-points(MIT)の
//! 表を同梱して使う(tools/build-kmoni-points.py で生成)。
//! 通信はHTTPのみ(HTTPSは提供されていない)。

use anyhow::{bail, Context, Result};
use image::RgbaImage;
use serde::Serialize;

const BASE_URL: &str = "http://www.kmoni.bosai.go.jp";
pub const LATEST_URL: &str = "http://www.kmoni.bosai.go.jp/webservice/server/pros/latest.json";

/// 凡例のカラーバーを上(震度6.95)から下(-2.95)へ1pxずつ読んだRGB列
static SCALE: &[u8] = include_bytes!("../../data/kmoni_jma_scale.bin");
const SCALE_TOP: f32 = 7.0 - 1.5 / 28.0;
const SCALE_STEP: f32 = 1.0 / 28.0;
/// 凡例のどの色とも、これ以上離れた色は「データなし」とみなす(RGB距離)
const MAX_COLOR_DISTANCE: i32 = 40;

/// 観測点の位置(緯度経度)と、色を読む画像上のピクセル
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Station {
    pub lon: f64,
    pub lat: f64,
    #[serde(skip)]
    pub px: (u32, u32),
}

/// 観測点表(経度,緯度,画像のx,画像のy,色を読むxのずれ,色を読むyのずれ)。
/// 観測点が重なって描かれる場所では、中心から少しずらした画素が自分の色になる。
static POINTS: &str = include_str!("../../data/kmoni_points.csv");

pub fn stations() -> Vec<Station> {
    POINTS
        .lines()
        .filter_map(|l| {
            let mut f = l.split(',');
            let mut num = || f.next()?.parse::<f64>().ok();
            let (lon, lat, x, y, ox, oy) = (num()?, num()?, num()?, num()?, num()?, num()?);
            Some(Station { lon, lat, px: ((x + ox) as u32, (y + oy) as u32) })
        })
        .collect()
}

pub fn decode(bytes: &[u8]) -> Result<RgbaImage> {
    Ok(image::load_from_memory_with_format(bytes, image::ImageFormat::Gif)?.to_rgba8())
}


/// 色→リアルタイム震度。凡例に無い色(データなし)は None。小数第1位に丸める。
pub fn intensity(rgb: [u8; 3]) -> Option<f32> {
    let (mut best, mut best_d) = (0usize, i32::MAX);
    for (i, c) in SCALE.as_chunks::<3>().0.iter().enumerate() {
        let d: i32 = (0..3).map(|k| (rgb[k] as i32 - c[k] as i32).pow(2)).sum();
        if d < best_d {
            (best, best_d) = (i, d);
        }
    }
    if best_d > MAX_COLOR_DISTANCE * MAX_COLOR_DISTANCE {
        return None;
    }
    let v = SCALE_TOP - best as f32 * SCALE_STEP;
    Some((v * 10.0).round() / 10.0)
}

/// 観測点ごとの震度(中心ピクセルの色)
pub fn read(img: &RgbaImage, stations: &[Station]) -> Vec<Option<f32>> {
    stations
        .iter()
        .map(|s| {
            let p = img.get_pixel(s.px.0, s.px.1).0;
            (p[3] == 255).then(|| intensity([p[0], p[1], p[2]])).flatten()
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct Latest {
    latest_time: String,
}
use serde::Deserialize;

/// サーバーの最新データ時刻(JST, UNIX秒)
pub async fn latest_time(client: &reqwest::Client) -> Result<i64> {
    let l: Latest = client
        .get(LATEST_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let t = chrono::NaiveDateTime::parse_from_str(&l.latest_time, "%Y/%m/%d %H:%M:%S")
        .with_context(|| format!("invalid latest_time: {}", l.latest_time))?;
    Ok(t.and_utc().timestamp() - 9 * 3600)
}

/// UNIX秒(JSTの壁時計に直す)→ "YYYYMMDDHHMMSS"
pub fn stamp(unix: i64) -> String {
    chrono::DateTime::from_timestamp(unix + 9 * 3600, 0)
        .map(|t| t.naive_utc().format("%Y%m%d%H%M%S").to_string())
        .unwrap_or_default()
}

pub fn frame_url(unix: i64) -> String {
    let s = stamp(unix);
    format!("{BASE_URL}/data/map_img/RealTimeImg/jma_s/{}/{s}.jma_s.gif", &s[..8])
}

/// 指定時刻のフレームを取得する。まだ(または既に)存在しない場合(404)は None。
/// 接続の切断・タイムアウトなど一時的な失敗は Err。
pub async fn fetch_frame(client: &reqwest::Client, unix: i64) -> Result<Option<RgbaImage>> {
    let resp = client.get(frame_url(unix)).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        bail!("frame {} → {}", stamp(unix), resp.status());
    }
    decode(&resp.bytes().await?).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 実際の地震(2026-10-17 11:54 千葉県北東部 M4.8)の約50秒後のフレーム。色が多様で観測点も重なりやすい。
    fn quake() -> RgbaImage {
        decode(include_bytes!("../../tests/fixtures/kmoni_realtime_quake.gif")).unwrap()
    }

    fn quiet() -> RgbaImage {
        decode(include_bytes!("../../tests/fixtures/kmoni_realtime_quiet.gif")).unwrap()
    }

    #[test]
    fn image_has_expected_size() {
        assert_eq!(quiet().dimensions(), (352, 400));
    }

    #[test]
    fn table_has_over_a_thousand_stations_inside_japan() {
        let st = stations();
        assert!((1500..2000).contains(&st.len()), "found {}", st.len());
        for s in &st {
            assert!((122.0..147.0).contains(&s.lon) && (23.0..47.0).contains(&s.lat), "{s:?}");
            assert!(s.px.0 < 352 && s.px.1 < 400, "{s:?}");
        }
        // 沖縄本島(26.2N, 127.7E)の近くに観測点がある
        assert!(st.iter().any(|s| (s.lat - 26.3).abs() < 0.4 && (s.lon - 127.8).abs() < 0.4));
    }

    #[test]
    fn quiet_image_reads_as_low_intensity() {
        let img = quiet();
        let st = stations();
        let v = read(&img, &st);
        let known: Vec<f32> = v.iter().flatten().copied().collect();
        assert!(known.len() as f32 > st.len() as f32 * 0.9, "{}/{}", known.len(), st.len());
        let low = known.iter().filter(|i| **i < -1.0).count();
        assert!(low as f32 > known.len() as f32 * 0.9, "{low}/{}", known.len());
    }

    #[test]
    fn legend_colors_map_back_to_their_intensity() {
        // 凡例の上端付近(濃い赤)は震度7近く、下端付近(濃い青)は-3近く
        let top = &SCALE[..3];
        let bottom = &SCALE[SCALE.len() - 3..];
        assert!(intensity([top[0], top[1], top[2]]).unwrap() > 6.8);
        assert!(intensity([bottom[0], bottom[1], bottom[2]]).unwrap() < -2.8);
        // 凡例に無い色(白・灰色)はデータなし
        assert_eq!(intensity([255, 255, 255]), None);
        assert_eq!(intensity([128, 128, 128]), None);
    }

    #[test]
    fn stamp_formats_jst_wall_clock() {
        // 2026-10-17 12:46:00 JST = 03:46:00 UTC
        let unix = chrono::NaiveDate::from_ymd_opt(2026, 10, 17)
            .unwrap()
            .and_hms_opt(3, 46, 0)
            .unwrap()
            .and_utc()
            .timestamp();
        assert_eq!(stamp(unix), "20261017124600");
        assert!(frame_url(unix).ends_with("/jma_s/20261017/20261017124600.jma_s.gif"));
    }

    /// 実サーバから最新フレームを取得して読み取れること。`cargo test -- --ignored` で実行(ネットワーク使用)。
    #[tokio::test]
    #[ignore]
    async fn live_frame_is_readable() {
        let client = reqwest::Client::new();
        let t = latest_time(&client).await.unwrap();
        let img = fetch_frame(&client, t).await.unwrap().expect("latest frame exists");
        let st = stations();
        let known = read(&img, &st).into_iter().flatten().count();
        assert!(st.len() > 1000 && known > 1000, "{} stations, {known} readable", st.len());
    }

    /// 実際の地震(2026-10-17 11:54 紀伊水道 M2.6)のフレーム: 震度が最大の観測点が震央のすぐ近くにある。
    /// 色の対応づけと観測点の位置がそろって正しくないと成り立たない。
    #[test]
    fn quake_frame_peaks_at_the_epicenter() {
        let img = quake();
        let st = stations();
        let vals = read(&img, &st);
        let (top, v) = st
            .iter()
            .zip(&vals)
            .filter_map(|(s, v)| v.map(|v| (s, v)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        assert!(v > 0.0, "top intensity {v}");
        assert!((top.lat - 34.1).abs() < 0.1 && (top.lon - 135.1).abs() < 0.1, "top station at {},{}", top.lat, top.lon);
    }

    /// 地図の標準の表示(src/lib/mapControls.ts の JAPAN_BOUNDS)が、南西諸島を除く全観測点を覆う。
    /// 数字を変えるときは両方を揃えること。
    #[test]
    fn map_bounds_cover_every_station_except_the_ryukyu_islands() {
        const WEST: f64 = 128.6;
        const SOUTH: f64 = 30.2;
        const EAST: f64 = 145.85;
        const NORTH: f64 = 45.52;
        for s in stations().iter().filter(|s| s.lat >= 29.5) {
            assert!(
                (WEST..=EAST).contains(&s.lon) && (SOUTH..=NORTH).contains(&s.lat),
                "station outside the map bounds: {s:?}"
            );
        }
    }
}
