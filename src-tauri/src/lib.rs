mod clock;
mod db;
mod eew;
mod history;
mod model;
mod monitor;
mod runtime;
mod settings;
mod sources;
mod time;
mod traveltime;
mod window;

use std::sync::Arc;
use std::time::Duration;

use tauri::{Emitter, Manager};

use clock::Clock;
use db::Db;
use monitor::Monitor;
use runtime::Sources;
use settings::Settings;
use std::sync::Mutex;
use eew::EewHub;
use model::{Earthquake, Eew};
use tauri_plugin_notification::NotificationExt;

/// P2P地震情報のレート制限(60req/分)に対し余裕を持たせた同期間隔
const SYNC_INTERVAL: Duration = Duration::from_secs(60);

#[tauri::command]
fn list_earthquakes(
    db: tauri::State<'_, Arc<Db>>,
    limit: u32,
    before: Option<String>,
) -> Result<Vec<Earthquake>, String> {
    db.list_earthquakes(limit.min(500), before.as_deref())
        .map_err(|e| e.to_string())
}

/// 震源の深さ(km)に対するP/S波の走時曲線
#[tauri::command]
fn travel_curve(depth_km: f64) -> traveltime::Curve {
    traveltime::curve(depth_km)
}

#[tauri::command]
fn active_eews(hub: tauri::State<'_, Arc<EewHub>>) -> Vec<Eew> {
    hub.active()
}

#[tauri::command]
fn get_settings(settings: tauri::State<'_, Mutex<Settings>>) -> Settings {
    settings.lock().unwrap().clone()
}

/// 設定を保存し、EEWソースを設定に合わせて起動し直す
#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    settings: tauri::State<'_, Mutex<Settings>>,
    sources: tauri::State<'_, Arc<Sources>>,
    monitor: tauri::State<'_, Arc<Monitor>>,
    new: Settings,
) -> Result<(), String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    new.save(&settings::path(&dir)).map_err(|e| e.to_string())?;
    sources.apply(&app, &new);
    monitor.set_always(new.kmoni_always);
    *settings.lock().unwrap() = new;
    Ok(())
}

#[tauri::command]
fn source_status(sources: tauri::State<'_, Arc<Sources>>) -> std::collections::BTreeMap<&'static str, sources::ws::Status> {
    sources.status()
}

#[tauri::command]
fn kmoni_stations(monitor: tauri::State<'_, Arc<Monitor>>) -> Vec<sources::kmoni::Station> {
    monitor.stations()
}

/// 強震モニタの画面を表示中かどうか(表示中だけ画像を取得する)
#[tauri::command]
fn kmoni_set_view(monitor: tauri::State<'_, Arc<Monitor>>, on: bool) {
    monitor.set_view(on);
}

/// 正しい時刻 − PCの時刻。測定前は None
#[tauri::command]
fn clock_offset(clock: tauri::State<'_, Arc<Clock>>) -> clock::Offset {
    clock.offset()
}

/// 開発時のみ: wolfx形式のJSONを受信したものとして投入する
#[cfg(debug_assertions)]
#[tauri::command]
fn debug_inject_eew(
    app: tauri::AppHandle,
    hub: tauri::State<'_, Arc<EewHub>>,
    json: String,
) -> Result<(), String> {
    let eew = sources::wolfx::parse(&json)
        .map_err(|e| e.to_string())?
        .ok_or("not an eew message")?;
    handle_eew(&app, &hub, eew);
    Ok(())
}

fn notify(app: &tauri::AppHandle, e: &Eew) {
    let title = if e.is_warning {
        "緊急地震速報(警報)"
    } else {
        "緊急地震速報(予報)"
    };
    let body = format!(
        "{} M{} 最大震度{}",
        e.hypocenter.as_deref().unwrap_or("震源不明"),
        e.magnitude.map_or("不明".into(), |m| format!("{m:.1}")),
        e.max_scale.as_deref().unwrap_or("不明"),
    );
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        eprintln!("notification failed: {err}");
    }
}

fn handle_eew(app: &tauri::AppHandle, hub: &EewHub, eew: Eew) {
    let Some(accepted) = hub.accept(eew) else {
        return;
    };
    // 今回受け付けた報が最新(受信時刻が最も新しい)
    if let Some(e) = accepted.active.iter().max_by_key(|e| e.received_at_ms) {
        let focus = window::wants_foreground(
            &app.state::<Mutex<Settings>>().lock().unwrap(),
            e,
            accepted.notify,
        );
        if focus {
            window::bring_to_front(app);
        }
        if accepted.notify {
            notify(app, e);
        }
    }
    let _ = app.emit("eew-updated", accepted.active);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 常駐するため多重起動はせず、2つ目の起動は既存のウィンドウを前面に出す
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::bring_to_front(app)
        }))
        .plugin(tauri_plugin_notification::init())
        .on_window_event(window::on_window_event)
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = Arc::new(Db::open(&dir.join("eqviewer.db"))?);
            app.manage(db.clone());

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::builder()
                    .user_agent(concat!("EQViewer/", env!("CARGO_PKG_VERSION")))
                    .timeout(Duration::from_secs(15))
                    .build()
                    .expect("http client");
                let mut jma = history::JmaState::default();
                loop {
                    // 情報源ごとに独立して同期する(片方が失敗してももう片方は進める)
                    let mut updated = 0;
                    match history::sync_p2p(&client, &db).await {
                        Ok(n) => updated += n,
                        Err(e) => eprintln!("history sync (p2p) failed: {e:#}"),
                    }
                    match history::sync_jma(&client, &db, &mut jma).await {
                        Ok(n) => updated += n,
                        Err(e) => eprintln!("history sync (jma) failed: {e:#}"),
                    }
                    if updated > 0 {
                        let _ = handle.emit("history-updated", updated);
                    }
                    tokio::time::sleep(SYNC_INTERVAL).await;
                }
            });

            let hub = Arc::new(EewHub::default());
            app.manage(hub.clone());
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let settings = Settings::load(&settings::path(&dir));
            // 速報の処理が設定を読むので、ソースを起動する前に登録する
            app.manage(Mutex::new(settings.clone()));
            let sources = Arc::new(Sources::new(tx));
            sources.apply(app.handle(), &settings);
            let monitor = Arc::new(Monitor::default());
            monitor.set_always(settings.kmoni_always);
            tauri::async_runtime::spawn(monitor.clone().run(app.handle().clone()));
            let clock = Arc::new(Clock::default());
            tauri::async_runtime::spawn(clock.clone().run(app.handle().clone()));
            app.manage(clock);
            app.manage(monitor);
            app.manage(sources);
            window::setup_tray(app.handle())?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(eew) = rx.recv().await {
                    handle_eew(&handle, &hub, eew);
                }
            });
            Ok(())
        })
        .invoke_handler(handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(debug_assertions)]
fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![list_earthquakes, active_eews, travel_curve, get_settings, save_settings, source_status, kmoni_stations, kmoni_set_view, clock_offset, debug_inject_eew]
}

#[cfg(not(debug_assertions))]
fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool {
    tauri::generate_handler![list_earthquakes, active_eews, travel_curve, get_settings, save_settings, source_status, kmoni_stations, kmoni_set_view, clock_offset]
}
