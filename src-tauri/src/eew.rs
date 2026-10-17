//! 複数ソースのEEWを統合し、進行中の速報を保持する。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::Eew;

/// 受信からこの時間を過ぎた速報は破棄する
const RETAIN_MS: i64 = 5 * 60 * 1000;

#[derive(Default)]
pub struct EewHub(Mutex<HashMap<String, Eew>>);

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// 受理した結果
pub struct Accepted {
    /// 通知すべきか(新規イベント、または予報→警報への格上げ)
    pub notify: bool,
    pub active: Vec<Eew>,
}

impl EewHub {
    /// 報を受け付ける。同じイベントの古い報・重複報(別ソース含む)は None。
    pub fn accept(&self, mut eew: Eew) -> Option<Accepted> {
        eew.received_at_ms = now_ms();
        let mut map = self.0.lock().unwrap();
        let notify = match map.get(&eew.event_id) {
            Some(cur) if eew.serial <= cur.serial || cur.is_cancel => return None,
            Some(cur) => !cur.is_warning && eew.is_warning,
            None => true,
        } && !eew.is_cancel
            && !eew.is_training;
        map.insert(eew.event_id.clone(), eew);
        Some(Accepted {
            notify,
            active: Self::snapshot(&mut map),
        })
    }

    pub fn active(&self) -> Vec<Eew> {
        Self::snapshot(&mut self.0.lock().unwrap())
    }

    fn snapshot(map: &mut HashMap<String, Eew>) -> Vec<Eew> {
        let limit = now_ms() - RETAIN_MS;
        map.retain(|_, e| e.received_at_ms >= limit);
        let mut v: Vec<_> = map.values().cloned().collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.received_at_ms));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eew(id: &str, serial: u32, warn: bool) -> Eew {
        Eew {
            event_id: id.into(),
            serial,
            is_warning: warn,
            ..Default::default()
        }
    }

    #[test]
    fn first_report_notifies_and_duplicates_are_ignored() {
        let hub = EewHub::default();
        assert!(hub.accept(eew("a", 1, false)).unwrap().notify);
        assert!(hub.accept(eew("a", 1, false)).is_none());
        assert!(hub.accept(eew("a", 0, false)).is_none());
    }

    #[test]
    fn upgrade_to_warning_notifies_again_but_plain_update_does_not() {
        let hub = EewHub::default();
        hub.accept(eew("a", 1, false));
        assert!(!hub.accept(eew("a", 2, false)).unwrap().notify);
        let a = hub.accept(eew("a", 3, true)).unwrap();
        assert!(a.notify);
        assert_eq!(a.active.len(), 1);
        assert!(!hub.accept(eew("a", 4, true)).unwrap().notify);
    }

    #[test]
    fn cancel_and_training_do_not_notify() {
        let hub = EewHub::default();
        let mut c = eew("a", 1, false);
        c.is_cancel = true;
        assert!(!hub.accept(c).unwrap().notify);
        let mut t = eew("b", 1, true);
        t.is_training = true;
        assert!(!hub.accept(t).unwrap().notify);
    }

    #[test]
    fn old_events_are_pruned() {
        let hub = EewHub::default();
        hub.accept(eew("a", 1, false));
        hub.0.lock().unwrap().get_mut("a").unwrap().received_at_ms -= RETAIN_MS + 1;
        assert!(hub.active().is_empty());
    }
}
