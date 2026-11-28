//! Видео с других сайтов: вставить ссылку (Rutube, VK Видео, YouTube - их встроенным плеером
//! внутри нашего, прямые mp4/webm/m3u8 - нашим плеером) или найти на Rutube (VK - если хост
//! указал ключ VK). Ссылка привязывается к фильму с сервиса, к фильму из ленты или живёт сама.

use crate::App;
use crate::auth::{Manager, Viewer};
use crate::error::{ApiError, ApiResult};
use crate::util::now_ms;
use axum::Json;
use axum::extract::{Path, Query, State};
use reqwest::Url;
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// Разобранная ссылка: вид и адрес встроенного плеера.
#[derive(Debug, PartialEq)]
pub struct Parsed {
    pub kind: &'static str,
    pub url: String,
    pub embed: Option<String>,
    /// Id у сайта (Rutube - для описания).
    pub id: Option<String>,
}

fn hex32(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `-123_456` → ("-123", "456").
fn vk_pair(s: &str) -> Option<(String, String)> {
    let (o, i) = s.split_once('_')?;
    let ok = |x: &str, neg: bool| {
        let d = if neg { x.strip_prefix('-').unwrap_or(x) } else { x };
        !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())
    };
    (ok(o, true) && ok(i, false)).then(|| (o.to_string(), i.to_string()))
}

pub fn parse(raw: &str) -> Option<Parsed> {
    let u = Url::parse(raw.trim()).ok()?;
    if crate::lordfilm::movie_url(&u) || crate::lordfilm::episode_target(&u).is_some() {
        return Some(Parsed { kind: "site", url: u.to_string(), embed: None, id: None });
    }
    if u.scheme() != "https" && u.scheme() != "http" {
        return None;
    }
    let host = u.host_str()?.trim_start_matches("www.").trim_start_matches("m.").to_lowercase();
    let segs: Vec<&str> = u.path_segments().map(|s| s.filter(|p| !p.is_empty()).collect()).unwrap_or_default();
    let q = |k: &str| u.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.into_owned());
    match host.as_str() {
        "dailymotion.com" | "dai.ly" => {
            let id = if host == "dai.ly" { segs.first().copied()? } else { *segs.get(1)? };
            if !id.bytes().all(|b| b.is_ascii_alphanumeric()) || id.len() > 32 {
                return None;
            }
            Some(Parsed {
                kind: "site",
                url: format!("https://www.dailymotion.com/video/{id}"),
                embed: None,
                id: Some(id.into()),
            })
        }
        "rutube.ru" => {
            let id = match segs.as_slice() {
                ["video", id, ..] | ["play", "embed", id, ..] | ["shorts", id, ..] => *id,
                _ => return None,
            };
            hex32(id).then(|| Parsed {
                kind: "rutube",
                url: format!("https://rutube.ru/video/{id}/"),
                embed: Some(format!("https://rutube.ru/play/embed/{id}")),
                id: Some(id.to_string()),
            })
        }
        "vk.com" | "vkvideo.ru" | "vk.ru" => {
            let (oid, id, hash) = if segs.first() == Some(&"video_ext.php") {
                (q("oid")?, q("id")?, q("hash"))
            } else {
                let key = q("z")
                    .and_then(|z| z.strip_prefix("video").map(|v| v.split('/').next().unwrap_or("").to_string()))
                    .or_else(|| segs.last().and_then(|s| s.strip_prefix("video")).map(String::from))?;
                let (o, i) = vk_pair(&key)?;
                (o, i, None)
            };
            vk_pair(&format!("{oid}_{id}"))?;
            let mut embed = format!("https://vkvideo.ru/video_ext.php?oid={oid}&id={id}&hd=2");
            if let Some(h) = hash.filter(|h| h.bytes().all(|b| b.is_ascii_alphanumeric())) {
                embed.push_str(&format!("&hash={h}"));
            }
            Some(Parsed {
                kind: "vk",
                url: format!("https://vkvideo.ru/video{oid}_{id}"),
                embed: Some(embed),
                id: Some(format!("{oid}_{id}")),
            })
        }
        "youtube.com" | "youtu.be" | "youtube-nocookie.com" => {
            let id = if host == "youtu.be" {
                segs.first().map(|s| s.to_string())
            } else if matches!(segs.first(), Some(&"embed") | Some(&"shorts") | Some(&"live")) {
                segs.get(1).map(|s| s.to_string())
            } else {
                q("v")
            }?;
            (id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')).then(|| {
                Parsed {
                    kind: "youtube",
                    url: format!("https://www.youtube.com/watch?v={id}"),
                    embed: Some(format!("https://www.youtube-nocookie.com/embed/{id}?autoplay=1")),
                    id: Some(id),
                }
            })
        }
        _ => {
            let path = u.path().to_lowercase();
            [".mp4", ".m4v", ".webm", ".m3u8", ".mpd", ".mov"].iter().any(|e| path.ends_with(e)).then(|| Parsed {
                kind: "direct",
                url: u.to_string(),
                embed: None,
                id: None,
            })
        }
    }
}

/// Свои сайты хоста (`video.sites`): домены, с чьих страниц видео открывается в нашем плеере.
pub fn extra_sites(app: &App) -> Vec<String> {
    super::json_list(app.db.setting("video.sites"))
}

/// Домен сайта из того, что ввёл хост: «https://www.Kino.example/films» → «kino.example».
pub fn site_domain(raw: &str) -> Option<String> {
    let s = raw.trim().to_lowercase();
    let s = s.split("://").last().unwrap_or("");
    let host = s.split(['/', '?', '#']).next().unwrap_or("").trim_start_matches("www.");
    let ok = host.contains('.')
        && host.len() <= 100
        && host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        && !host.starts_with('.')
        && !host.ends_with('.');
    ok.then(|| host.to_string())
}

/// Как `parse`, а ссылка со своего сайта хоста - вид `site` (видео ищется на странице).
pub fn parse_with(raw: &str, sites: &[String]) -> Option<Parsed> {
    if let Some(p) = parse(raw) {
        return Some(p);
    }
    let u = Url::parse(raw.trim()).ok()?;
    if crate::site_sources::is_provider(&u) && crate::outbound::valid_url(&u) {
        return Some(Parsed { kind: "site", url: u.to_string(), embed: None, id: None });
    }
    if u.scheme() != "https" && u.scheme() != "http" {
        return None;
    }
    let host = u.host_str()?.trim_start_matches("www.").to_lowercase();
    sites.iter().any(|s| host == *s || host.ends_with(&format!(".{s}"))).then(|| Parsed {
        kind: "site",
        url: u.to_string(),
        embed: None,
        id: None,
    })
}

/// Название и картинка страницы своего сайта (`<title>`, `og:image`) - если ответит за 6 с.
async fn page_info(url: &str) -> (Option<String>, Option<String>) {
    let Ok(Ok(page)) = tokio::time::timeout(Duration::from_secs(6), super::stream_link::page(url)).await else {
        return (None, None);
    };
    let meta = |name: &str| {
        let i = page.find(&format!("property=\"{name}\""))?;
        let rest = &page[i..];
        let c = rest.find("content=\"")? + 9;
        let e = rest[c..].find('"')?;
        Some(rest[c..c + e].to_string()).filter(|s| !s.is_empty())
    };
    let title = meta("og:title").or_else(|| {
        let s = page.find("<title")?;
        let s = s + page[s..].find('>')? + 1;
        let e = page[s..].find("</title>")?;
        Some(page[s..s + e].trim().to_string()).filter(|t| !t.is_empty())
    });
    (title.map(|t| t.replace("&quot;", "\"").replace("&amp;", "&").replace("&#39;", "'")), meta("og:image"))
}

const COLS: &str = "l.id, l.kind, l.url, l.embed, l.label, NULLIF(l.thumb, ''), l.duration, l.title_id, l.ext_id, \
    l.added_by, COALESCE(p.name, ''), l.added_at, lp.position, lp.duration, l.season, l.episode, \
    EXISTS (SELECT 1 FROM settings pref WHERE pref.key = ('video.preferred.' || COALESCE('t' || l.title_id, 'e' || l.ext_id) || '.' || COALESCE(l.season, -1) || '.' || COALESCE(l.episode, -1)) AND pref.value = CAST(l.id AS TEXT)), \
    (SELECT domain FROM catalog_links WHERE link_id = l.id)";

/// Подпись ролика без мусора сайта: «Начало | Inception (2010, 4K)» - «Начало». Берём русскую часть до « | » или « / »,
/// убираем скобки с годом или качеством и хвосты «4K», «1080p», «смотреть онлайн» (просьба владельца 25.11).
pub fn tidy_label(label: &str) -> String {
    static JUNK: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(r"(?i)\s*[\(\[][^\)\]]*(?:\b(?:19|20)\d{2}\b|\b\d{3,4}p\b|\b[48]k\b|\bu?hd\b|hdr|rip|web)[^\)\]]*[\)\]]|\s*\b(?:\d{3,4}p|[48]k|u?hd|full\s*hd|hdr)\b|\s*смотреть(?:\s+(?:онлайн|бесплатно|в хорошем качестве))*").unwrap()
    });
    let cyr = |s: &str| s.chars().any(|c| ('а'..='я').contains(&c.to_lowercase().next().unwrap_or(c)) || c == 'ё');
    let parts: Vec<&str> = label.split(" | ").flat_map(|p| p.split(" / ")).collect();
    let part = parts.iter().find(|p| cyr(p)).or(parts.first()).copied().unwrap_or(label);
    let s = JUNK.replace_all(part, "");
    let s = s.trim().trim_end_matches([',', '-', '·', ':', '|']).trim();
    if s.is_empty() { label.trim().to_string() } else { s.to_string() }
}

/// Что показывать в плеере и очереди комнаты: у ссылки фильма - его название (с сезоном и серией), иначе - подпись без мусора.
pub fn display_title(app: &App, id: i64) -> Option<String> {
    app.db
        .with(|c| {
            c.query_row(
                // Русское название фильма ленты - как у карточек: сначала перевод из `title_localizations`.
                "SELECT l.label, COALESCE(t.title, (SELECT lo.title FROM title_localizations lo WHERE lo.id = e.id), e.title),
                        l.season, l.episode FROM links l
                 LEFT JOIN titles t ON t.id = l.title_id LEFT JOIN ext_titles e ON e.id = l.ext_id WHERE l.id = ?",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<i64>>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
            .optional()
        })
        .ok()
        .flatten()
        .map(|(label, film, season, episode)| (label, film.map(|f| crate::names::ru_title(&f)), season, episode))
        .map(|(label, film, season, episode)| match (film.filter(|f| !f.trim().is_empty()), season, episode) {
            (Some(f), Some(s), Some(e)) => format!("{f} - {s} сезон, {e} серия"),
            (Some(f), _, _) => f,
            (None, _, _) => tidy_label(&label),
        })
}

/// Ссылки с автором и местом, где остановился этот зритель (`?1` - его профиль).
const FROM: &str = "links l LEFT JOIN profiles p ON p.id = l.added_by
    LEFT JOIN link_progress lp ON lp.link_id = l.id AND lp.profile_id = ?1";

fn row(r: &rusqlite::Row) -> rusqlite::Result<Value> {
    Ok(json!({
        "id": r.get::<_, i64>(0)?, "kind": r.get::<_, String>(1)?, "url": r.get::<_, String>(2)?,
        "embed": r.get::<_, Option<String>>(3)?, "label": r.get::<_, String>(4)?,
        "thumb": super::img(r.get(5)?), "duration": r.get::<_, Option<i64>>(6)?,
        "titleId": r.get::<_, Option<i64>>(7)?, "extId": r.get::<_, Option<String>>(8)?,
        "addedBy": r.get::<_, Option<i64>>(9)?, "author": r.get::<_, String>(10)?, "addedAt": r.get::<_, i64>(11)?,
        "progress": r.get::<_, Option<f64>>(12)?.map(|p| json!({"position": p, "duration": r.get::<_, Option<f64>>(13).ok().flatten().unwrap_or(0.0)})),
        "season": r.get::<_, Option<i64>>(14)?, "episode": r.get::<_, Option<i64>>(15)?, "preferred": r.get::<_, bool>(16)?,
        "catalogDomain": r.get::<_, Option<String>>(17)?,
        "display": tidy_label(&r.get::<_, String>(4)?),
    }))
}

/// Ссылки фильма с сервиса, фильма из ленты или «просто ссылки» (оба None).
pub fn of(app: &App, profile: i64, title: Option<i64>, ext: Option<&str>) -> ApiResult<Vec<Value>> {
    let sql = format!(
        "SELECT {COLS} FROM {FROM}
         WHERE l.title_id IS ?2 AND l.ext_id IS ?3 ORDER BY lp.updated_at IS NULL, lp.updated_at DESC, l.added_at DESC"
    );
    let mut items = app.db.with(|c| {
        let mut st = c.prepare(&sql)?;
        let r = st.query_map(params![profile, title, ext], row)?;
        r.collect::<rusqlite::Result<Vec<_>>>()
    })?;
    let domains = crate::own_sites::enabled_domains(app);
    items.retain(|v| v["catalogDomain"].as_str().is_none_or(|d| domains.iter().any(|c| c == d)));
    items.sort_by_key(|v| {
        (
            !v["preferred"].as_bool().unwrap_or(false),
            v["catalogDomain"].as_str().and_then(|d| domains.iter().position(|c| c == d)).unwrap_or(usize::MAX),
        )
    });
    Ok(items)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSource {
    ext_id: String,
    url: String,
}

/// Импорт источника к известному фильму из каталога; сериал - только по подтверждённому списку серий.
pub async fn import_source(
    State(app): State<Arc<App>>,
    m: Manager,
    Json(b): Json<ImportSource>,
) -> ApiResult<Json<Value>> {
    let ext = b.ext_id.trim();
    let card = super::discover::ext_card(&app, &m.0, ext)?;
    let raw = Url::parse(b.url.trim()).map_err(|_| ApiError::bad("bad_link", "Введите корректную ссылку"))?;
    if !crate::outbound::valid_url(&raw) {
        return Err(ApiError::bad("bad_link", "Недопустимый адрес"));
    }
    let series = card.kind == "series";
    let mut sites = extra_sites(&app);
    // Хост явно добавляет адрес: сохраняем домен, чтобы источник работал для зрителей. Администратор сайты
    // не выбирает - ему такой адрес откажут.
    if !series && parse_with(raw.as_str(), &sites).is_none() {
        if m.0.role != "host" {
            return Err(unknown_site_or_bad(&b.url));
        }
        let domain = raw.host_str().and_then(site_domain).ok_or_else(|| ApiError::bad("bad_link", BAD_LINK))?;
        if !sites.contains(&domain) {
            if sites.len() >= 30 {
                return Err(ApiError::bad("too_many", "Не больше 30 сайтов"));
            }
            sites.push(domain);
            app.db.set_setting("video.sites", &serde_json::to_string(&sites).unwrap_or_else(|_| "[]".into()))?;
        }
    }
    let entries: Vec<(Option<i64>, Option<i64>, String)> = if series {
        let list = crate::lordfilm::episodes_on_page(&raw).await.map_err(|_| {
            ApiError::bad("unsupported_source", "Не удалось импортировать серии: проверьте ссылку и поддержку сайта")
        })?;
        list.into_iter()
            .map(|(s, e)| {
                let mut u = raw.clone();
                u.set_fragment(Some(&format!("nova-season={s}&episode={e}")));
                (Some(s), Some(e), u.to_string())
            })
            .collect()
    } else {
        let parsed = parse_with(raw.as_str(), &sites).ok_or_else(|| ApiError::bad("bad_link", BAD_LINK))?;
        vec![(None, None, parsed.url)]
    };
    let profile = m.0.profile.id;
    let title_id = card.local;
    let ext_id = if title_id.is_some() { None } else { Some(ext) };
    let scope = title_id.map(|id| format!("t{id}")).unwrap_or_else(|| format!("e{ext}"));
    let count = app.db.tx(|c| {
        let mut count = 0;
        for (season, episode, url) in &entries {
            let Some(parsed) = parse_with(url, &sites) else { continue };
            let old: Option<i64> = c.query_row("SELECT id FROM links WHERE title_id IS ?1 AND ext_id IS ?2 AND url=?3 AND season IS ?4 AND episode IS ?5", params![title_id, ext_id, parsed.url, season, episode], |r| r.get(0)).optional()?;
            let id = if let Some(id)=old { id } else {
                c.execute("INSERT INTO links (title_id,ext_id,kind,url,embed,label,added_by,added_at,season,episode) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![title_id,ext_id,parsed.kind,parsed.url,parsed.embed,card.title,profile,now_ms(),season,episode])?;
                c.last_insert_rowid()
            };
            let key=format!("video.preferred.{scope}.{}.{}",season.unwrap_or(-1),episode.unwrap_or(-1));
            c.execute("INSERT INTO settings(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,id.to_string()])?;
            count += 1;
        }
        Ok(count)
    })?;
    app.kick_material();
    Ok(Json(json!({"count":count})))
}

/// Источник выбирает только администратор; привязка общая для всех профилей.
pub async fn prefer(State(app): State<Arc<App>>, _m: Manager, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    crate::own_sites::link_enabled(&app, id)?;
    type LinkTarget = (Option<i64>, Option<String>, Option<i64>, Option<i64>);
    let target: Option<LinkTarget> = app.db.with(|c| {
        c.query_row("SELECT title_id, ext_id, season, episode FROM links WHERE id = ?", [id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .optional()
    })?;
    let (title, ext, season, episode) = target.ok_or_else(ApiError::not_found)?;
    let scope = title
        .map(|n| format!("t{n}"))
        .or_else(|| ext.map(|e| format!("e{e}")))
        .ok_or_else(|| ApiError::bad("bad_link", "Сначала привяжите видео к фильму или сериалу"))?;
    let key = format!("video.preferred.{scope}.{}.{}", season.unwrap_or(-1), episode.unwrap_or(-1));
    app.db.set_setting(&key, &id.to_string())?;
    Ok(Json(json!({"ok": true})))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    title_id: Option<i64>,
    ext_id: Option<String>,
}

pub async fn list(State(app): State<Arc<App>>, v: Viewer, Query(q): Query<ListQuery>) -> ApiResult<Json<Value>> {
    if let Some(t) = q.title_id {
        super::cards_by_ids(&app, v.profile.kids, &[t])?.pop().ok_or_else(ApiError::not_found)?;
    }
    Ok(Json(json!(of(&app, v.profile.id, q.title_id, q.ext_id.as_deref())?)))
}

pub async fn one(State(app): State<Arc<App>>, v: Viewer, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    crate::own_sites::link_enabled(&app, id)?;
    let sql = format!("SELECT {COLS} FROM {FROM} WHERE l.id = ?2");
    let mut one = app
        .db
        .with(|c| c.query_row(&sql, params![v.profile.id, id], row).optional())?
        .ok_or_else(ApiError::not_found)?;
    if let Some(t) = display_title(&app, id) {
        one["display"] = json!(t);
    }
    Ok(Json(one))
}

#[derive(Deserialize)]
pub struct ProgressBody {
    position: f64,
    duration: f64,
}

/// Где остановился у видео по ссылке. Фильм из ленты сам становится «Смотрю», досмотрен (`play::finished`) -
/// «Просмотрел».
pub async fn progress(
    State(app): State<Arc<App>>,
    v: Viewer,
    Path(id): Path<i64>,
    Json(b): Json<ProgressBody>,
) -> ApiResult<Json<Value>> {
    if !(b.position.is_finite() && b.duration.is_finite() && b.position >= 0.0 && b.duration > 0.0) {
        return Err(ApiError::bad("bad_progress", "Неверное место в видео"));
    }
    let (done, watched) = store_link_progress(&app, v.profile.id, id, b.position, b.duration)?;
    Ok(Json(json!({"ok": true, "done": done, "watched": watched})))
}

/// Записать место профиля в видео по ссылке (и «Смотрю»/«Просмотрел» у фильма из ленты).
/// Вернуть (досмотрено, фильм ленты стал «Просмотрено» только что).
pub fn store_link_progress(app: &App, me: i64, id: i64, position: f64, duration: f64) -> ApiResult<(bool, bool)> {
    type Target = (Option<String>, Option<i64>, Option<i64>, Option<i64>);
    let target: Option<Target> = app.db.with(|c| {
        c.query_row("SELECT ext_id, season, episode, title_id FROM links WHERE id = ?", [id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .optional()
    })?;
    let (ext, season, episode, title) = target.ok_or_else(ApiError::not_found)?;
    let now = now_ms();
    let b = ProgressBody { position, duration };
    let done = super::play::finished(b.position.min(b.duration), b.duration, None);
    let watched = app.db.tx(|c| {
        c.execute(
            "INSERT INTO link_progress (profile_id, link_id, position, duration, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(profile_id, link_id) DO UPDATE SET position = ?3, duration = ?4, updated_at = ?5",
            params![me, id, b.position, b.duration, now],
        )?;
        if let Some(e) = &ext {
            c.execute("DELETE FROM ext_dropped WHERE profile_id = ? AND ext_id = ?", params![me, e])?;
            let series: bool = c.query_row("SELECT kind = 'series' FROM ext_titles WHERE id = ?", [e], |r| r.get(0))?;
            if done && let (Some(s), Some(ep)) = (season, episode) {
                c.execute(
                    "INSERT OR IGNORE INTO ext_seen (profile_id, ext_id, season, episode, at) VALUES (?, ?, ?, ?, ?)",
                    params![me, e, s, ep, now],
                )?;
            }
            let status = if done && !series { "watched" } else { "watching" };
            let before: Option<String> = c
                .query_row("SELECT status FROM ext_marks WHERE profile_id = ? AND ext_id = ?", params![me, e], |r| {
                    r.get(0)
                })
                .optional()?;
            c.execute(
                "INSERT INTO ext_marks (profile_id, ext_id, status, at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(profile_id, ext_id) DO UPDATE SET status = ?3, at = ?4
                 WHERE ext_marks.status != 'watched' OR ?3 = 'watched' OR ?5",
                // Сериал, отмеченный «Просмотрено», снова смотрят (новый сезон, пересмотр) - это «Смотрю».
                params![me, e, status, now, series],
            )?;
            return Ok(status == "watched" && before.as_deref() != Some("watched"));
        }
        // Ссылка у своего фильма или сериала (26.11: «то, что смотрю, не сохраняется» - место было только у ссылки, а «Смотрю»,
        // «Продолжить» и «Просмотрено» про неё не знали): то же, что делает статус в каталоге.
        if let Some(t) = title {
            let series: bool =
                c.query_row("SELECT kind = 'series' FROM titles WHERE id = ?", [t], |r| r.get(0)).optional()?.unwrap_or(false);
            c.execute("DELETE FROM dropped WHERE profile_id = ? AND title_id = ?", params![me, t])?;
            let was: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM watched WHERE profile_id = ? AND title_id = ?)",
                params![me, t],
                |r| r.get(0),
            )?;
            if done && !series {
                c.execute("DELETE FROM watching WHERE profile_id = ? AND title_id = ?", params![me, t])?;
                c.execute("DELETE FROM my_list WHERE profile_id = ? AND title_id = ?", params![me, t])?;
                c.execute(
                    "INSERT INTO watched (profile_id, title_id, at) VALUES (?1, ?2, ?3) ON CONFLICT DO UPDATE SET at = excluded.at",
                    params![me, t, now],
                )?;
                return Ok(!was);
            }
            if !was {
                c.execute("DELETE FROM my_list WHERE profile_id = ? AND title_id = ?", params![me, t])?;
                c.execute(
                    "INSERT INTO watching (profile_id, title_id, at) VALUES (?1, ?2, ?3) ON CONFLICT DO UPDATE SET at = excluded.at",
                    params![me, t, now],
                )?;
            }
        }
        Ok(false)
    })?;
    Ok((done, watched))
}

/// Совместный просмотр видео по ссылке: место комнаты - у всех, кто в ней (длина - из ссылки; не знаем - не пишем).
pub fn save_room_link_place(app: &App, profiles: &[i64], link: i64, position: f64) {
    if !position.is_finite() || position < 5.0 {
        return;
    }
    let duration: Option<i64> = app
        .db
        .with(|c| c.query_row("SELECT duration FROM links WHERE id = ?", [link], |r| r.get(0)).optional())
        .ok()
        .flatten()
        .flatten();
    let Some(duration) = duration.filter(|d| *d > 0) else { return };
    for p in profiles {
        let _ = store_link_progress(app, *p, link, position, duration as f64);
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewLink {
    url: String,
    title_id: Option<i64>,
    ext_id: Option<String>,
    label: Option<String>,
    thumb: Option<String>,
    duration: Option<i64>,
    season: Option<i64>,
    episode: Option<i64>,
    source_domain: Option<String>,
}

/// Ссылка с сайта, которого нет в «Своих сайтах» хоста: отдельная ошибка с доменом (хосту окно предложит разрешить его).
fn unknown_site_or_bad(raw: &str) -> ApiError {
    let site = Url::parse(raw.trim())
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .and_then(|u| u.host_str().and_then(site_domain));
    match site {
        Some(d) => ApiError::bad(
            "unknown_site",
            format!("Сайт {d} хост не разрешил. Разрешает хост: «Управление → Настройки → Свои сайты»"),
        ),
        None => ApiError::bad("bad_link", BAD_LINK),
    }
}

const BAD_LINK: &str = "Такую ссылку не открыть: нужна ссылка на Rutube, VK Видео, YouTube, прямая на .mp4, .webm, .m3u8 \
     или на сайт, который хост добавил в «Управление → Настройки → Свои сайты»";

/// Описание видео на Rutube (название, картинка, длительность) - если сайт ответит за 5 с.
async fn rutube_info(app: &App, id: &str) -> Option<(String, Option<String>, Option<i64>)> {
    let v: Value = app
        .http
        .get(format!("https://rutube.ru/api/video/{id}/"))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    Some((v["title"].as_str()?.to_string(), v["thumbnail_url"].as_str().map(String::from), v["duration"].as_i64()))
}

/// Картинку видео - к себе (браузер ходит только к нам). Только https и только с известных сайтов.
async fn keep_thumb(app: &App, url: Option<&str>) -> Option<String> {
    let u = Url::parse(url?).ok()?;
    if u.scheme() != "https" || !crate::outbound::valid_url(&u) {
        return None;
    }
    tokio::time::timeout(Duration::from_secs(6), app.images.download(u.as_str())).await.ok()?.ok()
}

#[derive(Deserialize)]
pub struct ThumbnailQuery {
    url: String,
}

pub async fn thumbnail(
    State(app): State<Arc<App>>,
    _v: Viewer,
    Query(q): Query<ThumbnailQuery>,
) -> ApiResult<axum::response::Redirect> {
    if q.url.len() > 4096 {
        return Err(ApiError::not_found());
    }
    let name = keep_thumb(&app, Some(&q.url)).await.ok_or_else(ApiError::not_found)?;
    Ok(axum::response::Redirect::temporary(&format!("/img/{name}")))
}

/// Картинки ссылок Rutube, сохранённых без них (до 1.0 картинки с rtbcdn.ru не скачивались), -
/// один раз после запуска.
pub async fn backfill_thumbs(app: Arc<App>) {
    tokio::time::sleep(Duration::from_secs(20)).await;
    let todo: Vec<(i64, String)> = app
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT id, url FROM links WHERE kind = 'rutube' AND (thumb IS NULL OR thumb = '') ORDER BY id DESC LIMIT 200",
            )?;
            let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            r.collect()
        })
        .unwrap_or_default();
    for (id, url) in todo {
        let Some(rid) = parse(&url).and_then(|p| p.id) else { continue };
        let Some((_, th, _)) = rutube_info(&app, &rid).await else { continue };
        if let Some(name) = keep_thumb(&app, th.as_deref()).await {
            let _ = app.db.with(|c| c.execute("UPDATE links SET thumb = ? WHERE id = ?", params![name, id]));
        }
    }
}

pub async fn add(State(app): State<Arc<App>>, v: Viewer, Json(b): Json<NewLink>) -> ApiResult<Json<Value>> {
    if b.season.is_some() != b.episode.is_some()
        || b.season.is_some_and(|s| !(0..=1000).contains(&s))
        || b.episode.is_some_and(|e| !(1..=10000).contains(&e))
    {
        return Err(ApiError::bad("bad_episode", "Укажите сезон и номер серии"));
    }
    let mut sites = extra_sites(&app);
    let source_domain = if let Some(domain) = b.source_domain.as_deref() {
        let normalized = site_domain(domain).ok_or_else(|| ApiError::bad("source", "Неверный адрес источника"))?;
        if !crate::own_sites::enabled_domains(&app).contains(&normalized)
            || (b.title_id.is_none() && b.ext_id.is_none())
        {
            return Err(ApiError::bad("source_disabled", "Источник не подключён или выключен"));
        }
        // Каталог нашёл ссылку на плеер (online-serial.com, horsez.org и т.п.). Такой домен
        // не обязан быть в «Своих сайтах»: источник уже разрешён хостом как каталог.
        if !sites.iter().any(|s| s == &normalized) {
            sites.push(normalized.clone());
        }
        Some(normalized)
    } else {
        None
    };
    // Ссылка пришла из поиска по каталогу: её домен может отличаться от домена каталога
    // (Dooplay ведёт на online-serial.com, horsez.org и другие плееры). Источник уже разрешён.
    let p = parse_with(&b.url, &sites)
        .or_else(|| {
            source_domain.as_ref().and_then(|_| {
                let u = Url::parse(b.url.trim()).ok()?;
                crate::outbound::valid_url(&u).then(|| Parsed {
                    kind: "site",
                    url: u.to_string(),
                    embed: None,
                    id: None,
                })
            })
        })
        .ok_or_else(|| unknown_site_or_bad(&b.url))?;
    if let Some((season, episode)) = Url::parse(&p.url).ok().as_ref().and_then(crate::lordfilm::episode_target)
        && (b.season != Some(season) || b.episode != Some(episode))
    {
        return Err(ApiError::bad("bad_episode", "Серия в ссылке не совпадает с выбранной серией"));
    }
    if b.title_id.is_some() && b.ext_id.is_some() {
        return Err(ApiError::bad("bad_link", "Ссылка - либо к фильму с сервиса, либо к фильму из ленты"));
    }
    if let Some(t) = b.title_id {
        super::cards_by_ids(&app, v.profile.kids, &[t])?.pop().ok_or_else(ApiError::not_found)?;
    }
    if let Some(e) = &b.ext_id {
        super::discover::ext_card(&app, &v, e)?;
    }
    let existing: Option<i64> = app.db.with(|c| c.query_row(
        "SELECT id FROM links WHERE url = ?1 AND title_id IS ?2 AND ext_id IS ?3 AND season IS ?4 AND episode IS ?5 LIMIT 1",
        params![p.url, b.title_id, b.ext_id, b.season, b.episode], |r| r.get(0),
    ).optional())?;
    if let Some(id) = existing {
        if let Some(domain) = source_domain.as_deref() {
            let item = b.ext_id.clone().unwrap_or_else(|| format!("t:{}", b.title_id.unwrap_or_default()));
            let stream = format!(
                "play:{}",
                crate::util::sha256_hex(format!("{}:{:?}:{:?}", p.url, b.season, b.episode).as_bytes())
            );
            app.db.with(|c| {
                c.execute(
                    "INSERT OR IGNORE INTO catalog_links(link_id,domain,item_key,stream_key) VALUES(?1,?2,?3,?4)",
                    params![id, domain, item, stream],
                )
            })?;
        }
        return Ok(Json(json!({"id": id})));
    }
    // Поиск при просмотре создаёт ссылку, привязанную к тайтлу. Импортированные эпизоды тоже
    // принадлежат хосту, но не являются его отдельными ссылками: не ограничиваем ими просмотр.
    if b.title_id.is_none() && b.ext_id.is_none() {
        let n: i64 = app.db.with(|c| {
            c.query_row(
                "SELECT COUNT(*) FROM links l WHERE l.added_by = ? AND l.title_id IS NULL AND l.ext_id IS NULL AND NOT EXISTS (SELECT 1 FROM catalog_links cl WHERE cl.link_id = l.id)",
                [v.profile.id],
                |r| r.get(0),
            )
        })?;
        if n >= 500 {
            return Err(ApiError::bad("too_many", "Не больше 500 отдельных ссылок"));
        }
    }
    let label = b
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Видео")
        .chars()
        .take(200)
        .collect::<String>();
    let thumb: Option<String> = None;
    let id = app.db.tx(|c| {
        c.execute(
            "INSERT INTO links (title_id, ext_id, kind, url, embed, label, thumb, duration, added_by, added_at, season, episode)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![b.title_id, b.ext_id, p.kind, p.url, p.embed, label, thumb, b.duration, v.profile.id, now_ms(), b.season, b.episode],
        )?;
        let id=c.last_insert_rowid();
        if let Some(domain)=source_domain.as_deref() {
            let item=b.ext_id.clone().unwrap_or_else(||format!("t:{}",b.title_id.unwrap_or_default()));
            let stream=format!("play:{}",crate::util::sha256_hex(format!("{}:{:?}:{:?}",p.url,b.season,b.episode).as_bytes()));
            c.execute("INSERT INTO catalog_links(link_id,domain,item_key,stream_key) VALUES(?1,?2,?3,?4)",params![id,domain,item,stream])?;
        }
        Ok(id)
    })?;
    // Новая ссылка = новый материал: поиск по этому фильму должен увидеть её сразу.
    app.kick_material();
    // Starting playback must never wait for a thumbnail or remote page metadata.
    let background = app.clone();
    tokio::spawn(async move {
        let mut image = b.thumb;
        let mut title = None;
        let mut duration = b.duration;
        if p.kind == "rutube" && (image.is_none() || b.label.is_none()) {
            if let Some((name, thumb, seconds)) = rutube_info(&background, p.id.as_deref().unwrap_or_default()).await {
                if b.label.is_none() {
                    title = Some(name);
                }
                image = image.or(thumb);
                duration = duration.or(seconds);
            }
        } else if p.kind == "site" && (image.is_none() || b.label.is_none()) {
            let (name, thumb) = page_info(&p.url).await;
            if b.label.is_none() {
                title = name;
            }
            image = image.or(thumb);
        }
        let thumb = keep_thumb(&background, image.as_deref()).await;
        let _ = background.db.with(|c|c.execute("UPDATE links SET thumb=COALESCE(?1,thumb),label=COALESCE(?2,label),duration=COALESCE(duration,?3) WHERE id=?4",params![thumb,title,duration,id]));
    });
    Ok(Json(json!({"id": id})))
}

pub async fn delete(State(app): State<Arc<App>>, v: Viewer, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    let by: Option<Option<i64>> =
        app.db.with(|c| c.query_row("SELECT added_by FROM links WHERE id = ?", [id], |r| r.get(0)).optional())?;
    let by = by.ok_or_else(ApiError::not_found)?;
    if by != Some(v.profile.id) && !v.manages() {
        return Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "not_yours", "Удалить может тот, кто добавил"));
    }
    app.db.with(|c| c.execute("DELETE FROM links WHERE id = ?", [id]))?;
    Ok(Json(json!({"ok": true})))
}

/// Результаты Rutube (`/api/search/video/`): видео, без каналов и плейлистов.
pub fn parse_rutube(v: &Value) -> Vec<Value> {
    v["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let id = r["id"].as_str().filter(|i| hex32(i))?;
            Some(json!({
                "site": "rutube", "url": format!("https://rutube.ru/video/{id}/"),
                "label": r["title"].as_str().unwrap_or("Видео"), "thumbUrl": r["thumbnail_url"].as_str(),
                "duration": r["duration"].as_i64(), "author": r["author"]["name"].as_str(),
            }))
        })
        .collect()
}

/// Результаты VK API `video.search`.
pub fn parse_vk(v: &Value) -> Vec<Value> {
    v["response"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let (o, i) = (r["owner_id"].as_i64()?, r["id"].as_i64()?);
            let thumb =
                r["image"].as_array().and_then(|a| a.iter().rev().nth(1).or(a.last())).and_then(|x| x["url"].as_str());
            Some(json!({
                "site": "vk", "url": format!("https://vkvideo.ru/video{o}_{i}"),
                "label": r["title"].as_str().unwrap_or("Видео"), "thumbUrl": thumb,
                "duration": r["duration"].as_i64(), "author": Value::Null,
            }))
        })
        .collect()
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub provider: Option<String>,
    #[serde(rename = "titleId")]
    pub title_id: Option<i64>,
    #[serde(rename = "extId")]
    pub ext_id: Option<String>,
    pub season: Option<i64>,
    pub episode: Option<i64>,
    pub mode: Option<String>,
    /// Только видео не короче стольких секунд (целый фильм, а не трейлер) - до скачивания картинок.
    pub min: Option<i64>,
}

/// Подтверждённая метаданными или заявленная в названии высота; неизвестную не выдумываем.
pub fn quality_hint(label: &str) -> u32 {
    let label = label.to_lowercase();
    for (height, tags) in [
        (2160, &["2160", "4k", "4к", "uhd"][..]),
        (1440, &["1440", "2k"][..]),
        (1080, &["1080", "full hd", "fullhd"][..]),
        (720, &["720"][..]),
        (480, &["480"][..]),
    ] {
        if tags.iter().any(|tag| label.contains(tag)) {
            return height;
        }
    }
    0
}

/// Поиск видео: Rutube всегда, VK Видео - если хост указал ключ VK. Картинки - к себе.
pub async fn search(State(app): State<Arc<App>>, v: Viewer, Query(q): Query<SearchQuery>) -> ApiResult<Json<Value>> {
    if let Some(id) = q.title_id {
        super::cards_by_ids(&app, v.profile.kids, &[id])?.pop().ok_or_else(ApiError::not_found)?;
    }
    if let Some(id) = &q.ext_id {
        super::discover::ext_card(&app, &v, id)?;
    }
    let mut items = crate::video_search::find(&app, &q).await.map_err(|e| ApiError::bad("search", e.to_string()))?;
    // Отмеченные «не тот фильм» - не предлагаем.
    let bad: std::collections::HashSet<String> = app.db.with(|c| {
        let mut st = c.prepare("SELECT url FROM bad_videos")?;
        let r = st.query_map([], |r| r.get(0))?;
        r.collect::<rusqlite::Result<_>>()
    })?;
    items.retain(|i| i["url"].as_str().is_none_or(|u| !bad.contains(u)));
    Ok(Json(json!(items)))
}

/// Состояние источников видео для зрителя: кто сейчас отвечает (без счётчиков ошибок и времени).
pub async fn sources_status(_v: Viewer) -> ApiResult<Json<Value>> {
    let list: Vec<Value> = crate::video_health::snapshot()
        .into_iter()
        .map(|s| json!({"site": s.site, "label": s.label, "state": s.state}))
        .collect();
    Ok(Json(json!(list)))
}

/// «Это не тот фильм»: видео по ссылке оказалось другим (ролик с похожим названием, трейлер).
/// Адрес - в чёрный список поиска для всех, ссылка у фильма удаляется (решать может любой зритель:
/// неверное видео неверно для всех).
pub async fn wrong(State(app): State<Arc<App>>, v: Viewer, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    type Row = (String, Option<i64>, Option<String>);
    let row: Option<Row> = app.db.with(|c| {
        c.query_row("SELECT url, title_id, ext_id FROM links WHERE id = ?", [id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .optional()
    })?;
    let (url, title, ext) = row.ok_or_else(ApiError::not_found)?;
    app.db.tx(|tx| {
        tx.execute(
            "INSERT OR REPLACE INTO bad_videos (url, title_id, ext_id, by_profile, at) VALUES (?, ?, ?, ?, ?)",
            params![url, title, ext, v.profile.id, now_ms()],
        )?;
        tx.execute("DELETE FROM links WHERE url = ? AND (title_id IS ?2 AND ext_id IS ?3)", params![url, title, ext])?;
        Ok(())
    })?;
    Ok(Json(json!({"ok": true})))
}

/// Где зритель остановился в этой работе (та же серия) по любому её видео: на другом устройстве
/// могли открыть другой источник того же фильма.
pub async fn resume(State(app): State<Arc<App>>, v: Viewer, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    let row: Option<(i64, f64, f64, i64)> = app.db.with(|c| {
        c.query_row(
            "SELECT o.id, p.position, p.duration, p.updated_at
               FROM links l
               JOIN links o ON (o.id = l.id
                   OR (l.ext_id IS NOT NULL AND o.ext_id = l.ext_id)
                   OR (l.title_id IS NOT NULL AND o.title_id = l.title_id))
                 AND o.season IS l.season AND o.episode IS l.episode
               JOIN link_progress p ON p.link_id = o.id AND p.profile_id = ?1
              WHERE l.id = ?2
              ORDER BY p.updated_at DESC LIMIT 1",
            params![v.profile.id, id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
    })?;
    Ok(Json(match row {
        Some((link, position, duration, at)) => {
            json!({"linkId": link, "position": position, "duration": duration, "updatedAt": at})
        }
        None => json!({"linkId": null, "position": null, "duration": null, "updatedAt": null}),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidy_labels() {
        assert_eq!(tidy_label("Начало | Inception (2010, 4K)"), "Начало");
        assert_eq!(tidy_label("Interstellar / Интерстеллар [1080p]"), "Интерстеллар");
        assert_eq!(tidy_label("Дюна 4K смотреть онлайн"), "Дюна");
        assert_eq!(tidy_label("Матрица (1999)"), "Матрица");
        assert_eq!(tidy_label("Мой фильм (режиссёрская версия)"), "Мой фильм (режиссёрская версия)");
        assert_eq!(tidy_label("Big Buck Bunny"), "Big Buck Bunny");
    }

    #[test]
    fn parses_links() {
        let r = parse("https://rutube.ru/video/0123456789abcdef0123456789abcdef/?r=wd").unwrap();
        assert_eq!(r.kind, "rutube");
        assert_eq!(r.embed.as_deref(), Some("https://rutube.ru/play/embed/0123456789abcdef0123456789abcdef"));
        let v = parse("https://vk.com/video-123_456789").unwrap();
        assert_eq!(v.embed.as_deref(), Some("https://vkvideo.ru/video_ext.php?oid=-123&id=456789&hd=2"));
        let v = parse("https://vkvideo.ru/video?z=video-1_2%2Fpl_cat").unwrap();
        assert_eq!(v.id.as_deref(), Some("-1_2"));
        let v = parse("https://vk.com/video_ext.php?oid=5&id=7&hash=abc123").unwrap();
        assert!(v.embed.unwrap().ends_with("&hash=abc123"));
        let y = parse("https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(y.embed.as_deref(), Some("https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ?autoplay=1"));
        assert_eq!(parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1").unwrap().kind, "youtube");
        assert_eq!(parse("https://cdn.example.com/a/movie.MP4").unwrap().kind, "direct");
        assert_eq!(parse("https://cdn.example.com/a/manifest.mpd").unwrap().kind, "direct");
        assert!(parse("https://example.com/page").is_none());
        assert!(parse("javascript:alert(1)").is_none());
        assert!(parse("https://rutube.ru/video/short/").is_none());
        assert!(parse("https://vk.com/video-1_x").is_none());
    }

    #[test]
    fn parses_search() {
        let r = parse_rutube(&json!({"results": [
            {"id": "0123456789abcdef0123456789abcdef", "title": "Фильм", "thumbnail_url": "https://pic.rutube.ru/a.jpg",
             "duration": 6000, "author": {"name": "Канал"}},
            {"id": "bad", "title": "x"}
        ]}));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["url"], "https://rutube.ru/video/0123456789abcdef0123456789abcdef/");
        let k = parse_vk(&json!({"response": {"items": [{"owner_id": -1, "id": 2, "title": "В", "duration": 5,
            "image": [{"url": "https://sun.userapi.com/s.jpg"}, {"url": "https://sun.userapi.com/m.jpg"}]}]}}));
        assert_eq!(k[0]["url"], "https://vkvideo.ru/video-1_2");
        assert_eq!(k[0]["thumbUrl"], "https://sun.userapi.com/s.jpg");
    }
}
