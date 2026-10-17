use serde::{Deserialize, Serialize};

/// 観測点・地域ごとの震度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub pref: String,
    pub addr: String,
    pub scale: String,
    pub is_area: bool,
    /// 市町村コード(気象庁XML由来。地図の `regioncode` と同じ)
    #[serde(default)]
    pub city_code: Option<String>,
}

/// 複数ソース・複数電文を統合した地震情報。
/// `id` は統合キー(検知時刻 = 気象庁XMLの ArrivalTime = P2Pの earthquake.time。RFC3339, +09:00)。
/// `origin_time` は表示用の発生時刻(気象庁XMLがあればその OriginTime、なければ `id`)。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Earthquake {
    pub id: String,
    pub origin_time: String,
    pub name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub depth_km: Option<f64>,
    pub magnitude: Option<f64>,
    /// 最大震度: "1".."4", "5-", "5+", "6-", "6+", "7", "5-over"(5弱以上と推定)
    pub max_scale: Option<String>,
    pub domestic_tsunami: Option<String>,
    pub foreign_tsunami: Option<String>,
    pub points: Vec<Point>,
}

/// 1つのソースの1電文を正規化したもの。欠損は None / 空。
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// 同じ地震の電文をまとめるキー(検知時刻)
    pub key_time: String,
    /// 発生時刻(わかる場合のみ)
    pub origin_time: Option<String>,
    pub issue_time: String,
    /// 大きいほど情報が確定的(マージ時に優先)
    pub priority: u8,
    pub name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub depth_km: Option<f64>,
    pub magnitude: Option<f64>,
    pub max_scale: Option<String>,
    pub domestic_tsunami: Option<String>,
    pub foreign_tsunami: Option<String>,
    pub points: Vec<Point>,
}

/// 電文を優先度順(低→高)・発表時刻順に適用して1件にまとめる。
pub fn merge(key_time: &str, mut reports: Vec<Report>) -> Earthquake {
    reports.sort_by(|a, b| (a.priority, &a.issue_time).cmp(&(b.priority, &b.issue_time)));
    let mut eq = Earthquake {
        id: key_time.to_string(),
        origin_time: key_time.to_string(),
        ..Default::default()
    };
    for r in reports {
        macro_rules! over {
            ($($f:ident),*) => { $(if r.$f.is_some() { eq.$f = r.$f; })* };
        }
        if let Some(t) = r.origin_time {
            eq.origin_time = t;
        }
        over!(name, latitude, longitude, depth_km, magnitude, max_scale);
        for (dst, src) in [
            (&mut eq.domestic_tsunami, r.domestic_tsunami),
            (&mut eq.foreign_tsunami, r.foreign_tsunami),
        ] {
            if src.as_deref().is_some_and(|s| s != "Unknown") || dst.is_none() {
                *dst = src.or(dst.take());
            }
        }
        if !r.points.is_empty() {
            eq.points = r.points;
        }
    }
    eq
}

/// 緊急地震速報の予測震度地域
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForecastArea {
    pub name: String,
    pub scale_from: Option<String>,
    /// "over"(以上) を取り得る
    pub scale_to: Option<String>,
    /// 警報(true)か予報(false)か
    pub is_warning: bool,
    /// 既に主要動が到達していると配信元が示しているか
    pub arrived: bool,
}

/// 緊急地震速報(正規化後)。複数ソース共通。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Eew {
    pub source: String,
    pub event_id: String,
    pub serial: u32,
    pub origin_time: String,
    pub report_time: String,
    pub is_warning: bool,
    pub is_final: bool,
    pub is_cancel: bool,
    pub is_training: bool,
    pub hypocenter: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub depth_km: Option<f64>,
    pub magnitude: Option<f64>,
    /// 最大震度: "1".."4", "5-", "5+", "6-", "6+", "7"
    pub max_scale: Option<String>,
    pub areas: Vec<ForecastArea>,
    /// 受信した時刻(UNIX ms)。表示期間の判定に使う
    pub received_at_ms: i64,
}

/// 震度の大小比較用。未知の値は None。
pub fn scale_rank(s: &str) -> Option<usize> {
    ["0", "1", "2", "3", "4", "5-", "5+", "6-", "6+", "7"]
        .iter()
        .position(|x| *x == s)
}

/// 予測最大震度が5弱以上(=警報の対象)か
pub fn is_warning_scale(s: Option<&str>) -> bool {
    s.and_then(scale_rank).is_some_and(|r| r >= 5)
}
