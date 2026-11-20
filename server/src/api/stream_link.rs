//! Видео Rutube и VK Видео - в нашем плеере. Сервер находит поток сайта (HLS) и отдаёт его через
//! себя: браузеру сайты свой поток напрямую не дают (Rutube - без CORS, VK - ссылка привязана к
//! адресу компьютера хоста). Плейлисты переписываются на `p?u=…`, кусочки идут потоком.
//! Ходим только на известные адреса сайтов - посредником «куда угодно» сервер не станет.

use crate::App;
use crate::auth::Viewer;
use crate::error::{ApiError, ApiResult};
use crate::util::now_ms;
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use parking_lot::Mutex;
use reqwest::Url;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// Как обычный браузер: иначе сайты отдают страницу-заглушку. Один UA без `Accept` и
/// `Sec-Fetch-*` уже не проходит - одни и те же каталоги отвечают 403 даже на страницу проверки.
const UA: &str = crate::site_sources::USER_AGENT;
pub(crate) const ACCEPT: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8";
pub(crate) const ACCEPT_LANGUAGE: &str = "ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7";
/// Ссылки на поток у сайтов живут несколько часов - помним 20 минут.
const KEEP_MS: i64 = 20 * 60_000;

const RUTUBE: &[&str] = &["rutube.ru", "rtbcdn.ru", "rutubelist.ru"];
const VK: &[&str] = &["okcdn.ru", "vkuser.net", "userapi.com", "mycdn.me", "vkvideo.ru", "vk.com", "vk-cdn.net"];

fn hosts(kind: &str) -> &'static [&'static str] {
    match kind {
        "rutube" => RUTUBE,
        "vk" => VK,
        _ => &[],
    }
}

/// Адрес с сайта этой ссылки (и только https).
pub fn allowed(kind: &str, u: &Url) -> bool {
    let Some(h) = u.host_str().map(str::to_lowercase) else { return false };
    u.scheme() == "https" && hosts(kind).iter().any(|a| h == *a || h.ends_with(&format!(".{a}")))
}

/// Ссылка → (главный плейлист или файл, когда узнали, какие ещё адреса можно (свой сайт)).
type Cache = Mutex<HashMap<String, (String, i64, Vec<String>)>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(Mutex::default)
}

fn client() -> reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        crate::outbound::builder()
            .user_agent(UA)
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("public video client")
    })
    .clone()
}

/// Заголовки обычного перехода по ссылке. Без них антибот и часть каталогов отвечают 403
/// ещё до выдачи страницы проверки.
pub(crate) fn browser_document(req: reqwest::RequestBuilder, same_origin: bool) -> reqwest::RequestBuilder {
    req.header(header::ACCEPT, ACCEPT)
        .header(header::ACCEPT_LANGUAGE, ACCEPT_LANGUAGE)
        .header("Upgrade-Insecure-Requests", "1")
        .header("Sec-Fetch-Dest", "document")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-Site", if same_origin { "same-origin" } else { "none" })
        .header("Sec-Fetch-User", "?1")
}

/// GET с переходами и куками (VK по дороге ставит куки и переадресует к себе же).
pub async fn page(url: &str) -> anyhow::Result<String> {
    Ok(page_with_url(url).await?.1)
}

pub async fn page_with_url(url: &str) -> anyhow::Result<(Url, String)> {
    page_with_cookies(url, HashMap::new(), None).await
}

pub async fn page_for(app: Option<&App>, url: &str) -> anyhow::Result<(Url, String)> {
    let parsed = Url::parse(url)?;
    let mut cookies = HashMap::new();
    if let (Some(app), Some(source)) = (app, parsed.host_str().and_then(crate::site_sources::by_domain))
        && parsed.origin() == Url::parse(source.url)?.origin()
    {
        let values = app.source_sessions.lock().get(source.id).cloned().unwrap_or_default();
        for (k, v) in values {
            if !k.contains([';', '\r', '\n']) && !v.contains([';', '\r', '\n']) {
                cookies.insert(k, v);
            }
        }
    }
    page_with_cookies(url, cookies, None).await
}

async fn page_with_cookies(
    url: &str,
    mut cookies: HashMap<String, String>,
    parent: Option<&Url>,
) -> anyhow::Result<(Url, String)> {
    let mut url = Url::parse(url)?;
    if let Some(host) = url.host_str() {
        for (k, v) in crate::antibot::saved(host) {
            cookies.entry(k).or_insert(v);
        }
    }
    let referer = parent
        .filter(|u| crate::outbound::valid_url(u))
        .and_then(|u| Url::parse(&format!("{}/", u.origin().ascii_serialization())).ok())
        .or_else(|| {
            url::form_urlencoded::parse(url.fragment().unwrap_or("").as_bytes())
                .find(|(k, _)| k == "nova-origin")
                .and_then(|(_, v)| Url::parse(&v).ok())
                .filter(|u| {
                    u.scheme() == "https"
                        && u.path() == "/"
                        && u.query().is_none()
                        && u.host_str().and_then(crate::site_sources::by_domain).is_some()
                })
        });
    let mut solved = false;
    for _ in 0..10 {
        anyhow::ensure!(crate::outbound::valid_url(&url), "внутренний адрес запрещён");
        let jar = cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
        let mut request = browser_document(client().get(url.clone()), referer.is_some())
            .header(header::COOKIE, jar)
            .timeout(Duration::from_secs(12));
        if let Some(ref origin) = referer {
            request = request.header(header::REFERER, origin.as_str());
        }
        let r = request.send().await?;
        for c in r.headers().get_all(header::SET_COOKIE) {
            let c = c.to_str().unwrap_or("");
            let kv = c.split(';').next().unwrap_or("");
            if let Some((k, v)) = kv.split_once('=') {
                cookies.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        if r.status().is_redirection() {
            let loc = r.headers().get(header::LOCATION).and_then(|l| l.to_str().ok()).unwrap_or("");
            let next = url.join(loc)?;
            if next.origin() != url.origin() {
                cookies.clear();
            }
            url = next;
            continue;
        }
        let status = r.status();
        let text = String::from_utf8_lossy(&crate::outbound::limited(r, 4 * 1024 * 1024).await?).into_owned();
        if !solved
            && crate::antibot::is_challenge(&text)
            && let Ok(pairs) = crate::antibot::solve(&client(), &url, &text, &cookies).await
        {
            for (k, v) in pairs {
                cookies.insert(k, v);
            }
            if let Some(host) = url.host_str() {
                // Браузер шлёт и исходную `antibot=…` из Set-Cookie, и cookie из ответа проверки.
                // Только второй значения мало: страница открывается, но AJAX плеера молчит.
                let all: Vec<_> = cookies.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                crate::antibot::remember(host, &all);
            }
            solved = true;
            continue;
        }
        anyhow::ensure!(status.is_success(), "сайт ответил {}", status.as_u16());
        return Ok((url, text));
    }
    anyhow::bail!("слишком много переадресаций")
}

/// POST к AJAX-плееру страницы (Dooplay и похожие темы). Сессия и куки антибота те же,
/// что у самой страницы, иначе сайт отдаёт пустой ответ или снова 403.
pub async fn ajax(app: Option<&App>, endpoint: &Url, referer: &Url, body: &str) -> anyhow::Result<String> {
    anyhow::ensure!(crate::outbound::valid_url(endpoint), "внутренний адрес запрещён");
    let mut cookies = HashMap::new();
    if let (Some(app), Some(source)) = (app, endpoint.host_str().and_then(crate::site_sources::by_domain))
        && endpoint.origin() == Url::parse(source.url)?.origin()
    {
        for (k, v) in app.source_sessions.lock().get(source.id).cloned().unwrap_or_default() {
            if !k.contains([';', '\r', '\n']) && !v.contains([';', '\r', '\n']) {
                cookies.insert(k, v);
            }
        }
    }
    if let Some(host) = endpoint.host_str() {
        for (k, v) in crate::antibot::saved(host) {
            cookies.entry(k).or_insert(v);
        }
    }
    let jar = cookies.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ");
    let response = browser_document(
        client()
            .post(endpoint.clone())
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded;")
            .header(header::ORIGIN, referer.origin().ascii_serialization())
            .header(header::REFERER, referer.as_str())
            .header(header::COOKIE, jar),
        true,
    )
    .body(body.to_string())
    .timeout(Duration::from_secs(12))
    .send()
    .await?;
    anyhow::ensure!(response.status().is_success(), "плеер ответил {}", response.status().as_u16());
    Ok(String::from_utf8_lossy(&crate::outbound::limited(response, 1024 * 1024).await?).into_owned())
}

/// Строка JSON из страницы: `"key":"…"` → значение (с разбором `\/` и `&`).
pub fn json_str(page: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let at = page.find(&pat)? + pat.len();
    let rest = &page[at..];
    let b = rest.as_bytes();
    let mut end = 0;
    while end < b.len() {
        match b[end] {
            b'\\' => end += 2,
            b'"' => break,
            _ => end += 1,
        }
    }
    serde_json::from_str::<String>(&format!("\"{}\"", rest.get(..end)?)).ok().filter(|s| !s.is_empty())
}

/// Адрес видео на странице своего сайта: сначала поток HLS (.m3u8), потом файл (.mp4, .webm).
pub fn find_media(page: &str) -> Option<String> {
    let text = page.replace("\\/", "/").replace("\\u0026", "&").replace("&amp;", "&");
    let re =
        regex::Regex::new(r#"(https?://[^\s"'<>\\]+\.(m3u8|mp4|webm)(?:\?[^\s"'<>\\]*)?)(?:[\s"'<>\\]|$)"#).ok()?;
    let found: Vec<(String, bool)> = re.captures_iter(&text).map(|c| (c[1].to_string(), &c[2] == "m3u8")).collect();
    found.iter().max_by_key(|f| (f.1, super::links::quality_hint(&f.0))).map(|f| f.0.clone())
}

/// Первый встроенный плеер страницы (`<iframe src>`): видео часто лежит в нём.
#[cfg(test)]
pub fn find_iframe(page: &str, base: &Url) -> Option<Url> {
    find_iframes(page, base).into_iter().next()
}

pub fn find_iframes(page: &str, base: &Url) -> Vec<Url> {
    let Ok(re) = regex::Regex::new(r#"(?i)<iframe[^>]+(?:src|data-src)\s*=\s*(?:["']([^"']+)["']|([^\s>"']+))"#) else {
        return vec![];
    };
    re.captures_iter(page)
        .filter_map(|c| base.join(&crate::site_sources::decode(c.get(1).or_else(|| c.get(2))?.as_str().trim())).ok())
        .filter(|u| u.scheme() == "https" || u.scheme() == "http")
        .filter(|u| {
            !u.path().to_ascii_lowercase().split('/').any(|part| matches!(part, "trailer" | "trailers" | "teaser"))
        })
        .collect()
}

/// «cdn.kino.example» → «kino.example»: кусочки у сайтов часто на соседних поддоменах.
fn base_domain(u: &Url) -> Option<String> {
    let h = u.host_str()?.to_lowercase();
    let parts: Vec<&str> = h.split('.').collect();
    (parts.len() >= 2).then(|| parts[parts.len() - 2..].join("."))
}

/// Адрес с сайта этой ссылки или с адресов, которые нашлись у своего сайта.
fn allowed_for(kind: &str, u: &Url, extra: &[String]) -> bool {
    if allowed(kind, u) {
        return true;
    }
    let Some(h) = u.host_str().map(str::to_lowercase) else { return false };
    matches!(kind, "site" | "direct")
        && matches!(u.scheme(), "https" | "http")
        && extra.iter().any(|a| h == *a || h.ends_with(&format!(".{a}")))
}

/// Главный плейлист (или файл) видео у сайта и какие ещё адреса можно (свой сайт).
async fn master(kind: &str, id: &str, url: &str, embed: Option<&str>) -> anyhow::Result<(String, Vec<String>)> {
    let found = match kind {
        "site"
            if crate::site_sources::is_provider(&Url::parse(url)?)
                || (Url::parse(url)?.fragment().is_some_and(|f| f.contains("nova-season="))
                    && !crate::lordfilm::series_url(&Url::parse(url)?)) =>
        {
            let media = crate::site_sources::resolve_player(&Url::parse(url)?)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Плеер не поддержан"))?;
            let mut extra: Vec<String> = base_domain(&Url::parse(&media.url)?).into_iter().collect();
            for (_, sub) in &media.subtitles {
                if let Some(host) = Url::parse(sub).ok().and_then(|u| u.host_str().map(String::from)) {
                    extra.push(host);
                }
            }
            (media.url, extra)
        }
        "rutube" => {
            let v: serde_json::Value = client()
                .get(format!("https://rutube.ru/api/play/options/{id}/?no_404=true&referer=https%3A%2F%2Frutube.ru"))
                .timeout(Duration::from_secs(10))
                .send()
                .await?
                .json()
                .await?;
            let m = v["video_balancer"]["m3u8"]
                .as_str()
                .or(v["video_balancer"]["default"].as_str())
                .map(String::from)
                .ok_or_else(|| anyhow::anyhow!("Rutube не отдал видео (закрыто или удалено)"))?;
            (m, vec![])
        }
        "vk" => {
            let p = page(embed.ok_or_else(|| anyhow::anyhow!("нет ссылки VK"))?).await?;
            let m = json_str(&p, "hls")
                .or_else(|| json_str(&p, "hls_ondemand"))
                .ok_or_else(|| anyhow::anyhow!("VK не отдал видео (закрыто, удалено или только для своих)"))?;
            (m, vec![])
        }
        "site" if Url::parse(url)?.host_str() == Some("www.dailymotion.com") => {
            let v: Value = client()
                .get(format!("https://www.dailymotion.com/player/metadata/video/{id}"))
                .timeout(Duration::from_secs(6))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let stream = v["qualities"]
                .as_object()
                .into_iter()
                .flatten()
                .flat_map(|(_, v)| v.as_array().into_iter().flatten())
                .filter(|v| v["type"].as_str().is_some_and(|t| t.contains("mpegURL") || t.contains("mpegurl")))
                .find_map(|v| v["url"].as_str())
                .ok_or_else(|| anyhow::anyhow!("Dailymotion не предоставляет доступный HLS-поток"))?;
            (stream.to_string(), vec!["dailymotion.com".into(), "dmcdn.net".into()])
        }
        "site" if crate::lordfilm::series_url(&Url::parse(url)?) => {
            let (media, _) = crate::lordfilm::episode_stream(&Url::parse(url)?).await?;
            let extra = base_domain(&Url::parse(&media)?).into_iter().collect();
            (media, extra)
        }
        "site" => {
            let base = Url::parse(url)?;
            let mut extra: Vec<String> = base_domain(&base).into_iter().collect();
            let p = page(url).await?;
            let mut media = find_media(&p);
            let mut queue: std::collections::VecDeque<(Url, Url, usize)> =
                find_iframes(&p, &base).into_iter().map(|u| (u, base.clone(), 1)).collect();
            let has_player = !queue.is_empty();
            let mut visited = std::collections::HashSet::new();
            while media.is_none() && visited.len() < 8 {
                let Some((frame, parent, depth)) = queue.pop_front() else { break };
                if !visited.insert(frame.to_string()) {
                    continue;
                }
                // Не останавливаемся на рекламном/недоступном iframe: пробуем следующий плеер.
                let Ok((actual, html)) = page_with_cookies(frame.as_str(), HashMap::new(), Some(&parent)).await else {
                    continue;
                };
                extra.extend(base_domain(&frame));
                media = find_media(&html);
                if depth < 3 {
                    queue.extend(find_iframes(&html, &actual).into_iter().map(|u| (u, actual.clone(), depth + 1)));
                }
            }
            let media = media.ok_or_else(|| anyhow::anyhow!(if has_player {
                "встроенный плеер сайта не предоставил прямую ссылку на видео. Попробуйте другую ссылку или источник"
            } else {
                "на странице не нашлось видео. Попробуйте другую ссылку или источник"
            }))?;
            extra.extend(Url::parse(&media).ok().as_ref().and_then(base_domain));
            extra.dedup();
            (media, extra)
        }
        _ => anyhow::bail!("этот сайт играет только своим плеером"),
    };
    let u = Url::parse(&found.0)?;
    anyhow::ensure!(allowed_for(kind, &u, &found.1) && crate::outbound::valid_url(&u), "неожиданный адрес видео");
    Ok(found)
}

/// Плейлист: адреса кусочков и вложенных плейлистов - через нас (`p?u=`).
pub fn rewrite(text: &str, base: &Url) -> String {
    let wrap = |s: &str| -> String {
        match base.join(s.trim()) {
            Ok(u) => format!("p?u={}", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(u.as_str())),
            Err(_) => s.to_string(),
        }
    };
    let mut out = String::with_capacity(text.len() + 1024);
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            out.push('\n');
            continue;
        }
        if t.starts_with('#') {
            // URI="…" в EXT-X-KEY, EXT-X-MAP, EXT-X-MEDIA.
            if let Some(i) = t.find("URI=\"") {
                let s = i + 5;
                if let Some(e) = t[s..].find('"') {
                    out.push_str(&t[..s]);
                    out.push_str(&wrap(&t[s..s + e]));
                    out.push_str(&t[s + e..]);
                    out.push('\n');
                    continue;
                }
            }
            out.push_str(t);
        } else {
            out.push_str(&wrap(t));
        }
        out.push('\n');
    }
    out
}

type LinkRow = (String, String, Option<String>);

fn link_of(app: &App, id: i64) -> ApiResult<LinkRow> {
    let row: Option<LinkRow> = app.db.with(|c| {
        c.query_row("SELECT kind, url, embed FROM links WHERE id = ?", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()
    })?;
    row.ok_or_else(ApiError::not_found)
}

fn playlist(body: String) -> Response {
    ([(header::CONTENT_TYPE, "application/vnd.apple.mpegurl"), (header::CACHE_CONTROL, "no-cache")], body)
        .into_response()
}

fn failed(e: impl std::fmt::Display) -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, "site", format!("Видео с сайта не открылось: {e}"))
}

fn b64(u: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(u)
}

/// Видео ссылки у сайта (из памяти на 20 минут): вид, адрес, какие ещё адреса можно.
async fn resolve(app: &App, id: i64) -> ApiResult<(String, String, Vec<String>)> {
    crate::own_sites::link_enabled(app, id)?;
    let (kind, url, embed) = link_of(app, id)?;
    if kind == "direct" {
        let parsed = Url::parse(&url).map_err(failed)?;
        if !crate::outbound::valid_url(&parsed) {
            return Err(ApiError::bad("source", "Внутренний адрес запрещён"));
        }
        let host = parsed.host_str().ok_or_else(ApiError::not_found)?.to_lowercase();
        return Ok((kind, url, vec![host]));
    }
    if hosts(&kind).is_empty() && kind != "site" {
        return Err(ApiError::bad("own_player", "Этот сайт играет только своим плеером"));
    }
    if let Some(c) = cache().lock().get(&url).filter(|c| now_ms() - c.1 < KEEP_MS) {
        return Ok((kind, c.0.clone(), c.2.clone()));
    }
    // Одновременные зрители одного видео ждут одно получение адреса у провайдера.
    static FLIGHTS: OnceLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> = OnceLock::new();
    let gate = {
        let mut gates = FLIGHTS.get_or_init(Mutex::default).lock();
        if gates.len() > 1024 {
            gates.retain(|_, gate| Arc::strong_count(gate) > 1);
        }
        gates.entry(url.clone()).or_default().clone()
    };
    let _guard = gate.lock().await;
    crate::own_sites::link_enabled(app, id)?;
    if let Some(c) = cache().lock().get(&url).filter(|c| now_ms() - c.1 < KEEP_MS) {
        return Ok((kind, c.0.clone(), c.2.clone()));
    }
    let sid = super::links::parse(&url).and_then(|p| p.id).unwrap_or_default();
    let (m, extra) = master(&kind, &sid, &url, embed.as_deref()).await.map_err(failed)?;
    cache().lock().insert(url, (m.clone(), now_ms(), extra.clone()));
    Ok((kind, m, extra))
}

fn invalidate(app: &App, id: i64) {
    if let Ok((_, url, _)) = link_of(app, id) {
        cache().lock().remove(&url);
    }
}

fn is_playlist(u: &Url) -> bool {
    u.path().to_lowercase().ends_with(".m3u8")
}

/// Переходы CDN проверяются на каждом шаге, Range сохраняется.
async fn fetch_media(
    kind: &str,
    mut url: Url,
    extra: &[String],
    range: Option<&axum::http::HeaderValue>,
) -> anyhow::Result<(Url, reqwest::Response)> {
    for _ in 0..5 {
        anyhow::ensure!(
            allowed_for(kind, &url, extra) && crate::outbound::valid_url(&url),
            "неразрешённый адрес потока"
        );
        let mut request = client()
            .get(url.clone())
            .header(
                header::REFERER,
                if kind == "vk" {
                    "https://vkvideo.ru/"
                } else if extra.iter().any(|h| h == "dailymotion.com") {
                    "https://www.dailymotion.com/"
                } else {
                    "https://rutube.ru/"
                },
            )
            .timeout(Duration::from_secs(30));
        if let Some(range) = range {
            request = request.header(header::RANGE, range.clone());
        }
        let response = request.send().await?;
        if response.status().is_redirection() {
            let next = response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| anyhow::anyhow!("нет адреса перехода CDN"))?;
            url = url.join(next)?;
            continue;
        }
        anyhow::ensure!(response.status().is_success(), "CDN ответил {}", response.status());
        return Ok((url, response));
    }
    anyhow::bail!("слишком много переходов CDN")
}

/// Что играть плееру: поток HLS через нас или файл через нас (с перемоткой).
pub async fn source(State(app): State<Arc<App>>, _v: Viewer, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    let (kind, m, _) = resolve(&app, id).await?;
    let u = Url::parse(&m).map_err(failed)?;
    Ok(Json(if u.path().ends_with(".mpd") {
        json!({"url":format!("/api/links/{id}/stream/index.mpd"),"hls":false,"dash":true})
    } else if !matches!(kind.as_str(), "site" | "direct") || is_playlist(&u) {
        json!({"url": format!("/api/links/{id}/stream/index.m3u8"), "hls": true})
    } else {
        json!({"url": format!("/api/links/{id}/stream/p?u={}", b64(&m)), "hls": false})
    }))
}

pub async fn dash_index(State(app): State<Arc<App>>, _v: Viewer, Path(id): Path<i64>) -> ApiResult<Response> {
    let (kind, m, extra) = resolve(&app, id).await?;
    let base = Url::parse(&m).map_err(failed)?;
    let (base, r) = fetch_media(&kind, base, &extra, None).await.map_err(failed)?;
    let bytes =
        crate::outbound::limited(r.error_for_status().map_err(failed)?, 2 * 1024 * 1024).await.map_err(failed)?;
    let text = std::str::from_utf8(&bytes).map_err(failed)?;
    let body = crate::dash_proxy::rewrite(text, &base, id).map_err(failed)?;
    Ok(([(header::CONTENT_TYPE, "application/dash+xml"), (header::CACHE_CONTROL, "no-cache")], body).into_response())
}

pub async fn dash_part(
    State(app): State<Arc<App>>,
    v: Viewer,
    Path((id, origin, path)): Path<(i64, String, String)>,
    uri: axum::http::Uri,
    headers: axum::http::HeaderMap,
) -> ApiResult<Response> {
    let url = crate::dash_proxy::target(&origin, &path, uri.query()).map_err(failed)?;
    part(State(app), v, Path(id), Query(PartQuery { u: b64(url.as_str()) }), headers).await
}

/// Главный плейлист видео по ссылке - для нашего плеера.
#[derive(Deserialize)]
pub struct QualityQuery {
    height: Option<u32>,
    level: Option<usize>,
}

fn select_quality(text: &str, height: Option<u32>, level: Option<usize>) -> String {
    if height.is_none() && level.is_none() {
        return text.to_owned();
    }
    let lines: Vec<_> = text.lines().collect();
    let wanted = height.map(|h| format!("x{h}"));
    let mut found = false;
    let mut output = Vec::new();
    let mut n = 0;
    let mut variant = 0;
    while n < lines.len() {
        if lines[n].starts_with("#EXT-X-STREAM-INF:") {
            let matches = level.map_or_else(
                || {
                    wanted.as_ref().is_some_and(|wanted| {
                        lines[n].split_once("RESOLUTION=").is_some_and(|(_, r)| {
                            r.split(',').next().is_some_and(|resolution| resolution.ends_with(wanted))
                        })
                    })
                },
                |l| l == variant,
            );
            variant += 1;
            if matches {
                found = true;
                output.push(lines[n]);
                if n + 1 < lines.len() {
                    output.push(lines[n + 1]);
                }
            }
            n += 2;
        } else {
            output.push(lines[n]);
            n += 1;
        }
    }
    if found { output.join("\n") } else { text.to_owned() }
}

pub async fn index(
    State(app): State<Arc<App>>,
    _v: Viewer,
    Path(id): Path<i64>,
    Query(q): Query<QualityQuery>,
) -> ApiResult<Response> {
    let (kind, m, extra) = resolve(&app, id).await?;
    let base = Url::parse(&m).map_err(failed)?;
    let (base, r) = fetch_media(&kind, base, &extra, None).await.map_err(|error| {
        invalidate(&app, id);
        failed(error)
    })?;
    if !r.status().is_success() {
        return Err(failed(format!("сайт ответил {}", r.status().as_u16())));
    }
    let text =
        String::from_utf8(crate::outbound::limited(r, 2 * 1024 * 1024).await.map_err(failed)?).map_err(failed)?;
    let text = rewrite(&select_quality(&text, q.height, q.level), &base);
    let text = crate::site_sources::metadata(&m)
        .map(|media| crate::site_sources::hls_manifest(&text, &media, id))
        .unwrap_or(text);
    Ok(playlist(text))
}

pub async fn subtitle_playlist(
    State(app): State<Arc<App>>,
    _v: Viewer,
    Path((id, track)): Path<(i64, String)>,
) -> ApiResult<Response> {
    let track = track.strip_suffix(".m3u8").unwrap_or(&track).parse::<usize>().map_err(|_| ApiError::not_found())?;
    let (kind, url, extra) = resolve(&app, id).await?;
    let media = crate::site_sources::metadata(&url).ok_or_else(ApiError::not_found)?;
    let (_, subtitle) = media.subtitles.get(track).ok_or_else(ApiError::not_found)?;
    let mut duration = media.duration;
    if !duration.is_finite() || duration <= 0.0 {
        let mut playlist_url = Url::parse(&url).map_err(failed)?;
        for _ in 0..3 {
            let (base, r) = fetch_media(&kind, playlist_url, &extra, None).await.map_err(failed)?;
            let data = crate::outbound::limited(r.error_for_status().map_err(failed)?, 2 * 1024 * 1024)
                .await
                .map_err(failed)?;
            let text = std::str::from_utf8(&data).map_err(failed)?;
            duration =
                text.lines().filter_map(|l| l.strip_prefix("#EXTINF:")?.split(',').next()?.parse::<f64>().ok()).sum();
            if duration.is_finite() && duration > 0.0 {
                break;
            }
            let child = text
                .lines()
                .find(|l| !l.is_empty() && !l.starts_with('#'))
                .ok_or_else(|| failed("Не найдена длительность субтитров"))?;
            playlist_url = base.join(child).map_err(failed)?;
        }
        if !duration.is_finite() || duration <= 0.0 {
            return Err(failed("Не удалось определить длительность видео для субтитров"));
        }
    }
    Ok(playlist(format!(
        "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:{}\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXTINF:{duration:.3},\n/api/links/{id}/stream/p?u={}\n#EXT-X-ENDLIST\n",
        duration.ceil() as u64,
        b64(subtitle)
    )))
}

#[derive(Deserialize)]
pub struct PartQuery {
    u: String,
}

/// Вложенный плейлист, кусочек видео или файл целиком - с сайта через нас (Range - как есть).
pub async fn part(
    State(app): State<Arc<App>>,
    _v: Viewer,
    Path(id): Path<i64>,
    Query(q): Query<PartQuery>,
    headers: axum::http::HeaderMap,
) -> ApiResult<Response> {
    let (kind, _, extra) = resolve(&app, id).await?;
    let raw =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(q.u.as_bytes()).map_err(|_| ApiError::not_found())?;
    let u = std::str::from_utf8(&raw).ok().and_then(|s| Url::parse(s).ok()).ok_or_else(ApiError::not_found)?;
    if !allowed_for(&kind, &u, &extra) || !crate::outbound::valid_url(&u) {
        return Err(ApiError::not_found());
    }
    let (u, r) = fetch_media(&kind, u, &extra, headers.get(header::RANGE)).await.map_err(|error| {
        invalidate(&app, id);
        failed(error)
    })?;
    let status = r.status();
    if !status.is_success() {
        return Err(failed(format!("сайт ответил {}", status.as_u16())));
    }
    let ct = r.headers().get(header::CONTENT_TYPE).and_then(|c| c.to_str().ok()).unwrap_or("").to_lowercase();
    if ct.contains("mpegurl") || is_playlist(&u) {
        let text =
            String::from_utf8(crate::outbound::limited(r, 2 * 1024 * 1024).await.map_err(failed)?).map_err(failed)?;
        return Ok(playlist(rewrite(&text, &u)));
    }
    let code = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK);
    let mut resp = Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, if ct.is_empty() { "video/mp2t".to_string() } else { ct })
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, max-age=3600");
    for h in [header::CONTENT_LENGTH, header::CONTENT_RANGE] {
        if let Some(v) = r.headers().get(&h) {
            resp = resp.header(h, v.clone());
        }
    }
    resp.body(Body::from_stream(r.bytes_stream())).map_err(ApiError::internal)
}

/// Диагностика цепочки для «Обзора»: пользователь видит, где именно видео ломается.
/// Адреса CDN наружу не отдаём - только понятные шаги, время и host'ы.
#[derive(Deserialize)]
pub struct DiagnoseBody {
    pub url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagStep {
    key: &'static str,
    title: &'static str,
    note: &'static str,
    status: &'static str,
    ms: u64,
    detail: String,
}

const DIAG_STEPS: &[(&str, &str, &str)] = &[
    ("parse", "Ссылка", "Узнаём сайт и вид видео по адресу"),
    ("public", "Публичный адрес", "Пускаем только внешние сайты, не внутренние"),
    ("master", "Поток сайта", "Спрашиваем у сайта настоящий адрес видео"),
    ("playlist", "Плейлист", "Читаем плейлист или первые байты файла"),
    ("segment", "Кусок видео", "Берём первый кусок, чтобы убедиться в байтах"),
];

fn diag_step(key: &'static str, status: &'static str, ms: u64, detail: String) -> DiagStep {
    let (_, title, note) = DIAG_STEPS.iter().find(|(k, _, _)| *k == key).expect("diagnostic step");
    DiagStep { key, title, note, status, ms, detail }
}

fn diag_skip(steps: &mut Vec<DiagStep>, from: usize) {
    for (key, _, _) in DIAG_STEPS.iter().skip(from) {
        steps.push(diag_step(key, "skip", 0, "Не проверялось: выше была ошибка".into()));
    }
}

fn diag_result(steps: Vec<DiagStep>, ok: bool, verdict: &str, host: Option<&str>) -> Json<Value> {
    Json(json!({ "ok": ok, "verdict": verdict, "host": host, "steps": steps }))
}

fn diag_error(e: impl std::fmt::Display) -> String {
    e.to_string().chars().take(300).collect()
}

fn diag_media_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))
}

/// Что выбрано в адресе: сезон, серия, озвучка (для сериалов из каталогов).
fn diag_choice(url: &Url) -> String {
    let Some(fragment) = url.fragment() else { return String::new() };
    let pairs: HashMap<&str, &str> = fragment.split('&').filter_map(|p| p.split_once('=')).collect();
    let mut out = String::new();
    if let Some(s) = pairs.get("nova-season") {
        out.push_str(&format!(", сезон {}", s));
    }
    if let Some(e) = pairs.get("episode") {
        out.push_str(&format!(", серия {}", e));
    }
    if let Some(v) = pairs.get("nova-voice") {
        out.push_str(&format!(", озвучка {}", v));
    }
    out
}

fn diag_base_url<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let i = text.find(&format!("<{tag}"))?;
    let from = text[i..].find('>')? + i + 1;
    let to = text[from..].find("</")? + from;
    Some(text[from..to].trim())
}

/// Живая проверка цепочки: поиск адреса → сайт → плейлист → первый кусок.
pub async fn diagnose(
    State(app): State<Arc<App>>,
    v: Viewer,
    Json(body): Json<DiagnoseBody>,
) -> ApiResult<Json<Value>> {
    let key = format!("streamdiag:{}", v.profile.id);
    if app.diag_limit.blocked(&key) {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Слишком много проверок. Подождите минуту.",
        ));
    }
    app.diag_limit.fail(&key);

    let mut steps = Vec::with_capacity(DIAG_STEPS.len());

    let t = std::time::Instant::now();
    let Some(parsed) = super::links::parse_with(body.url.trim(), &super::links::extra_sites(&app)) else {
        steps.push(diag_step(
            "parse",
            "fail",
            t.elapsed().as_millis() as u64,
            "Непонятная ссылка. Нужен адрес видео или поддерживаемого сайта".into(),
        ));
        diag_skip(&mut steps, 1);
        return Ok(diag_result(steps, false, "Непонятная ссылка", None));
    };
    let parsed_url = Url::parse(&parsed.url).ok();
    let choice = parsed_url.as_ref().map(diag_choice).unwrap_or_default();
    let site_host = parsed_url.as_ref().and_then(|u| u.host_str()).unwrap_or("неизвестно");
    steps.push(diag_step(
        "parse",
        "ok",
        t.elapsed().as_millis() as u64,
        format!("Вид: {}, сайт: {}{}", parsed.kind, site_host, choice),
    ));

    let t = std::time::Instant::now();
    let Ok(site) = Url::parse(&parsed.url) else {
        steps.push(diag_step("public", "fail", t.elapsed().as_millis() as u64, "Неверный адрес сайта".into()));
        diag_skip(&mut steps, 2);
        return Ok(diag_result(steps, false, "Неверный адрес сайта", None));
    };
    if !crate::outbound::valid_url(&site) {
        steps.push(diag_step("public", "fail", t.elapsed().as_millis() as u64, "Внутренний адрес запрещён".into()));
        diag_skip(&mut steps, 2);
        return Ok(diag_result(steps, false, "Внутренний адрес запрещён", None));
    }
    let host: Option<String> = site.host_str().map(str::to_lowercase);
    steps.push(diag_step(
        "public",
        "ok",
        t.elapsed().as_millis() as u64,
        format!("Публичный адрес: {}", host.as_deref().unwrap_or("сайт")),
    ));

    let t = std::time::Instant::now();
    let (media, extra) =
        match master(parsed.kind, parsed.id.as_deref().unwrap_or_default(), &parsed.url, parsed.embed.as_deref()).await
        {
            Ok(v) => v,
            Err(e) => {
                steps.push(diag_step("master", "fail", t.elapsed().as_millis() as u64, diag_error(e)));
                diag_skip(&mut steps, 3);
                return Ok(diag_result(steps, false, "Сайт не отдал поток", host.as_deref()));
            }
        };
    let Ok(media_url) = Url::parse(&media) else {
        steps.push(diag_step(
            "master",
            "fail",
            t.elapsed().as_millis() as u64,
            "Сайт вернул неверный адрес видео".into(),
        ));
        diag_skip(&mut steps, 3);
        return Ok(diag_result(steps, false, "Сайт вернул неверный адрес видео", host.as_deref()));
    };
    let path = media_url.path().to_lowercase();
    let m3u8 = path.ends_with(".m3u8");
    let mpd = path.ends_with(".mpd");
    let manifest = m3u8 || mpd;
    steps.push(diag_step(
        "master",
        "ok",
        t.elapsed().as_millis() as u64,
        format!(
            "Поток: {}, вид: {}",
            media_url.host_str().unwrap_or("сайт"),
            if m3u8 {
                "HLS"
            } else if mpd {
                "DASH"
            } else {
                "файл"
            }
        ),
    ));

    let t = std::time::Instant::now();
    let range = (!manifest).then(|| axum::http::HeaderValue::from_static("bytes=0-1023"));
    let (base, response) = match fetch_media(parsed.kind, media_url, &extra, range.as_ref()).await {
        Ok(v) => v,
        Err(e) => {
            steps.push(diag_step("playlist", "fail", t.elapsed().as_millis() as u64, diag_error(e)));
            diag_skip(&mut steps, 4);
            return Ok(diag_result(steps, false, "Плейлист не открылся", host.as_deref()));
        }
    };
    let status = response.status().as_u16();
    let data = match crate::outbound::limited(response, 2 * 1024 * 1024).await {
        Ok(v) => v,
        Err(e) => {
            steps.push(diag_step("playlist", "fail", t.elapsed().as_millis() as u64, diag_error(e)));
            diag_skip(&mut steps, 4);
            return Ok(diag_result(steps, false, "Плейлист слишком большой", host.as_deref()));
        }
    };
    if data.is_empty() {
        steps.push(diag_step("playlist", "fail", t.elapsed().as_millis() as u64, "Сайт вернул пустой ответ".into()));
        diag_skip(&mut steps, 4);
        return Ok(diag_result(steps, false, "Плейлист пуст", host.as_deref()));
    }
    let mut text = String::from_utf8_lossy(&data).into_owned();
    if m3u8 && !text.trim_start().starts_with("#EXTM3U") {
        steps.push(diag_step(
            "playlist",
            "fail",
            t.elapsed().as_millis() as u64,
            "Ответ не похож на HLS-плейлист".into(),
        ));
        diag_skip(&mut steps, 4);
        return Ok(diag_result(steps, false, "Плейлист не похож на HLS", host.as_deref()));
    }
    if mpd && !text.contains("<MPD") {
        steps.push(diag_step(
            "playlist",
            "fail",
            t.elapsed().as_millis() as u64,
            "Ответ не похож на DASH-манифест".into(),
        ));
        diag_skip(&mut steps, 4);
        return Ok(diag_result(steps, false, "Манифест не похож на DASH", host.as_deref()));
    }
    let detail = if manifest {
        format!("{}: {} байт, ответ {}", if m3u8 { "HLS" } else { "DASH" }, data.len(), status)
    } else {
        format!("Файл: {} байт по Range, ответ {}", data.len(), status)
    };
    steps.push(diag_step("playlist", "ok", t.elapsed().as_millis() as u64, detail));

    let t = std::time::Instant::now();
    if !manifest {
        steps.push(diag_step(
            "segment",
            "ok",
            t.elapsed().as_millis() as u64,
            "Оригинал: первые байты уже получены Range-запросом".into(),
        ));
        return Ok(diag_result(steps, true, "Видео доступно: файл отдаёт байты", host.as_deref()));
    }

    let mut next = base;
    let mut target = None;
    if m3u8 {
        for _ in 0..3 {
            let Some(line) = diag_media_line(&text) else { break };
            let Ok(u) = next.join(line) else { break };
            if u.path().to_lowercase().ends_with(".m3u8") {
                match fetch_media(parsed.kind, u, &extra, None).await {
                    Ok((b, r)) => match crate::outbound::limited(r, 2 * 1024 * 1024).await {
                        Ok(v) => {
                            text = String::from_utf8_lossy(&v).into_owned();
                            next = b;
                        }
                        Err(e) => {
                            steps.push(diag_step("segment", "fail", t.elapsed().as_millis() as u64, diag_error(e)));
                            return Ok(diag_result(steps, false, "Вложенный плейлист не открылся", host.as_deref()));
                        }
                    },
                    Err(e) => {
                        steps.push(diag_step("segment", "fail", t.elapsed().as_millis() as u64, diag_error(e)));
                        return Ok(diag_result(steps, false, "Вложенный плейлист не открылся", host.as_deref()));
                    }
                }
            } else {
                target = Some(u);
                break;
            }
        }
    } else if let Some(url) = diag_base_url(&text, "BaseURL").and_then(|v| next.join(v).ok()) {
        target = Some(url);
    }
    let Some(target) = target else {
        let why = if m3u8 {
            "В плейлисте нет адреса куска"
        } else {
            "В манифесте DASH нет BaseURL"
        };
        steps.push(diag_step("segment", "fail", t.elapsed().as_millis() as u64, why.into()));
        return Ok(diag_result(steps, false, why, host.as_deref()));
    };
    let seg_range = axum::http::HeaderValue::from_static("bytes=0-1023");
    match fetch_media(parsed.kind, target, &extra, Some(&seg_range)).await {
        Ok((_, r)) => match crate::outbound::limited(r, 4 * 1024 * 1024).await {
            Ok(v) => {
                steps.push(diag_step(
                    "segment",
                    "ok",
                    t.elapsed().as_millis() as u64,
                    format!("Первый кусок: {} байт", v.len()),
                ));
                Ok(diag_result(steps, true, "Видео доступно: плейлист и первый кусок отдают данные", host.as_deref()))
            }
            Err(e) => {
                steps.push(diag_step("segment", "fail", t.elapsed().as_millis() as u64, diag_error(e)));
                Ok(diag_result(steps, false, "Кусок видео не прочитался", host.as_deref()))
            }
        },
        Err(e) => {
            steps.push(diag_step("segment", "fail", t.elapsed().as_millis() as u64, diag_error(e)));
            Ok(diag_result(steps, false, "Кусок видео не открылся", host.as_deref()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_quality_preserves_audio_and_auto() {
        let text = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",URI=\"audio.m3u8\"\n#EXT-X-STREAM-INF:RESOLUTION=1280x720,AUDIO=\"a\"\n720.m3u8\n#EXT-X-STREAM-INF:RESOLUTION=640x360,AUDIO=\"a\"\n360.m3u8\n";
        let selected = select_quality(text, Some(720), None);
        assert!(selected.contains("#EXT-X-MEDIA:"));
        assert!(selected.contains("720.m3u8"));
        assert!(!selected.contains("360.m3u8"));
        assert_eq!(select_quality(text, None, None), text);
        assert_eq!(select_quality(text, Some(1080), None), text);
        let exact = select_quality(text, None, Some(1));
        assert!(exact.contains("360.m3u8"));
        assert!(!exact.contains("720.m3u8"));
        assert!(exact.contains("#EXT-X-MEDIA:TYPE=AUDIO"));
    }

    #[test]
    fn rewrites_playlists() {
        let base = Url::parse("https://river.rtbcdn.ru/hls/a/master.m3u8?x=1").unwrap();
        let src =
            "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXTINF:2,\nseg-1.ts\n\nhttps://x.rtbcdn.ru/b.m3u8\n";
        let out = rewrite(src, &base);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "#EXTM3U");
        assert!(lines[1].starts_with("#EXT-X-KEY:METHOD=AES-128,URI=\"p?u=") && lines[1].ends_with('"'));
        let seg = lines[3].strip_prefix("p?u=").unwrap();
        let back = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(seg).unwrap();
        assert_eq!(std::str::from_utf8(&back).unwrap(), "https://river.rtbcdn.ru/hls/a/seg-1.ts");
        assert!(lines[5].starts_with("p?u="));
    }

    #[test]
    fn only_known_sites() {
        let ok = |k: &str, u: &str| allowed(k, &Url::parse(u).unwrap());
        assert!(ok("rutube", "https://river-m9.rtbcdn.ru/x.ts"));
        assert!(ok("vk", "https://vkvd743.okcdn.ru/video.m3u8"));
        assert!(!ok("rutube", "https://evil.example/x.ts"));
        assert!(!ok("rutube", "https://rtbcdn.ru.evil.example/x.ts"));
        assert!(!ok("vk", "http://vkvd743.okcdn.ru/video.m3u8"));
        assert!(!ok("youtube", "https://www.youtube.com/"));
    }

    #[test]
    fn json_strings_from_page() {
        let page = r#"var p = {"hls":"https:\/\/vkvd1.okcdn.ru\/video.m3u8?a=1&b=2","x":"y"};"#;
        assert_eq!(json_str(page, "hls").as_deref(), Some("https://vkvd1.okcdn.ru/video.m3u8?a=1&b=2"));
        assert_eq!(json_str(page, "none"), None);
    }

    #[test]
    fn media_on_site_pages() {
        assert_eq!(
            find_media(r#"hls: "https://cdn.example/movie.mp4/master.m3u8?token=abc""#).as_deref(),
            Some("https://cdn.example/movie.mp4/master.m3u8?token=abc")
        );
        assert_eq!(find_media(r#"src="https://cdn.example/movie.mp4/thumbnail.jpg""#), None);
        let p = r#"<video src="https://cdn.kino.example/a/film.mp4?t=1"></video><script>var f={"hls":"https:\/\/s2.kino.example\/v\/master.m3u8?k=1&x=2"}</script>"#;
        assert_eq!(find_media(p).as_deref(), Some("https://s2.kino.example/v/master.m3u8?k=1&x=2"));
        assert_eq!(find_media("<p>нет видео</p>"), None);
        let base = Url::parse("https://kino.example/film/1").unwrap();
        let f = find_iframe(r#"<iframe width="1" src="/player/77"></iframe>"#, &base).unwrap();
        assert_eq!(f.as_str(), "https://kino.example/player/77");
        assert_eq!(find_iframe("<iframe src=/player/88 allowfullscreen>", &base).unwrap().path(), "/player/88");
        assert!(find_iframes("<iframe src=/embed/trailer/88>", &base).is_empty());
        let extra = vec!["kino.example".to_string()];
        assert!(allowed_for("site", &Url::parse("https://s2.kino.example/x.ts").unwrap(), &extra));
        assert!(!allowed_for("site", &Url::parse("https://evil.example/x.ts").unwrap(), &extra));
        assert!(!allowed_for("vk", &Url::parse("https://s2.kino.example/x.ts").unwrap(), &extra));
        assert!(allowed_for("direct", &Url::parse("https://s2.kino.example/x.ts").unwrap(), &extra));
        assert!(!allowed_for("direct", &Url::parse("https://evil.example/x.ts").unwrap(), &extra));
    }

    #[tokio::test]
    #[ignore = "requires the live external source and CDN"]
    async fn lordfilm_live_movie_playlist_and_segment() {
        tokio::time::timeout(Duration::from_secs(55), async {
            let found = crate::lordfilm::search_names(&["Дюна"], Some(2021)).await.expect("source search");
            let url = found.first().expect("matching movie")["url"].as_str().unwrap();
            assert_eq!(super::super::links::parse(url).unwrap().kind, "site");
            let (stream, extra) = master("site", "", url, None).await.expect("native stream");
            let (base, response) = fetch_media("site", Url::parse(&stream).unwrap(), &extra, None).await.unwrap();
            assert!(response.status().is_success());
            let text = response.text().await.unwrap();
            assert!(text.starts_with("#EXTM3U"));
            let variant = text.lines().find(|s| !s.trim().is_empty() && !s.starts_with('#')).unwrap();
            let (base, response) = fetch_media("site", base.join(variant).unwrap(), &extra, None).await.unwrap();
            assert!(response.status().is_success());
            let text = response.text().await.unwrap();
            assert!(text.starts_with("#EXTM3U"));
            let segment = text.lines().find(|s| !s.trim().is_empty() && !s.starts_with('#')).unwrap();
            let range = axum::http::HeaderValue::from_static("bytes=0-1023");
            let (_, mut response) =
                fetch_media("site", base.join(segment).unwrap(), &extra, Some(&range)).await.unwrap();
            assert!(response.status().is_success());
            assert!(!response.chunk().await.unwrap().unwrap().is_empty());
        })
        .await
        .expect("live source timeout");
    }
}
