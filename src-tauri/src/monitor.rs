//! 強震モニタのポーリング。画面で必要なとき(強震モニタ表示中、または設定で常時取得)だけ動く。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;
use tokio::time::{interval, MissedTickBehavior};

use crate::sources::kmoni::{self, Station};

/// サーバーの最新時刻との差を取り直す間隔
const RESYNC: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize, Clone)]
struct Frame {
    /// 画像の時刻 "YYYYMMDDHHMMSS"(JST)
    time: String,
    /// 観測点ごとの震度。順序は `Monitor::stations` と同じ。データなしは null
    values: Vec<Option<f32>>,
}

#[derive(Default)]
pub struct Monitor {
    /// 設定: 常に取得する
    always: AtomicBool,
    /// 強震モニタの画面を表示中
    view: AtomicBool,
    wake: Notify,
    stations: Mutex<Vec<Station>>,
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl Monitor {
    pub fn set_always(&self, on: bool) {
        self.always.store(on, Ordering::Relaxed);
        self.wake.notify_one();
    }

    pub fn set_view(&self, on: bool) {
        self.view.store(on, Ordering::Relaxed);
        self.wake.notify_one();
    }

    fn active(&self) -> bool {
        self.always.load(Ordering::Relaxed) || self.view.load(Ordering::Relaxed)
    }

    pub fn stations(&self) -> Vec<Station> {
        self.stations.lock().unwrap().clone()
    }

    pub async fn run(self: Arc<Self>, app: AppHandle) {
        let client = reqwest::Client::builder()
            .user_agent(concat!("EQViewer/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(5))
            .build()
            .expect("http client");
        loop {
            if !self.active() {
                self.wake.notified().await;
                continue;
            }
            let mut ticker = interval(Duration::from_secs(1));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            // サーバー時刻 - ローカル時刻。None の間は最新時刻を問い合わせる。
            let mut offset: Option<i64> = None;
            let mut synced = Instant::now();
            let mut failures = 0u32;
            while self.active() {
                ticker.tick().await;
                if offset.is_none() || synced.elapsed() > RESYNC {
                    match kmoni::latest_time(&client).await {
                        Ok(server) => {
                            offset = Some(server - now_unix());
                            synced = Instant::now();
                        }
                        Err(e) => {
                            eprintln!("[kmoni] latest time: {e:#}");
                            continue;
                        }
                    }
                }
                let target = now_unix() + offset.unwrap_or(0);
                match kmoni::fetch_frame(&client, target).await {
                    Ok(Some(img)) => {
                        failures = 0;
                        self.publish(&app, &img, target);
                    }
                    // 画像がまだ無い(時刻が先に進みすぎた)。次の周期で時刻を取り直す。
                    Ok(None) => offset = None,
                    // 接続の切断・タイムアウトは一時的な失敗として、その秒の分だけ諦める。
                    // サーバーは断続的に接続を切るため、ここで時刻を取り直すと要求が倍に増える。
                    Err(e) => {
                        failures += 1;
                        if failures == 1 || failures.is_multiple_of(30) {
                            eprintln!("[kmoni] frame ({failures} in a row): {e:#}");
                        }
                    }
                }
            }
        }
    }

    fn publish(&self, app: &AppHandle, img: &image::RgbaImage, unix: i64) {
        let stations = {
            let mut st = self.stations.lock().unwrap();
            if st.is_empty() {
                *st = kmoni::stations();
                let _ = app.emit("kmoni-stations", st.clone());
            }
            st.clone()
        };
        let _ = app.emit(
            "kmoni-frame",
            Frame {
                time: kmoni::stamp(unix),
                values: kmoni::read(img, &stations),
            },
        );
    }
}
