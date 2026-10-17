//! EEWソースの起動・停止と接続状態の管理。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc::UnboundedSender;

use crate::model::Eew;
use crate::settings::Settings;
use crate::sources::ws::{self, Keepalive, OnStatus, Options, Status};
use crate::sources::{axis, p2p_ws, wolfx};

/// 使うソースの名前。AXISが有効ならwolfxは使わない。P2P地震情報(警報のみ)は常に冗長化として使う。
pub fn active_sources(s: &Settings) -> Vec<&'static str> {
    vec![
        if s.axis_active() { axis::SOURCE } else { wolfx::SOURCE },
        p2p_ws::SOURCE,
    ]
}

pub struct Sources {
    tx: UnboundedSender<Eew>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    status: Arc<Mutex<BTreeMap<&'static str, Status>>>,
}

impl Sources {
    pub fn new(tx: UnboundedSender<Eew>) -> Self {
        Self {
            tx,
            tasks: Mutex::default(),
            status: Arc::default(),
        }
    }

    pub fn status(&self) -> BTreeMap<&'static str, Status> {
        self.status.lock().unwrap().clone()
    }

    /// 設定に合わせて全ソースを止めて起動し直す。
    pub fn apply(&self, app: &AppHandle, settings: &Settings) {
        let mut tasks = self.tasks.lock().unwrap();
        for t in tasks.drain(..) {
            t.abort();
        }
        self.status.lock().unwrap().clear();
        let _ = app.emit("source-status", self.status());

        for name in active_sources(settings) {
            let on_status: OnStatus = {
                let (app, map) = (app.clone(), self.status.clone());
                Arc::new(move |st| {
                    let snapshot = {
                        let mut m = map.lock().unwrap();
                        m.insert(name, st);
                        m.clone()
                    };
                    let _ = app.emit("source-status", snapshot);
                })
            };
            let tx = self.tx.clone();
            let handle = match name {
                axis::SOURCE => {
                    let token = settings.axis_token.trim().to_string();
                    tauri::async_runtime::spawn(ws::run(
                        Options {
                            name,
                            keepalive: Keepalive::Text(Duration::from_secs(60), "hb"),
                            idle_timeout: None,
                            parse: axis::parse,
                        },
                        move || {
                            let token = token.clone();
                            async move { axis::connect(&token).await }
                        },
                        tx,
                        on_status,
                    ))
                }
                wolfx::SOURCE => tauri::async_runtime::spawn(ws::run(
                    Options {
                        name,
                        keepalive: Keepalive::Text(Duration::from_secs(30), "ping"),
                        idle_timeout: Some(Duration::from_secs(150)),
                        parse: wolfx::parse,
                    },
                    || ws::plain(wolfx::URL),
                    tx,
                    on_status,
                )),
                _ => tauri::async_runtime::spawn(ws::run(
                    Options {
                        name,
                        keepalive: Keepalive::Frame(Duration::from_secs(30)),
                        idle_timeout: Some(Duration::from_secs(90)),
                        parse: p2p_ws::parse,
                    },
                    || ws::plain(p2p_ws::URL),
                    tx,
                    on_status,
                )),
            };
            tasks.push(handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_replaces_wolfx_and_p2p_always_stays() {
        let mut s = Settings::default();
        assert_eq!(active_sources(&s), ["wolfx", "p2pquake"]);
        s.axis_enabled = true;
        assert_eq!(active_sources(&s), ["wolfx", "p2pquake"]);
        s.axis_token = "t".into();
        assert_eq!(active_sources(&s), ["axis", "p2pquake"]);
    }
}
