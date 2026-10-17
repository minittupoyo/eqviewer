//! 履歴の同期。DBを正とし、未取得の電文だけを追記する。

use std::collections::BTreeSet;

use anyhow::Result;

use crate::db::Db;
use crate::model::{merge, Report};
use crate::sources::{jma_xml, p2p};

const PAGE: u32 = 100;
/// 初回(DB空)でも取得する最大ページ数。サービスへの負荷を抑える。
const MAX_PAGES: u32 = 5;

/// 未取得の電文を取り込み、更新した地震の数を返す。
/// 既知の電文を含むページに到達した時点で打ち切る(差分取得)。
pub async fn sync_p2p(client: &reqwest::Client, db: &Db) -> Result<usize> {
    let mut touched = BTreeSet::new();
    for page in 0..MAX_PAGES {
        let quakes = p2p::fetch_history(client, page * PAGE, PAGE).await?;
        let mut reached_known = false;
        for (q, raw) in &quakes {
            if db.has_report(p2p::SOURCE, &q.id)? {
                reached_known = true;
                continue;
            }
            let Ok(r) = p2p::normalize(q) else { continue };
            db.insert_report(p2p::SOURCE, &q.id, &r.key_time, raw)?;
            touched.insert(r.key_time);
        }
        if reached_known || (quakes.len() as u32) < PAGE {
            break;
        }
    }
    for t in &touched {
        rebuild(db, t)?;
    }
    Ok(touched.len())
}

/// 気象庁XMLの同期状態
#[derive(Default)]
pub struct JmaState {
    /// 長期フィード(約1週間分)を取り込み済みか。取りこぼしがあれば次の周期でやり直す。
    long_done: bool,
    /// 高頻度フィードのETag(未更新なら304で済ませる)
    etag: Option<String>,
}

/// 気象庁XMLの未取得の電文を取り込み、更新した地震の数を返す。
/// 起動直後は長期フィードで過去分を補完し、以降は高頻度フィードだけを見る。
pub async fn sync_jma(client: &reqwest::Client, db: &Db, st: &mut JmaState) -> Result<usize> {
    let (url, etag) = if st.long_done {
        (jma_xml::FEED_SHORT, st.etag.as_deref())
    } else {
        (jma_xml::FEED_LONG, None)
    };
    let Some((xml, new_etag)) = jma_xml::fetch_feed(client, url, etag).await? else {
        return Ok(0);
    };
    let mut touched = BTreeSet::new();
    let mut failed = 0;
    for entry in jma_xml::parse_feed(&xml)? {
        if db.has_report(jma_xml::SOURCE, &entry.id)? {
            continue;
        }
        let body = match jma_xml::fetch_report(client, &entry.url).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[jma] {}: {e:#}", entry.id);
                failed += 1;
                continue;
            }
        };
        match jma_xml::normalize(&body) {
            Ok(Some(r)) => {
                db.insert_report(jma_xml::SOURCE, &entry.id, &r.key_time, &body)?;
                touched.insert(r.key_time);
            }
            // 取消・訓練など対象外。再取得しないよう既知として記録する。
            Ok(None) => db.insert_report(jma_xml::SOURCE, &entry.id, "", "")?,
            Err(e) => {
                eprintln!("[jma] {}: unparsable: {e:#}", entry.id);
                failed += 1;
            }
        }
        // 気象庁サーバーへの負荷を抑える
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    for t in &touched {
        rebuild(db, t)?;
    }
    if failed == 0 {
        if st.long_done {
            st.etag = new_etag;
        }
        st.long_done = true;
    }
    Ok(touched.len())
}

/// 統合キーに属する全ソースの電文から統合結果を作り直して保存する。
pub fn rebuild(db: &Db, key: &str) -> Result<()> {
    let reports: Vec<Report> = db
        .reports_for(key)?
        .iter()
        .filter_map(|(source, payload)| match source.as_str() {
            p2p::SOURCE => serde_json::from_str::<p2p::Quake>(payload)
                .ok()
                .and_then(|q| p2p::normalize(&q).ok()),
            jma_xml::SOURCE => jma_xml::normalize(payload).ok().flatten(),
            _ => None,
        })
        .collect();
    if !reports.is_empty() {
        db.put_earthquake(&merge(key, reports))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuild_from_fixture_is_listed_newest_first() {
        let db = Db::open_in_memory().unwrap();
        let raw: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../tests/fixtures/p2p_history_551.json")).unwrap();
        let mut times = BTreeSet::new();
        for v in raw {
            let q: p2p::Quake = serde_json::from_value(v.clone()).unwrap();
            let r = p2p::normalize(&q).unwrap();
            db.insert_report(p2p::SOURCE, &q.id, &r.key_time, &v.to_string())
                .unwrap();
            times.insert(r.key_time);
        }
        for t in &times {
            rebuild(&db, t).unwrap();
        }
        let list = db.list_earthquakes(1000, None).unwrap();
        assert_eq!(list.len(), times.len());
        assert!(list.windows(2).all(|w| w[0].id > w[1].id));
        let older = db.list_earthquakes(1000, Some(&list[3].id)).unwrap();
        assert_eq!(older.len(), list.len() - 4);
    }

    fn p2p_db() -> (Db, BTreeSet<String>) {
        let db = Db::open_in_memory().unwrap();
        let raw: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../tests/fixtures/p2p_history_551.json")).unwrap();
        let mut keys = BTreeSet::new();
        for v in raw {
            let q: p2p::Quake = serde_json::from_value(v.clone()).unwrap();
            let r = p2p::normalize(&q).unwrap();
            db.insert_report(p2p::SOURCE, &q.id, &r.key_time, &v.to_string()).unwrap();
            keys.insert(r.key_time);
        }
        for k in &keys {
            rebuild(&db, k).unwrap();
        }
        (db, keys)
    }

    fn add_jma(db: &Db, id: &str, xml: &str) -> String {
        let r = jma_xml::normalize(xml).unwrap().unwrap();
        db.insert_report(jma_xml::SOURCE, id, &r.key_time, xml).unwrap();
        rebuild(db, &r.key_time).unwrap();
        r.key_time
    }

    /// 発生時刻と検知時刻が3分ずれる地震でも、P2Pと気象庁XMLが1件にまとまる
    #[test]
    fn jma_report_merges_into_the_same_earthquake_as_p2p() {
        let (db, keys) = p2p_db();
        let key = add_jma(&db, "a", include_str!("../tests/fixtures/jma_vxse53_offset_3min_foreign.xml"));
        assert!(keys.contains(&key), "P2Pにも同じキーの地震がある");
        let list = db.list_earthquakes(1000, None).unwrap();
        assert_eq!(list.len(), keys.len(), "重複して増えていない");
        let eq = list.iter().find(|e| e.id == key).unwrap();
        assert_eq!(eq.origin_time, "2026-10-16T12:35:00+09:00", "表示は気象庁の発生時刻");
        assert_eq!(eq.name.as_deref(), Some("カムチャツカ半島付近"));
        assert_eq!(eq.depth_km, Some(160.0));
    }

    #[test]
    fn jma_fills_a_gap_when_p2p_is_missing() {
        let db = Db::open_in_memory().unwrap();
        let key = add_jma(&db, "a", include_str!("../tests/fixtures/jma_vxse53_small.xml"));
        let list = db.list_earthquakes(10, None).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, key);
        assert_eq!(list[0].points.len(), 3);
        assert!(list[0].points.iter().all(|p| p.city_code.is_some()));
    }

    /// 気象庁XMLの観測点(市町村コードつき)がP2Pの観測点を置き換える
    #[test]
    fn jma_points_replace_p2p_points() {
        let (db, _) = p2p_db();
        let key = add_jma(&db, "a", include_str!("../tests/fixtures/jma_vxse53_small.xml"));
        let eq = db.list_earthquakes(1000, None).unwrap().into_iter().find(|e| e.id == key).unwrap();
        assert!(!eq.points.is_empty() && eq.points.iter().all(|p| p.city_code.is_some()));
    }

    /// 実サーバーから取り込めること(ネットワーク使用)。`cargo test -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn live_sync_imports_then_becomes_idempotent() {
        let client = reqwest::Client::new();
        let db = Db::open_in_memory().unwrap();
        let mut st = JmaState::default();
        let n = sync_jma(&client, &db, &mut st).await.unwrap();
        assert!(n >= 5, "imported {n}");
        let first = db.list_earthquakes(1000, None).unwrap();
        assert!(first.iter().all(|e| e.max_scale.is_some() && !e.points.is_empty()));
        // 2回目は高頻度フィードのみ。新しい地震が無ければ0件で、件数も変わらない
        sync_jma(&client, &db, &mut st).await.unwrap();
        let second = db.list_earthquakes(1000, None).unwrap();
        assert!(second.len() <= first.len() + 1);
        eprintln!("{} earthquakes imported", first.len());
    }
}
