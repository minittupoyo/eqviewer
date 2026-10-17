use anyhow::{Context, Result};
use chrono::NaiveDateTime;

/// "2026/10/17 11:54:00" (JST, 小数秒は任意) → "2026-10-17T11:54:00+09:00"
pub fn jst_to_rfc3339(s: &str) -> Result<String> {
    let t = NaiveDateTime::parse_from_str(s, "%Y/%m/%d %H:%M:%S%.f")
        .with_context(|| format!("invalid time: {s}"))?;
    Ok(t.format("%Y-%m-%dT%H:%M:%S+09:00").to_string())
}
