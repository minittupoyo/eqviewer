//! PCの時計ずれの測定。NICTの公開NTPサーバーにSNTPで問い合わせ、
//! 「正しい時刻 − PCの時刻」(ミリ秒)を求める。緊急地震速報の経過時間(到達円)の計算に使う。

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::net::UdpSocket;
use tokio::time::{sleep, timeout};

const SERVER: &str = "ntp.nict.jp:123";
/// NTPエポック(1900)とUNIXエポック(1970)の差(秒)
const NTP_UNIX_DELTA: f64 = 2_208_988_800.0;
/// 1回の測定で取る標本数。往復遅延が最小のものを採用する。
const SAMPLES: usize = 4;
const RESYNC: Duration = Duration::from_secs(30 * 60);
const RETRY: Duration = Duration::from_secs(60);

#[derive(Debug, Default)]
pub struct Clock {
    offset_ms: AtomicI64,
    measured: std::sync::atomic::AtomicBool,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Offset {
    /// 正しい時刻 − PCの時刻(ms)。測定できていなければ None
    pub offset_ms: Option<i64>,
}

/// SNTPの4つのタイムスタンプ(UNIX秒)から、時計のずれ(秒)と往復遅延(秒)を求める。
/// t1: 要求送信(PC) / t2: 受信(サーバー) / t3: 応答送信(サーバー) / t4: 応答受信(PC)
pub fn offset_and_delay(t1: f64, t2: f64, t3: f64, t4: f64) -> (f64, f64) {
    (((t2 - t1) + (t3 - t4)) / 2.0, (t4 - t1) - (t3 - t2))
}

fn unix_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// NTPの64bitタイムスタンプ(秒32bit+小数32bit)をUNIX秒にする
fn ntp_time(b: &[u8]) -> f64 {
    let secs = u32::from_be_bytes(b[0..4].try_into().unwrap()) as f64;
    let frac = u32::from_be_bytes(b[4..8].try_into().unwrap()) as f64;
    secs - NTP_UNIX_DELTA + frac / 4_294_967_296.0
}

/// 応答パケットから (t2, t3) を取り出す。サーバー応答でない・Kiss-o'-Deathなどは拒否する。
pub fn parse_response(p: &[u8]) -> Result<(f64, f64)> {
    if p.len() < 48 {
        bail!("short NTP response ({} bytes)", p.len());
    }
    if p[0] & 7 != 4 {
        bail!("not a server-mode response");
    }
    if p[1] == 0 {
        bail!("kiss-o'-death or unsynchronized server");
    }
    let (t2, t3) = (ntp_time(&p[32..40]), ntp_time(&p[40..48]));
    if t3 == 0.0 - NTP_UNIX_DELTA {
        bail!("server has no transmit timestamp");
    }
    Ok((t2, t3))
}

async fn sample(sock: &UdpSocket) -> Result<(f64, f64)> {
    let mut req = [0u8; 48];
    req[0] = 0x1B; // LI=0, VN=3, Mode=3(client)
    let t1 = unix_now();
    sock.send(&req).await?;
    let mut buf = [0u8; 64];
    let n = timeout(Duration::from_secs(3), sock.recv(&mut buf))
        .await
        .context("NTP timeout")??;
    let t4 = unix_now();
    let (t2, t3) = parse_response(&buf[..n])?;
    Ok(offset_and_delay(t1, t2, t3, t4))
}

/// 正しい時刻 − PCの時刻(ms)。往復遅延が最小の標本を採用する。
pub async fn measure_offset_ms() -> Result<i64> {
    let sock = UdpSocket::bind("0.0.0.0:0").await?;
    sock.connect(SERVER).await?;
    let mut best: Option<(f64, f64)> = None;
    for _ in 0..SAMPLES {
        if let Ok((off, delay)) = sample(&sock).await
            && best.is_none_or(|(_, d)| delay < d)
        {
            best = Some((off, delay));
        }
        sleep(Duration::from_millis(200)).await;
    }
    let (off, _) = best.context("no NTP sample")?;
    Ok((off * 1000.0).round() as i64)
}

impl Clock {
    pub fn offset(&self) -> Offset {
        Offset {
            offset_ms: self
                .measured
                .load(Ordering::Relaxed)
                .then(|| self.offset_ms.load(Ordering::Relaxed)),
        }
    }

    /// 起動時と30分ごとに測定し、結果を `clock-offset` イベントで通知する。
    pub async fn run(self: std::sync::Arc<Self>, app: AppHandle) {
        loop {
            let wait = match measure_offset_ms().await {
                Ok(ms) => {
                    self.offset_ms.store(ms, Ordering::Relaxed);
                    self.measured.store(true, Ordering::Relaxed);
                    let _ = app.emit("clock-offset", self.offset());
                    RESYNC
                }
                Err(e) => {
                    eprintln!("[clock] {e:#}");
                    RETRY
                }
            };
            sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_is_zero_for_synchronized_clocks_with_symmetric_delay() {
        // 往路・復路とも0.05秒
        let (off, delay) = offset_and_delay(100.0, 100.05, 100.06, 100.11);
        assert!(off.abs() < 1e-9, "{off}");
        assert!((delay - 0.10).abs() < 1e-9, "{delay}");
    }

    #[test]
    fn offset_is_positive_when_the_pc_clock_is_behind() {
        // PCが2.5秒遅れている: PCの時刻で t1=97.5, t4=97.61。サーバーは t2=100.05, t3=100.06
        let (off, _) = offset_and_delay(97.5, 100.05, 100.06, 97.61);
        assert!((off - 2.5).abs() < 1e-9, "{off}");
    }

    fn packet(mode: u8, stratum: u8, t2: f64, t3: f64) -> [u8; 48] {
        let mut p = [0u8; 48];
        p[0] = 0x18 | mode; // VN=3
        p[1] = stratum;
        for (range, t) in [(32..40, t2), (40..48, t3)] {
            let ntp = t + NTP_UNIX_DELTA;
            let secs = ntp.trunc() as u32;
            let frac = (ntp.fract() * 4_294_967_296.0) as u32;
            p[range.start..range.start + 4].copy_from_slice(&secs.to_be_bytes());
            p[range.start + 4..range.end].copy_from_slice(&frac.to_be_bytes());
        }
        p
    }

    #[test]
    fn parses_timestamps_with_subsecond_precision() {
        let (t2, t3) = parse_response(&packet(4, 1, 1_760_000_000.25, 1_760_000_000.5)).unwrap();
        assert!((t2 - 1_760_000_000.25).abs() < 1e-6);
        assert!((t3 - 1_760_000_000.5).abs() < 1e-6);
    }

    #[test]
    fn rejects_bad_responses() {
        assert!(parse_response(&[0u8; 10]).is_err());
        // クライアントモードのまま(サーバー応答でない)
        assert!(parse_response(&packet(3, 1, 1.0e9, 1.0e9)).is_err());
        // stratum 0 = Kiss-o'-Death
        assert!(parse_response(&packet(4, 0, 1.0e9, 1.0e9)).is_err());
    }

    /// 実サーバーに問い合わせて妥当な値が返ること。`cargo test -- --ignored`(UDP 123を使用)
    #[tokio::test]
    #[ignore]
    async fn measures_against_nict() {
        let ms = measure_offset_ms().await.unwrap();
        assert!(ms.abs() < 3_600_000, "{ms}");
        eprintln!("offset {ms} ms");
    }
}
