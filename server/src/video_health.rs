//! Ð—Ð´Ð¾Ñ€Ð¾Ð²ÑŒÐµ Ð¸ÑÑ‚Ð¾Ñ‡Ð½Ð¸ÐºÐ¾Ð² Ð²Ð¸Ð´ÐµÐ¾: ÑÐºÐ¾Ð»ÑŒÐºÐ¾ Ñ€Ð°Ð· Ð¿Ð¾Ð´Ñ€ÑÐ´ Ð¸ÑÑ‚Ð¾Ñ‡Ð½Ð¸Ðº Ð½Ðµ Ð¾Ñ‚Ð²ÐµÑ‚Ð¸Ð» Ð¸ ÐºÐ°Ðº Ð±Ñ‹ÑÑ‚Ñ€Ð¾ Ð¾Ñ‚Ð²ÐµÑ‡Ð°Ð». Ð˜ÑÑ‚Ð¾Ñ‡Ð½Ð¸Ðº,
//! ÑƒÐ¿Ð°Ð²ÑˆÐ¸Ð¹ Ñ‚Ñ€Ð¸ Ñ€Ð°Ð·Ð° Ð¿Ð¾Ð´Ñ€ÑÐ´, Ð½Ð° 10 Ð¼Ð¸Ð½ÑƒÑ‚ Ð²Ñ‹Ð¿Ð°Ð´Ð°ÐµÑ‚ Ð¸Ð· Ð¿Ð¾Ð¸ÑÐºÐ° (Ð½Ðµ Ñ‚Ð¾Ñ€Ð¼Ð¾Ð·Ð¸Ñ‚ Ð¾ÑÑ‚Ð°Ð»ÑŒÐ½Ñ‹Ñ…), Ð¿Ð¾Ñ‚Ð¾Ð¼ ÐµÐ¼Ñƒ Ð´Ð°Ñ‘Ñ‚ÑÑ
//! Ð¾Ð´Ð½Ð° Ð¿Ñ€Ð¾Ð±Ð½Ð°Ñ Ð¿Ð¾Ð¿Ñ‹Ñ‚ÐºÐ°. Ð¡Ð¾ÑÑ‚Ð¾ÑÐ½Ð¸Ðµ Ð²Ð¸Ð´Ð½Ð¾ Ð°Ð´Ð¼Ð¸Ð½Ð¸ÑÑ‚Ñ€Ð°Ñ‚Ð¾Ñ€Ñƒ (Â«ÐžÐ±Ð·Ð¾Ñ€Â») Ð¸ Ð·Ñ€Ð¸Ñ‚ÐµÐ»ÑŽ (Ð±Ð»Ð¾Ðº Â«Ð¸ÑÑ‚Ð¾Ñ‡Ð½Ð¸Ðº Ð½Ðµ Ð½Ð°Ð¹Ð´ÐµÐ½Â»).

use crate::util::now_ms;
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const OPEN_AFTER: u32 = 3;
const PAUSE_MS: i64 = 600_000;

/// Ð˜ÑÑ‚Ð¾Ñ‡Ð½Ð¸ÐºÐ¸ Ð¿Ð¾Ð¸ÑÐºÐ°: (ÐºÐ»ÑŽÑ‡, Ð½Ð°Ð·Ð²Ð°Ð½Ð¸Ðµ Ð´Ð»Ñ Ð·Ñ€Ð¸Ñ‚ÐµÐ»Ñ).
pub const SITES: &[(&str, &str)] = &[
    ("lordfilm", "LordFilm"),
    ("rutube", "Rutube"),
    ("vk", "VK Ð’Ð¸Ð´ÐµÐ¾"),
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
    /// `ok`, `down` (Ð²Ñ‹ÐºÐ»ÑŽÑ‡ÐµÐ½ Ð½Ð° 10 Ð¼Ð¸Ð½ÑƒÑ‚) Ð¸Ð»Ð¸ `unknown` (ÐµÑ‰Ñ‘ Ð½Ðµ Ð¾Ñ‚Ð²ÐµÑ‡Ð°Ð»).
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

/// Ð’Ñ‹ÐºÐ»ÑŽÑ‡Ð°Ñ‚ÐµÐ»ÑŒ Ð¾Ñ‚ÐºÑ€Ñ‹Ñ‚: Ð¸ÑÑ‚Ð¾Ñ‡Ð½Ð¸Ðº ÑÐµÐ¹Ñ‡Ð°Ñ Ð¿Ñ€Ð¾Ð¿ÑƒÑÐºÐ°ÐµÐ¼.
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
        assert!(!open_at("t-open", now), "Ð´Ð²Ðµ Ð¾ÑˆÐ¸Ð±ÐºÐ¸ - ÐµÑ‰Ñ‘ Ñ€Ð°Ð±Ð¾Ñ‚Ð°ÐµÑ‚");
        record_at("t-open", false, 100, now);
        assert!(open_at("t-open", now), "Ñ‚Ñ€Ð¸ Ð¿Ð¾Ð´Ñ€ÑÐ´ - Ð²Ñ‹ÐºÐ»ÑŽÑ‡ÐµÐ½");
        record_at("t-reset", false, 1, now);
        record_at("t-reset", false, 1, now);
        record_at("t-reset", true, 50, now);
        record_at("t-reset", false, 1, now);
        assert!(!open_at("t-reset", now), "ÑƒÑÐ¿ÐµÑ… ÑÐ±Ñ€Ð¾ÑÐ¸Ð» ÑÑ‡Ñ‘Ñ‚Ñ‡Ð¸Ðº");
    }

    #[test]
    fn breaker_closes_after_ten_minutes_and_reopens_on_failed_trial() {
        let now = 5_000_000;
        for _ in 0..3 {
            record_at("t-close", false, 100, now);
        }
        assert!(open_at("t-close", now + 599_999));
        assert!(!open_at("t-close", now + PAUSE_MS + 1), "Ñ‡ÐµÑ€ÐµÐ· 10 Ð¼Ð¸Ð½ÑƒÑ‚ - Ð¿Ñ€Ð¾Ð±Ð½Ð°Ñ Ð¿Ð¾Ð¿Ñ‹Ñ‚ÐºÐ°");
        record_at("t-close", false, 100, now + PAUSE_MS + 1);
        assert!(
            open_at("t-close", now + PAUSE_MS + 2),
            "Ð¿Ñ€Ð¾Ð±Ð½Ð°Ñ Ð½Ðµ ÑƒÐ´Ð°Ð»Ð°ÑÑŒ - ÑÐ½Ð¾Ð²Ð° Ð²Ñ‹ÐºÐ»ÑŽÑ‡ÐµÐ½"
        );
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
        // Ð”Ñ€ÑƒÐ³Ð¸Ðµ Ñ‚ÐµÑÑ‚Ñ‹ Ð½Ðµ Ñ‚Ñ€Ð¾Ð³Ð°ÑŽÑ‚ Ð¾Ð±Ñ‰Ð¸Ð¹ ÑÐ¿Ð¸ÑÐ¾Ðº Ð¸ÑÑ‚Ð¾Ñ‡Ð½Ð¸ÐºÐ¾Ð², ÐºÑ€Ð¾Ð¼Ðµ ÑÑ‚Ð¾Ð³Ð¾.
        record_at("lordfilm", true, 10, now);
        record_at("rutube", true, 10, now);
    }
}
