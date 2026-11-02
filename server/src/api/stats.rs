//! Показатели для «Управления → Обзор»: библиотека, зрители, активность по дням и часам, самые смотримые, места на
//! диске по видам, плюс два действия хозяина - очистить кэш видео и сделать копию базы. Всё считается из базы и
//! папок данных, без внешних запросов.

use crate::auth::Manager;
use crate::error::{ApiError, ApiResult};
use crate::util::now_ms;
use crate::App;
use axum::extract::{Query, State};
use axum::Json;
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

type Out = ApiResult<Json<Value>>;
const DAY: i64 = 86_400_000;

#[derive(Deserialize)]
pub struct MetricsQuery {
    /// Часовой пояс зрителя, минуты к востоку от UTC (для графика по часам и дням).
    tz: Option<i64>,
}

/// Размер папки со всем содержимым (без переходов по ссылкам). Нет папки - ноль.
fn dir_size(root: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                stack.push(e.path());
            } else {
                total += meta.len();
            }
        }
    }
    total
}

/// Размеры папок данных: считать их долго (кэш видео бывает на десятки гигабайт), поэтому ответ живёт 30 секунд.
fn disk_parts(data: &Path) -> Value {
    static LAST: Mutex<Option<(Instant, Value)>> = Mutex::new(None);
    if let Some((at, v)) = &*LAST.lock() {
        if at.elapsed() < Duration::from_secs(30) {
            return v.clone();
        }
    }
    let db = ["fsfy.db", "fsfy.db-wal", "fsfy.db-shm"]
        .iter()
        .filter_map(|n| std::fs::metadata(data.join(n)).ok())
        .map(|m| m.len())
        .sum::<u64>();
    let hls = dir_size(&data.join("cache/hls"));
    let copy = dir_size(&data.join("cache/copy"));
    let cache = dir_size(&data.join("cache"));
    let v = json!({
        "db": db,
        "images": dir_size(&data.join("images")),
        "hls": hls,
        "copy": copy,
        "cacheOther": cache.saturating_sub(hls + copy),
        "backups": dir_size(&data.join("backups")),
        "logs": dir_size(&data.join("logs")),
    });
    *LAST.lock() = Some((Instant::now(), v.clone()));
    v
}

pub async fn metrics(State(app): State<Arc<App>>, _m: Manager, Query(q): Query<MetricsQuery>) -> Out {
    let shift = q.tz.unwrap_or(0).clamp(-14 * 60, 14 * 60) * 60_000;
    let now = now_ms();
    let app2 = app.clone();
    let parts = tokio::task::spawn_blocking(move || disk_parts(&app2.data)).await.unwrap_or(Value::Null);
    let value = app.db.with(|c| {
        let one = |sql: &str| -> rusqlite::Result<i64> { c.query_row(sql, [], |r| r.get(0)) };
        let ok_files = "f.missing = 0";
        let titles_of = |kind: &str| {
            format!("SELECT COUNT(*) FROM titles t WHERE t.kind = '{kind}' AND EXISTS (SELECT 1 FROM files f WHERE f.title_id = t.id AND {ok_files})")
        };
        let movies = one(&titles_of("movie"))?;
        let series = one(&titles_of("series"))?;
        let episodes = one(&format!(
            "SELECT COUNT(*) FROM files f JOIN titles t ON t.id = f.title_id WHERE t.kind = 'series' AND {ok_files}"
        ))?;
        let seconds: f64 = c.query_row(
            "SELECT COALESCE(SUM(json_extract(f.probe, '$.duration')), 0) FROM files f WHERE f.missing = 0 AND f.probe IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        let added_week = one(&format!("SELECT COUNT(*) FROM titles WHERE added_at > {}", now - 7 * DAY))?;
        let added_month = one(&format!("SELECT COUNT(*) FROM titles WHERE added_at > {}", now - 30 * DAY))?;
        let hidden = one("SELECT COUNT(*) FROM titles WHERE hidden = 1")?;

        // Разрешения файлов: сколько файлов и сколько места занимает каждое.
        let mut st = c.prepare(
            "SELECT CASE WHEN h >= 2000 THEN '4k' WHEN h >= 1300 THEN '1440' WHEN h >= 1000 THEN '1080' WHEN h >= 700 THEN '720' ELSE 'sd' END AS q,
                    COUNT(*), COALESCE(SUM(size), 0)
             FROM (SELECT size, COALESCE(json_extract(probe, '$.video.height'), 0) AS h FROM files WHERE missing = 0 AND probe IS NOT NULL)
             GROUP BY q",
        )?;
        let resolutions: Vec<Value> = st
            .query_map([], |r| Ok(json!({"q": r.get::<_, String>(0)?, "files": r.get::<_, i64>(1)?, "bytes": r.get::<_, i64>(2)?})))?
            .collect::<Result<_, _>>()?;
        let mut st = c.prepare(
            "SELECT COALESCE(json_extract(probe, '$.video.codec'), '?') AS k, COUNT(*) FROM files WHERE missing = 0 AND probe IS NOT NULL GROUP BY k ORDER BY 2 DESC LIMIT 5",
        )?;
        let codecs: Vec<Value> =
            st.query_map([], |r| Ok(json!({"codec": r.get::<_, String>(0)?, "files": r.get::<_, i64>(1)?})))?.collect::<Result<_, _>>()?;

        let users = one("SELECT COUNT(*) FROM users WHERE disabled = 0")?;
        let new_users = one(&format!("SELECT COUNT(*) FROM users WHERE created_at > {}", now - 7 * DAY))?;
        let logins_day = one(&format!("SELECT COUNT(*) FROM logins WHERE ok = 1 AND at > {}", now - DAY))?;
        let logins_week = one(&format!("SELECT COUNT(*) FROM logins WHERE ok = 1 AND at > {}", now - 7 * DAY))?;
        let failed_week = one(&format!("SELECT COUNT(*) FROM logins WHERE ok = 0 AND at > {}", now - 7 * DAY))?;
        let active_week = one(&format!("SELECT COUNT(DISTINCT profile_id) FROM progress WHERE updated_at > {}", now - 7 * DAY))?;

        // Просмотры по дням за две недели: сколько разных фильмов и серий смотрели и сколько человек.
        let mut st = c.prepare(
            "SELECT (updated_at + ?1) / ?2 AS d, COUNT(*), COUNT(DISTINCT profile_id) FROM progress WHERE updated_at > ?3 GROUP BY d",
        )?;
        let rows: Vec<(i64, i64, i64)> = st
            .query_map(rusqlite::params![shift, DAY, now - 14 * DAY], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        let today = (now + shift) / DAY;
        let days: Vec<Value> = (0..14)
            .map(|i| {
                let d = today - 13 + i;
                let hit = rows.iter().find(|r| r.0 == d);
                json!({"day": d * DAY - shift, "views": hit.map_or(0, |r| r.1), "people": hit.map_or(0, |r| r.2)})
            })
            .collect();

        // Когда смотрят: просмотры по часам суток за месяц.
        let mut st = c.prepare(
            "SELECT ((updated_at + ?1) / 3600000) % 24 AS h, COUNT(*) FROM progress WHERE updated_at > ?2 GROUP BY h",
        )?;
        let by_hour: Vec<(i64, i64)> = st
            .query_map(rusqlite::params![shift, now - 30 * DAY], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let hours: Vec<i64> = (0..24).map(|h| by_hour.iter().find(|r| r.0 == h).map_or(0, |r| r.1)).collect();

        // Самое смотримое: по числу разных зрителей.
        let mut st = c.prepare(
            "SELECT t.id, t.title, t.year, t.kind, COUNT(DISTINCT p.profile_id) AS v, SUM(p.done) AS fin
             FROM progress p JOIN files f ON f.id = p.file_id JOIN titles t ON t.id = f.title_id
             GROUP BY t.id ORDER BY v DESC, MAX(p.updated_at) DESC LIMIT 5",
        )?;
        let top: Vec<Value> = st
            .query_map([], |r| {
                Ok(json!({"id": r.get::<_, i64>(0)?, "title": r.get::<_, String>(1)?, "year": r.get::<_, Option<i64>>(2)?,
                    "kind": r.get::<_, String>(3)?, "viewers": r.get::<_, i64>(4)?, "finished": r.get::<_, i64>(5)?}))
            })?
            .collect::<Result<_, _>>()?;

        let reviews = one("SELECT COUNT(*) FROM reviews")?;
        let messages = one("SELECT COUNT(*) FROM messages")?;
        let lists = one("SELECT COUNT(*) FROM lists")?;
        let friends = one("SELECT COUNT(*) FROM friends WHERE status = 'accepted'")?;
        let wishes = one("SELECT COUNT(*) FROM wishes WHERE dismissed_at IS NULL")?;

        Ok::<Value, rusqlite::Error>(json!({
            "library": {"movies": movies, "series": series, "episodes": episodes, "seconds": seconds,
                "addedWeek": added_week, "addedMonth": added_month, "hidden": hidden,
                "resolutions": resolutions, "codecs": codecs},
            "people": {"users": users, "newWeek": new_users, "loginsDay": logins_day, "loginsWeek": logins_week,
                "failedWeek": failed_week, "activeWeek": active_week},
            "activity": {"days": days, "hours": hours, "top": top},
            "social": {"reviews": reviews, "messages": messages, "lists": lists, "friends": friends, "wishes": wishes},
        }))
    })?;
    let backups = std::fs::read_dir(app.data.join("backups"))
        .map(|d| d.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "db")).count())
        .unwrap_or(0);
    let mut out = value;
    out["disk"] = parts;
    out["backups"] = json!(backups);
    out["at"] = json!(now);
    Ok(Json(out))
}

/// Очистить кэш видео (перекодированное и «Оригинал»): освободит место, нужное зрителям потом снова пересоберётся.
/// Пока кто-то смотрит, не трогаем: у него эти файлы открыты.
pub async fn clear_cache(State(app): State<Arc<App>>, _m: Manager) -> Out {
    let busy: i64 = app.db.with(|c| {
        c.query_row("SELECT COUNT(DISTINCT profile_id) FROM progress WHERE updated_at > ?", [now_ms() - 90_000], |r| {
            r.get(0)
        })
    })?;
    if busy > 0 {
        return Err(ApiError::bad("busy", "Сейчас кто-то смотрит - кэш можно очистить, когда просмотр закончится."));
    }
    let data = app.data.clone();
    let freed = tokio::task::spawn_blocking(move || {
        let mut freed = 0;
        for part in ["cache/hls", "cache/copy"] {
            let dir = data.join(part);
            freed += dir_size(&dir);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
                }
            }
        }
        freed
    })
    .await
    .map_err(|e| ApiError::bad("cache", e.to_string()))?;
    Ok(Json(json!({"ok": true, "freed": freed})))
}

/// Копия базы сейчас: `backups/fsfy-ручная-<время>.db` (внутри базы - люди, отметки, отзывы, настройки). Хранятся пять
/// последних ручных копий.
pub async fn backup(State(app): State<Arc<App>>, _m: Manager) -> Out {
    let dir = app.data.join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| ApiError::bad("backup", e.to_string()))?;
    let name = format!("fsfy-manual-{}.db", now_ms());
    let path = dir.join(&name);
    let target = path.to_string_lossy().to_string();
    app.db.with(|c| c.execute("VACUUM INTO ?1", [target]).map(|_| ()))?;
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut manual: Vec<_> = std::fs::read_dir(&dir)
        .map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("fsfy-manual-")).collect())
        .unwrap_or_default();
    manual.sort_by_key(|e| e.file_name());
    while manual.len() > 5 {
        let _ = std::fs::remove_file(manual.remove(0).path());
    }
    Ok(Json(
        json!({"ok": true, "name": name, "bytes": bytes, "folder": dir.display().to_string().trim_start_matches(r"\\?\")}),
    ))
}
