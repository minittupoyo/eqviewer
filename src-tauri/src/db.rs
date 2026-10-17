use std::path::Path;
use std::sync::Mutex;

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::model::Earthquake;

/// 履歴DB。`reports` に生電文、`earthquakes` に統合結果を保持する。
/// 両テーブルの `origin_time` 列は、同じ地震をまとめる統合キー(検知時刻)を表す(列名は旧版との互換のため据え置き)。
pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS reports (
                source TEXT NOT NULL,
                source_id TEXT NOT NULL,
                origin_time TEXT NOT NULL,
                payload TEXT NOT NULL,
                PRIMARY KEY (source, source_id)
            );
            CREATE INDEX IF NOT EXISTS reports_origin ON reports(origin_time);
            CREATE TABLE IF NOT EXISTS earthquakes (
                origin_time TEXT PRIMARY KEY,
                data TEXT NOT NULL
            );",
        )?;
        Ok(Self(Mutex::new(conn)))
    }

    pub fn has_report(&self, source: &str, source_id: &str) -> Result<bool> {
        let conn = self.0.lock().unwrap();
        Ok(conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM reports WHERE source = ?1 AND source_id = ?2)",
            params![source, source_id],
            |r| r.get(0),
        )?)
    }

    pub fn insert_report(
        &self,
        source: &str,
        source_id: &str,
        origin_time: &str,
        payload: &str,
    ) -> Result<()> {
        self.0.lock().unwrap().execute(
            "INSERT OR REPLACE INTO reports (source, source_id, origin_time, payload) VALUES (?1, ?2, ?3, ?4)",
            params![source, source_id, origin_time, payload],
        )?;
        Ok(())
    }

    /// 統合キーに属する全ソースの生電文 `(source, payload)`
    pub fn reports_for(&self, key: &str) -> Result<Vec<(String, String)>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare("SELECT source, payload FROM reports WHERE origin_time = ?1")?;
        let rows = stmt.query_map(params![key], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn put_earthquake(&self, eq: &Earthquake) -> Result<()> {
        self.0.lock().unwrap().execute(
            "INSERT OR REPLACE INTO earthquakes (origin_time, data) VALUES (?1, ?2)",
            params![eq.id, serde_json::to_string(eq)?],
        )?;
        Ok(())
    }

    /// 発生時刻の新しい順。`before` を指定するとそれより前のみ。
    pub fn list_earthquakes(&self, limit: u32, before: Option<&str>) -> Result<Vec<Earthquake>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT data FROM earthquakes WHERE (?1 IS NULL OR origin_time < ?1)
             ORDER BY origin_time DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![before, limit], |r| r.get::<_, String>(0))?;
        rows.map(|s| Ok(serde_json::from_str(&s?)?)).collect()
    }
}
