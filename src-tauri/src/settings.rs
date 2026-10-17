use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub axis_enabled: bool,
    pub axis_token: String,
    /// 強震モニタを画面表示に関係なく常時取得する
    pub kmoni_always: bool,
    /// ウィンドウを閉じてもトレイに常駐する(false なら終了)
    pub close_to_tray: bool,
    /// 緊急地震速報(警報)の受信時にウィンドウを前面に出す
    pub focus_on_eew: bool,
    /// 自宅地点(緯度・経度)。設定されていれば、緊急地震速報で波の到達までの残り時間を出す
    /// 緊急地震速報の受信音を鳴らす
    pub sound_enabled: bool,
    /// 受信音の音量(0〜100)
    pub sound_volume: u8,
    pub home_lat: Option<f64>,
    pub home_lon: Option<f64>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            axis_enabled: false,
            axis_token: String::new(),
            kmoni_always: false,
            close_to_tray: true,
            focus_on_eew: true,
            sound_enabled: true,
            sound_volume: 70,
            home_lat: None,
            home_lon: None,
        }
    }
}

impl Settings {
    /// AXISを使う設定か(有効かつトークンが入力済み)
    pub fn axis_active(&self) -> bool {
        self.axis_enabled && !self.axis_token.trim().is_empty()
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_requires_both_flag_and_token() {
        let mut s = Settings::default();
        assert!(!s.axis_active());
        s.axis_enabled = true;
        assert!(!s.axis_active());
        s.axis_token = "  ".into();
        assert!(!s.axis_active());
        s.axis_token = "tok".into();
        assert!(s.axis_active());
        s.axis_enabled = false;
        assert!(!s.axis_active());
    }

    #[test]
    fn roundtrip_and_missing_file() {
        let dir = std::env::temp_dir().join(format!("eqviewer-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = path(&dir);
        assert_eq!(Settings::load(&p), Settings::default());
        let s = Settings {
            axis_enabled: true,
            axis_token: "t".into(),
            kmoni_always: true,
            close_to_tray: false,
            focus_on_eew: false,
            sound_enabled: false,
            sound_volume: 30,
            home_lat: Some(35.0),
            home_lon: Some(139.0),
        };
        s.save(&p).unwrap();
        assert_eq!(Settings::load(&p), s);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn background_options_default_on_and_old_files_get_defaults() {
        let d = Settings::default();
        assert!(d.close_to_tray && d.focus_on_eew);
        // 以前のバージョンの設定ファイル(新しい項目なし)でも既定値が入る
        let old: Settings = serde_json::from_str(r#"{"axis_enabled":true,"axis_token":"t"}"#).unwrap();
        assert!(old.axis_enabled && old.close_to_tray && old.focus_on_eew);
        assert_eq!((old.home_lat, old.home_lon), (None, None));
        assert!(old.sound_enabled && old.sound_volume == 70);
    }
}
