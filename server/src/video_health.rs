//! Здоровье источников видео: сколько раз подряд источник не ответил и как быстро отвечал. Источник,
//! упавший три раза подряд, на 10 минут выпадает из поиска (не тормозит остальных), потом ему даётся
//! одна пробная попытка. Состояние видно администратору («Обзор») и зрителю (блок «источник не найден»).

use crate::util::now_ms;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const OPEN_AFTER: u32 = 3;
const PAUSE_MS: i64 = 600_000;

/// Источники поиска: (ключ, название для зрителя).
pub const SITES: &[(&str, &str)] = &[
    ("lordfilm", "LordFilm"),
    ("rutube", "Rutube"),
    ("vk", "VK Видео"),
    ("archive", "Internet Archive"),
    ("peertube", "PeerTube"),
    ("commons", "Wikimedia Commons"),
];

#[derive(Default)]
struct Entry {
    fails: u32,
    open_until: i64,
    last_ok: bool,
    seen: bool,
    ok_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SourceState {
    pub site: String,
    pub label: String,
    /// `ok`, `down` (выключен на 10 минут) или `unknown` (ещё не отвечал).
    pub state: &'static str,
    pub fails: u32,
    pub ok_ms: Option<u64>,
}

fn entries() -> &'static Mutex<HashMap<String, Entry>> {
    static ENTRIES: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();
    ENTRIES.get_or_init(Mutex::default)
}

pub fn record_at(site: &str, ok: bool, ms: u64, now: i64) {
    let mut all = entries().lock();
    let e = all.entry(site.to_string()).or_default();
    e.seen = true;
    e.last_ok = ok;
    if ok {
        e.fails = 0;
        e.open_until = 0;
        e.ok_ms = Some(ms);
    } else {
        e.fails += 1;
        if e.fails >= OPEN_AFTER {
            e.open_until = now + PAUSE_MS;
        }
    }
}

pub fn record(site: &str, ok: bool, ms: u64) {
    record_at(site, ok, ms, now_ms());
}

/// Выключатель открыт: источник сейчас пропускаем.
pub fn open_at(site: &str, now: i64) -> bool {
    entries().lock().get(site).is_some_and(|e| e.open_until > now)
}

pub fn open(site: &str) -> bool {
    open_at(site, now_ms())
}

pub fn snapshot_at(now: i64) -> Vec<SourceState> {
    let all = entries().lock();
    SITES
        .iter()
        .map(|(site, label)| {
            let e = all.get(*site);
            let state = match e {
                Some(e) if e.open_until > now => "down",
                Some(e) if e.seen && e.last_ok => "ok",
                _ => "unknown",
            };
            SourceState {
                site: site.to_string(),
                label: label.to_string(),
                state,
                fails: e.map_or(0, |e| e.fails),
                ok_ms: e.and_then(|e| e.ok_ms),
            }
        })
        .collect()
}

pub fn snapshot() -> Vec<SourceState> {
    snapshot_at(now_ms())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaker_opens_after_three_failures_and_success_resets() {
        let now = 1_000_000;
        record_at("t-open", false, 100, now);
        record_at("t-open", false, 100, now);
        assert!(!open_at("t-open", now), "две ошибки - ещё работает");
        record_at("t-open", false, 100, now);
        assert!(open_at("t-open", now), "три подряд - выключен");
        record_at("t-reset", false, 1, now);
        record_at("t-reset", false, 1, now);
        record_at("t-reset", true, 50, now);
        record_at("t-reset", false, 1, now);
        assert!(!open_at("t-reset", now), "успех сбросил счётчик");
    }

    #[test]
    fn breaker_closes_after_ten_minutes_and_reopens_on_failed_trial() {
        let now = 5_000_000;
        for _ in 0..3 {
            record_at("t-close", false, 100, now);
        }
        assert!(open_at("t-close", now + 599_999));
        assert!(!open_at("t-close", now + PAUSE_MS + 1), "через 10 минут - пробная попытка");
        record_at("t-close", false, 100, now + PAUSE_MS + 1);
        assert!(open_at("t-close", now + PAUSE_MS + 2), "пробная не удалась - снова выключен");
        record_at("t-close", true, 80, now + 2 * PAUSE_MS + 5);
        assert!(!open_at("t-close", now + 2 * PAUSE_MS + 6));
    }

    #[test]
    fn snapshot_marks_down_ok_and_unknown() {
        let now = 9_000_000;
        for _ in 0..3 {
            record_at("lordfilm", false, 100, now);
        }
        record_at("rutube", true, 120, now);
        let s = snapshot_at(now);
        let by = |site: &str| s.iter().find(|x| x.site == site).unwrap().state;
        assert_eq!(by("lordfilm"), "down");
        assert_eq!(by("rutube"), "ok");
        assert_eq!(by("peertube"), "unknown");
        assert_eq!(s.iter().find(|x| x.site == "rutube").unwrap().label, "Rutube");
        // Другие тесты не трогают общий список источников, кроме этого.
        record_at("lordfilm", true, 10, now);
        record_at("rutube", true, 10, now);
    }
}
