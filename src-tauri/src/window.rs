//! ウィンドウのトレイ常駐・前面表示。

use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Window, WindowEvent};

use crate::model::Eew;
use crate::settings::Settings;

const MAIN: &str = "main";

/// ウィンドウを表示して前面に出す。
/// Windowsは裏のアプリが勝手に前面へ出るのを制限し、フォーカスだけでは
/// タスクバーが点滅するだけになるため、一瞬だけ最前面にしてからフォーカスする。
pub fn bring_to_front(app: &AppHandle) {
    let Some(w) = app.get_webview_window(MAIN) else {
        return;
    };
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_always_on_top(true);
    let _ = w.set_focus();
    let _ = w.set_always_on_top(false);
}

/// 受信した速報でウィンドウを前面に出すべきか。
/// 新規・格上げの警報だけが対象(予報は通知のみ。取消・訓練は対象外)。
pub fn wants_foreground(settings: &Settings, eew: &Eew, is_new_or_upgraded: bool) -> bool {
    settings.focus_on_eew
        && is_new_or_upgraded
        && eew.is_warning
        && !eew.is_cancel
        && !eew.is_training
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "表示", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .tooltip("EQViewer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => bring_to_front(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                bring_to_front(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// 閉じるボタンの扱い。設定が有効ならトレイに格納する。
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        // 設定が未登録の起動直後は既定(トレイに格納)として扱う
        let to_tray = window
            .app_handle()
            .try_state::<Mutex<Settings>>()
            .is_none_or(|s| s.lock().unwrap().close_to_tray);
        if to_tray {
            api.prevent_close();
            let _ = window.hide();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eew(warn: bool) -> Eew {
        Eew {
            is_warning: warn,
            ..Default::default()
        }
    }

    #[test]
    fn only_new_warnings_bring_the_window_forward() {
        let s = Settings::default();
        assert!(wants_foreground(&s, &eew(true), true));
        // 予報は通知のみ
        assert!(!wants_foreground(&s, &eew(false), true));
        // 既知の警報の続報では奪わない
        assert!(!wants_foreground(&s, &eew(true), false));
    }

    #[test]
    fn cancel_training_and_disabled_setting_never_steal_focus() {
        let s = Settings::default();
        let mut c = eew(true);
        c.is_cancel = true;
        assert!(!wants_foreground(&s, &c, true));
        let mut t = eew(true);
        t.is_training = true;
        assert!(!wants_foreground(&s, &t, true));
        let off = Settings { focus_on_eew: false, ..Settings::default() };
        assert!(!wants_foreground(&off, &eew(true), true));
    }
}
