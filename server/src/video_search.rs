//! Совпадение произведения и серии важнее качества и позиции в выдаче провайдера.
use crate::{
    App,
    api::links::{SearchQuery, parse_rutube, parse_vk},
    util,
};
use parking_lot::Mutex;
use reqwest::Url;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, Weak},
    time::Duration,
};
type SearchResult = Result<Vec<Value>, String>;
type Cache = Mutex<HashMap<String, (i64, SearchResult)>>;
/// Сколько хранить ответ поиска: находки - час, пустой ответ - полчаса (на сайтах появляются новые
/// выпуски), ошибка - несколько секунд (не считается «не нашли»).
fn ttl(result: &SearchResult) -> i64 {
    match result {
        Ok(items) if !items.is_empty() => 3_600_000,
        Ok(_) => 1_800_000,
        Err(_) => 5_000,
    }
}
fn cached(key: &str) -> Option<anyhow::Result<Vec<Value>>> {
    cache()
        .lock()
        .get(key)
        .filter(|(at, result)| util::now_ms() - at < ttl(result))
        .map(|(_, result)| result.clone().map_err(anyhow::Error::msg))
}
fn remember(key: String, result: SearchResult) {
    let mut c = cache().lock();
    let now = util::now_ms();
    c.retain(|_, (at, result)| now - *at < ttl(result));
    if c.len() >= 128
        && !c.contains_key(&key)
        && let Some(oldest) = c.iter().min_by_key(|(_, (at, _))| *at).map(|(key, _)| key.clone())
    {
        c.remove(&oldest);
    }
    c.insert(key, (now, result));
}
fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}
fn search_lock(key: &str) -> Arc<tokio::sync::Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>> = OnceLock::new();
    let mut locks = LOCKS.get_or_init(Mutex::default).lock();
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(key).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(tokio::sync::Mutex::new(()));
    locks.insert(key.to_owned(), Arc::downgrade(&lock));
    lock
}
fn years() -> &'static regex::Regex {
    static YEARS: OnceLock<regex::Regex> = OnceLock::new();
    YEARS.get_or_init(|| regex::Regex::new(r"\b(?:19|20)\d{2}\b").expect("static years"))
}
async fn get(app: &App, url: Url) -> Option<Value> {
    app.http.get(url).timeout(Duration::from_secs(5)).send().await.ok()?.error_for_status().ok()?.json().await.ok()
}
async fn archive(app: &App, title: &str) -> Option<Vec<Value>> {
    let safe = title.replace(['"', '\\'], " ");
    let query = format!("mediatype:movies AND title:\"{safe}\" AND licenseurl:(*publicdomain*)");
    let u = Url::parse_with_params(
        "https://archive.org/advancedsearch.php",
        &[
            ("q", query.as_str()),
            ("output", "json"),
            ("rows", "8"),
            ("fl[]", "identifier"),
            ("fl[]", "title"),
            ("fl[]", "licenseurl"),
        ],
    )
    .ok()?;
    let results = get(app, u).await?;
    let rows = results["response"]["docs"]
        .as_array()?
        .iter()
        .filter(|v| v["licenseurl"].as_str().is_some_and(|s| s.contains("creativecommons.org/publicdomain/")))
        .take(3);
    let items = futures_util::future::join_all(rows.map(|row| async move {
        let id = row["identifier"].as_str().filter(|s| s.bytes().all(|b| b.is_ascii_alphanumeric() || b"_- .".contains(&b)))?;
        let meta = get(app, Url::parse(&format!("https://archive.org/metadata/{id}")).ok()?).await?;
        let file = meta["files"].as_array()?.iter()
            .filter(|f| f["name"].as_str().is_some_and(|s| s.ends_with(".mp4")))
            .max_by_key(|f| f["format"] == "h.264 IA")?;
        let name = file["name"].as_str()?;
        let mut url = Url::parse("https://archive.org/download/").ok()?;
        url.path_segments_mut().ok()?.pop_if_empty().push(id).push(name);
        Some(json!({"site":"archive","url":url.as_str(),"label":row["title"],"thumbUrl":format!("https://archive.org/services/img/{id}"),"duration":file["length"].as_str().and_then(|s|s.parse::<f64>().ok()).map(|s|s.round() as i64),"author":"Internet Archive · Public Domain","license":row["licenseurl"]}))
    })).await;
    let out = items.into_iter().flatten().collect();
    Some(out)
}
/// Служебные слова в подписях трейлеров - не часть названия фильма.
const TRAILER_WORDS: &[&str] = &[
    "трейлер",
    "трейлеры",
    "trailer",
    "тизер",
    "teaser",
    "русский",
    "русском",
    "русская",
    "на",
    "дублированный",
    "дублированная",
    "дубляж",
    "озвучка",
    "озвучкой",
    "в",
    "с",
    "и",
    "официальный",
    "official",
    "финальный",
    "final",
    "новый",
    "второй",
    "hd",
    "full",
    "fullhd",
    "uhd",
    "4k",
    "4к",
    "1080p",
    "720p",
    "2160p",
    "1080",
    "720",
    "фильм",
    "фильма",
    "сериал",
    "сериала",
    "сезон",
    "season",
    "кино",
    "кинотеатрах",
    "премьера",
    "скоро",
    "the",
    "of",
    "and",
    "a",
    "movie",
    "film",
    "series",
    "rus",
    "ru",
    "dub",
    "subs",
];

/// Трейлер именно этого фильма: в подписи кроме названия (или оригинального названия) - только
/// служебные слова и числа. «Начало конца - трейлер» для «Начала» и «Дюна: Часть вторая» для «Дюны»
/// не подходят - это другие фильмы, хоть все слова названия в подписи есть.
pub fn trailer_exact(label: &str, titles: &[&str]) -> bool {
    let words = |s: &str| util::norm(s).split_whitespace().map(String::from).collect::<Vec<_>>();
    let allowed: std::collections::HashSet<String> = titles.iter().flat_map(|t| words(t)).collect();
    words(label)
        .iter()
        .all(|w| allowed.contains(w) || TRAILER_WORDS.contains(&w.as_str()) || w.chars().all(|c| c.is_ascii_digit()))
}

/// В подписи явно низкое качество (360p, 480p) - для трейлера не годится.
pub fn low_quality(label: &str) -> bool {
    let l = label.to_lowercase();
    ["240p", "360p", "480p", " 360", " 480"].iter().any(|t| l.contains(t))
}

pub fn matches(
    label: &str,
    title: &str,
    season: Option<i64>,
    episode: Option<i64>,
    year: Option<i64>,
    trailer: bool,
) -> bool {
    let text = util::norm(label);
    let name = util::norm(title);
    let words = name
        .split_whitespace()
        .filter(|w| !matches!(*w, "в" | "на" | "и" | "с" | "the" | "of" | "and" | "a" | "фильм" | "сериал"))
        .collect::<Vec<_>>();
    if words.is_empty() || !words.iter().all(|w| text.split_whitespace().any(|token| token == *w)) {
        return false;
    }
    let preview =
        text.contains("трейлер") || text.contains("trailer") || text.contains("тизер") || text.contains("teaser");
    if trailer {
        if let Some(year) = year {
            let years = years();
            let found = years.find_iter(label).filter_map(|m| m.as_str().parse::<i64>().ok()).collect::<Vec<_>>();
            if !found.is_empty() && !found.iter().any(|v| (*v - year).abs() <= 1) {
                return false;
            }
        }
        return preview;
    }
    if preview || text.contains("обзор") || text.contains("рецензия") {
        return false;
    }
    if let (Some(s), Some(e)) = (season, episode) {
        let pattern = format!(
            r"(?:^|\D)(?:s0*{s}\s*e0*{e}|0*{s}\s*сезон\s*0*{e}\s*серия|сезон\s*0*{s}\s*серия\s*0*{e}|season\s*0*{s}\s*episode\s*0*{e})(?:\D|$)"
        );
        if !regex::Regex::new(&pattern).expect("numeric episode pattern").is_match(&text) {
            return false;
        }
    } else if let Some(y) = year {
        let years = years();
        let found = years.find_iter(label).filter_map(|m| m.as_str().parse::<i64>().ok()).collect::<Vec<_>>();
        // Год на сайте и в IMDb часто расходится на 1 (фестиваль против проката): у названий из двух и
        // более слов допускаем ± 1, у однословных - только точный год.
        let close = |v: &i64| if words.len() >= 2 { (*v - y).abs() <= 1 } else { *v == y };
        if !found.is_empty() && !found.iter().any(close) {
            return false;
        }
        // Одно общее слово не отличает фильм от ролика про животных, игры и т.п.
        if words.len() == 1 && !found.contains(&y) {
            return false;
        }
    }
    true
}
/// Запрос, который сама строит страница фильма из ленты (`provider=lordfilm`, название, `extId`):
/// ключ кэша совпадает, поэтому прогретый ответ страница получает сразу.
pub fn warm_query(title: &str, ext_id: &str) -> SearchQuery {
    SearchQuery {
        q: title.to_string(),
        provider: Some("lordfilm".into()),
        title_id: None,
        ext_id: Some(ext_id.to_string()),
        season: None,
        episode: None,
        mode: None,
        min: None,
    }
}

/// Что прогревать: фильмы из «Хочу» и «Смотрю» (сначала свежие отметки), у которых нет сохранённой
/// ссылки и нет копии на сервисе: (id, название, как его видит страница).
pub fn warm_candidates(db: &crate::db::Db, limit: i64) -> rusqlite::Result<Vec<(String, String)>> {
    db.with(|c| {
        let mut st = c.prepare(
            "SELECT e.id, COALESCE((SELECT l.title FROM title_localizations l WHERE l.id = e.id), e.title)
             FROM ext_titles e JOIN ext_marks m ON m.ext_id = e.id
             WHERE e.kind = 'movie' AND m.status IN ('want', 'watching')
               AND NOT EXISTS (SELECT 1 FROM links k WHERE k.ext_id = e.id)
               AND NOT EXISTS (SELECT 1 FROM titles t WHERE t.imdb_id = e.id)
             GROUP BY e.id ORDER BY MAX(m.at) DESC LIMIT ?1",
        )?;
        let r = st.query_map([limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
        r.collect()
    })
}

/// Фоновый прогрев кэша поиска: раз в 10 минут (первый раз через 2 минуты после запуска) не больше 20
/// фильмов, по одному, с паузой 3 с, и только когда никто не смотрит (нет свежего прогресса за 60 с).
pub async fn warm(app: Arc<App>) {
    tokio::time::sleep(Duration::from_secs(120)).await;
    loop {
        let watching: bool = app
            .db
            .with(|c| {
                c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM progress2 WHERE updated_at > ?1)",
                    [util::now_ms() - 60_000],
                    |r| r.get(0),
                )
            })
            .unwrap_or(false);
        if !watching {
            for (id, title) in warm_candidates(&app.db, 20).unwrap_or_default() {
                let _ = find(&app, &warm_query(&title, &id)).await;
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
        tokio::time::sleep(Duration::from_secs(600)).await;
    }
}

/// Запрос к источнику с учётом здоровья: выключен (три ошибки подряд) - пропускаем без ожидания,
/// иначе запоминаем, ответил ли он и за сколько.
async fn tracked(site: &str, work: impl std::future::Future<Output = Option<Vec<Value>>>) -> Option<Vec<Value>> {
    if crate::video_health::open(site) {
        return None;
    }
    let started = std::time::Instant::now();
    let result = work.await;
    crate::video_health::record(site, result.is_some(), started.elapsed().as_millis() as u64);
    result
}

/// Подпись подходит, если совпало любое из названий (русское, оригинальное, прокатные).
pub fn any_name_matches(
    label: &str,
    names: &[&str],
    season: Option<i64>,
    episode: Option<i64>,
    year: Option<i64>,
    trailer: bool,
) -> bool {
    names.iter().filter(|n| !n.trim().is_empty()).any(|n| matches(label, n, season, episode, year, trailer))
}

/// Прокатные названия (`aka`) из JSON-списка в базе: до трёх.
fn aka_names(json: Option<String>) -> Vec<String> {
    json.and_then(|j| serde_json::from_str::<Vec<String>>(&j).ok()).unwrap_or_default().into_iter().take(3).collect()
}

pub async fn find(app: &Arc<App>, q: &SearchQuery) -> anyhow::Result<Vec<Value>> {
    let query = q.q.trim();
    let provider = q.provider.as_deref();
    anyhow::ensure!(
        provider.is_none_or(|p| matches!(p, "rutube" | "vk" | "archive" | "peertube" | "commons" | "lordfilm" | "own")),
        "Неизвестный источник"
    );
    let enabled = |name: &str| provider.is_none_or(|p| p == name);
    if query.chars().count() < 2 {
        return Ok(vec![]);
    }
    anyhow::ensure!(query.chars().count() <= 180, "Поисковый запрос слишком длинный");
    let token = app.db.setting_or("vk.token", "");
    let key = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        util::norm(query),
        q.min.unwrap_or(0),
        q.title_id.unwrap_or(0),
        q.ext_id.as_deref().unwrap_or(""),
        q.season.unwrap_or(0),
        q.episode.unwrap_or(0),
        q.mode.as_deref().unwrap_or("")
    );
    let catalogs = serde_json::to_string(&crate::own_sites::load(app)).unwrap_or_default();
    let key = format!(
        "{key}:{}:{}:{}",
        util::sha256_hex(token.as_bytes()),
        provider.unwrap_or("all"),
        &util::sha256_hex(catalogs.as_bytes())[..8]
    );
    if let Some(result) = cached(&key) {
        return result;
    }
    // Waiters recheck the cache after the first search; cancellation releases the lock.
    let lock = search_lock(&key);
    let _search = lock.lock().await;
    if let Some(result) = cached(&key) {
        return result;
    }
    static SEARCH_SLOTS: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    let slot = tokio::time::timeout(
        Duration::from_secs(10),
        SEARCH_SLOTS.get_or_init(|| tokio::sync::Semaphore::new(4)).acquire(),
    )
    .await;
    let _slot = match slot {
        Ok(permit) => permit?,
        Err(_) => {
            let message = "Поиск занят. Повторите через несколько секунд";
            remember(key, Err(message.to_owned()));
            anyhow::bail!(message);
        }
    };
    let expected: Option<(String, Option<String>, Option<i64>)> = if let Some(id) = q.title_id {
        app.db.with(|c| {
            c.query_row("SELECT title,original_title,year FROM titles WHERE id=?", [id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .optional()
        })?
    } else if let Some(id) = &q.ext_id {
        app.db.with(|c|c.query_row("SELECT COALESCE((SELECT title FROM title_localizations WHERE id=e.id),title),original,year FROM ext_titles e WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional())?
    } else {
        None
    };
    // Прокатные названия (aka): на сайтах фильм часто записан не так, как в IMDb.
    let aka: Vec<String> = if let Some(id) = q.title_id {
        aka_names(app.db.with(|c| c.query_row("SELECT aka FROM titles WHERE id=?", [id], |r| r.get(0)).optional())?)
    } else if let Some(id) = &q.ext_id {
        aka_names(
            app.db
                .with(|c| {
                    c.query_row("SELECT json_extract(extra, '$.aka') FROM ext_titles WHERE id=?", [id], |r| r.get(0))
                        .optional()
                })?
                .flatten(),
        )
    } else {
        vec![]
    };
    let ru = async {
        if !enabled("rutube") {
            return None;
        }
        tracked("rutube", async {
            get(app, Url::parse_with_params("https://rutube.ru/api/search/video/", &[("query", query)]).ok()?)
                .await
                .map(|v| parse_rutube(&v))
        })
        .await
    };
    let vk = async {
        if !enabled("vk") || token.is_empty() {
            return None;
        }
        tracked("vk", async {
            get(
                app,
                Url::parse_with_params(
                    "https://api.vk.com/method/video.search",
                    &[("q", query), ("count", "30"), ("adult", "0"), ("access_token", token.as_str()), ("v", "5.199")],
                )
                .ok()?,
            )
            .await
            .map(|v| parse_vk(&v))
        })
        .await
    };
    // Dailymotion проверен отдельно: metadata доступна, но CDN может запретить нативный поток.
    // Не выдаём такой результат за рабочую альтернативу; явная ссылка поддерживается отдельно.
    let archive_title = expected
        .as_ref()
        .and_then(|(_, original, _)| original.as_deref())
        .or_else(|| expected.as_ref().map(|(name, _, _)| name.as_str()))
        .unwrap_or(query);
    let open = async {
        if !enabled("archive") || q.mode.as_deref() == Some("trailer") || q.season.is_some() {
            return None;
        }
        tracked("archive", async {
            tokio::time::timeout(Duration::from_secs(5), archive(app, archive_title)).await.ok().flatten()
        })
        .await
    };
    let peer = async {
        if !enabled("peertube") {
            return None;
        }
        tracked("peertube", async {
            tokio::time::timeout(Duration::from_secs(6), crate::video_sources::peertube(app, query, q.min.unwrap_or(0)))
                .await
                .ok()
                .flatten()
        })
        .await
    };
    let wiki = async {
        if !enabled("commons") || q.season.is_some() || q.mode.as_deref() == Some("trailer") {
            return None;
        }
        tracked("commons", async {
            tokio::time::timeout(Duration::from_secs(5), crate::video_sources::commons(app, archive_title))
                .await
                .ok()
                .flatten()
        })
        .await
    };
    let lord = async {
        if !enabled("lordfilm") {
            return None;
        }
        if q.mode.as_deref() == Some("trailer") || q.season.is_some() != q.episode.is_some() {
            return Some(vec![]);
        }
        let (name, year) = expected.as_ref().map(|(name, _, year)| (name.as_str(), *year)).unwrap_or((query, None));
        tracked("lordfilm", async {
            if let (Some(season), Some(episode)) = (q.season, q.episode) {
                return tokio::time::timeout(
                    Duration::from_secs(15),
                    crate::lordfilm::search_episode(name, year, season, episode),
                )
                .await
                .ok()
                .flatten();
            }
            tokio::time::timeout(Duration::from_secs(10), crate::lordfilm::search(name, year)).await.ok().flatten()
        })
        .await
    };
    // Каталоги хоста («Управление → Настройки → Каталоги»): ищем на каждом их собственным поиском, фильмы и сериалы
    // целиком (по серии такой поиск не работает - у каждого сайта своя вёрстка серий).
    let own = async {
        if !enabled("own") || q.mode.as_deref() == Some("trailer") {
            return None;
        }
        let catalogs: Vec<_> = crate::own_sites::load(app)
            .into_iter()
            .filter_map(|mut c| {
                if !c.enabled {
                    return None;
                }
                if c.search.is_none()
                    && matches!(c.domain.as_str(), "mg.lordfilm.md" | "sr.lordfilm5.pro" | "lordserials.fan")
                {
                    c.search = Some(format!("https://{}/index.php?do=search&subaction=search&story={{q}}", c.domain));
                }
                (c.search.is_some() || c.domain == "old.yummyani.me").then_some(c)
            })
            .collect();
        if catalogs.is_empty() {
            return None;
        }
        let (name, year) = expected.as_ref().map(|(name, _, year)| (name.as_str(), *year)).unwrap_or((query, None));
        let names: Vec<&str> = std::iter::once(name)
            .chain(expected.as_ref().and_then(|(_, original, _)| original.as_deref()))
            .chain(aka.iter().map(String::as_str))
            .collect();
        let each = catalogs.iter().map(|c| {
            let names = &names;
            async move {
                let site = format!("own:{}", c.domain);
                tracked(&site, async {
                    tokio::time::timeout(Duration::from_secs(20), async {
                        if c.domain == "old.yummyani.me" {
                            crate::site_sources::yummy_find(names, year, q.season, q.episode).await
                        } else if let (Some(season), Some(episode)) = (q.season, q.episode) {
                            crate::own_sites::search_episode(app, c, names, year, season, episode).await
                        } else if q.season.is_some() || q.episode.is_some() {
                            Some(vec![])
                        } else {
                            crate::own_sites::search(c, names, year).await
                        }
                    })
                    .await
                    .ok()
                    .flatten()
                })
                .await
            }
        });
        let results: Vec<Option<Vec<Value>>> = futures_util::future::join_all(each).await;
        results.iter().any(Option::is_some).then(|| results.into_iter().flatten().flatten().collect())
    };
    let (ru, vk, open, peer, wiki, lord, own) = tokio::join!(ru, vk, open, peer, wiki, lord, own);
    if ru.is_none()
        && vk.is_none()
        && open.is_none()
        && peer.is_none()
        && wiki.is_none()
        && lord.is_none()
        && own.is_none()
    {
        let message = "Источники видео сейчас не отвечают";
        remember(key, Err(message.to_owned()));
        anyhow::bail!(message);
    }
    let mut out = ru
        .into_iter()
        .chain(vk)
        .chain(open)
        .chain(peer)
        .chain(wiki)
        .chain(lord)
        .chain(own)
        .flatten()
        // Страницы LordFilm и каталогов хоста не публикуют длительность: не выдумываем её и не отбрасываем точные совпадения.
        .filter(|i| {
            i["site"] == "lordfilm"
                || i["site"] == "own"
                || q.min.is_none_or(|m| i["duration"].as_i64().unwrap_or(0) >= m)
        })
        .filter(|i| {
            let label = i["label"].as_str().unwrap_or("");
            if let Some((title, original, year)) = &expected {
                let names: Vec<&str> = std::iter::once(title.as_str())
                    .chain(original.as_deref())
                    .chain(aka.iter().map(String::as_str))
                    .collect();
                any_name_matches(label, &names, q.season, q.episode, *year, q.mode.as_deref() == Some("trailer"))
            } else {
                q.mode.as_deref() != Some("trailer")
            }
        })
        .collect::<Vec<_>>();
    if q.mode.as_deref() == Some("trailer") {
        out.retain(|v| v["duration"].as_i64().is_some_and(|d| (20..=360).contains(&d)));
        if let Some((title, original, _)) = &expected {
            let names: Vec<&str> = std::iter::once(title.as_str()).chain(original.as_deref()).collect();
            out.retain(|v| {
                let label = v["label"].as_str().unwrap_or("");
                trailer_exact(label, &names) && !low_quality(label)
            });
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|i| seen.insert(i["url"].as_str().unwrap_or("").to_string()));
    let catalog_order = crate::own_sites::load(app);
    out.sort_by_key(|i| {
        (
            if i["site"] == "own" {
                catalog_order.iter().position(|c| Some(c.domain.as_str()) == i["author"].as_str()).unwrap_or(usize::MAX)
            } else {
                usize::MAX
            },
            std::cmp::Reverse(crate::api::links::quality_hint(i["label"].as_str().unwrap_or(""))),
            if i["site"] == "archive" {
                0
            } else if i["site"] == "vk" {
                1
            } else {
                2
            },
        )
    });
    out.truncate(36);
    for i in &mut out {
        i["thumb"] = json!(i["thumbUrl"].as_str().map(|u| {
            let mut p = Url::parse("http://localhost/api/links/thumbnail").expect("static URL");
            p.query_pairs_mut().append_pair("url", u);
            format!("/api/links/thumbnail?{}", p.query().unwrap_or_default())
        }));
    }
    remember(key, Ok(out.clone()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn fifty_waiters_share_failure_and_retry_after_cooldown() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..50 {
            let calls = calls.clone();
            tasks.push(tokio::spawn(async move {
                let key = "test-failed-provider";
                let lock = search_lock(key);
                let _guard = lock.lock().await;
                if cached(key).is_none() {
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::task::yield_now().await;
                    remember(key.to_owned(), Err("offline".to_owned()));
                }
                assert!(cached(key).unwrap().is_err());
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        cache().lock().get_mut("test-failed-provider").unwrap().0 -= 5_001;
        assert!(cached("test-failed-provider").is_none());
        remember("test-failed-provider".to_owned(), Ok(vec![json!({"url":"recovered"})]));
        assert_eq!(cached("test-failed-provider").unwrap().unwrap()[0]["url"], "recovered");
    }
    #[tokio::test]
    async fn identical_searches_share_lock_and_cancel_safely() {
        let first = search_lock("same-film");
        let second = search_lock("same-film");
        assert!(Arc::ptr_eq(&first, &second));
        let guard = first.lock().await;
        assert!(second.try_lock().is_err());
        assert!(search_lock("other-film").try_lock().is_ok());
        drop(guard);
        assert!(second.try_lock().is_ok());
        let weak = Arc::downgrade(&first);
        drop(first);
        drop(second);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn warm_query_matches_page_query() {
        let q = warm_query("Первый человек", "tt1213641");
        assert_eq!(q.q, "Первый человек");
        assert_eq!(q.ext_id.as_deref(), Some("tt1213641"));
        assert_eq!(q.provider.as_deref(), Some("lordfilm"));
        assert!(
            q.title_id.is_none() && q.season.is_none() && q.episode.is_none() && q.mode.is_none() && q.min.is_none()
        );
    }

    #[test]
    fn warm_candidates_are_wanted_movies_without_links_or_local_copy() {
        let db = crate::db::Db::memory().unwrap();
        db.with(|c| {
            c.execute_batch(
                "PRAGMA foreign_keys=OFF;
                 INSERT INTO ext_titles (id, kind, title, updated_at) VALUES
                   ('tt9000001','movie','Хочу фильм',0), ('tt9000002','series','Хочу сериал',0),
                   ('tt9000003','movie','Есть ссылка',0), ('tt9000004','movie','Есть у нас',0),
                   ('tt9000005','movie','Смотрю фильм',0), ('tt9000006','movie','Просто в ленте',0);
                 INSERT INTO ext_marks (profile_id, ext_id, status, at) VALUES
                   (1,'tt9000001','want',30), (1,'tt9000002','want',30), (1,'tt9000003','want',30),
                   (1,'tt9000004','want',30), (1,'tt9000005','watching',40);
                 INSERT INTO links (ext_id, kind, url, label, added_at) VALUES ('tt9000003','direct','https://example.com/a.mp4','x',0);
                 INSERT INTO titles (kind, key, title, imdb_id, added_at, updated_at) VALUES ('movie','k','Есть у нас','tt9000004',0,0);",
            )
        })
        .unwrap();
        let got = warm_candidates(&db, 10).unwrap();
        assert_eq!(
            got,
            [
                ("tt9000005".to_string(), "Смотрю фильм".to_string()),
                ("tt9000001".to_string(), "Хочу фильм".to_string())
            ]
        );
    }

    #[test]
    fn cache_ttl_by_result() {
        assert_eq!(ttl(&Ok(vec![json!({"url": "x"})])), 3_600_000, "находки - час");
        assert_eq!(ttl(&Ok(vec![])), 1_800_000, "пусто - полчаса");
        assert_eq!(ttl(&Err("x".into())), 5_000, "ошибка - коротко");
    }

    #[test]
    fn year_off_by_one_matches_for_multi_word_titles() {
        assert!(matches("Первый человек (2019)", "Первый человек", None, None, Some(2018), false));
        assert!(matches("Первый человек (2017)", "Первый человек", None, None, Some(2018), false));
        assert!(!matches("Первый человек (2020)", "Первый человек", None, None, Some(2018), false));
        // Однословное название - по-прежнему только точный год.
        assert!(!matches("Мышь про животных 2025", "Мышь", None, None, Some(2024), false));
        assert!(!matches("Смешные домашние животные 2025", "Животные", None, None, Some(2026), false));
    }

    #[test]
    fn any_name_matches_uses_release_names() {
        let names = ["Человек на Луне", "First Man", "Первый человек"];
        assert!(any_name_matches("Первый человек (2018, 1080p)", &names, None, None, Some(2018), false));
        assert!(any_name_matches("First Man (2018)", &names, None, None, Some(2018), false));
        assert!(!any_name_matches("Новый Человек-паук (2012)", &names, None, None, Some(2018), false));
        assert!(!any_name_matches("Первый человек (2018)", &[], None, None, Some(2018), false));
    }

    #[test]
    fn correct_episode_and_movie() {
        assert!(!matches("Принц Персии — русский трейлер", "", None, None, None, true));
        assert!(!matches("Смешные домашние животные 2025", "Животные", None, None, Some(2026), false));
        assert!(!matches("Невероятно смешные животные", "Животные", None, None, Some(2026), false));
        assert!(matches("Животные | Animals (2026)", "Животные", None, None, Some(2026), false));
        assert!(!matches(
            "Секс в большом городе (фильм, 2008)",
            "Секс в большом городе",
            Some(1),
            Some(1),
            Some(1998),
            false
        ));
        assert!(!matches(
            "Теория Большого взрыва 1 сезон 1 серия",
            "Секс в большом городе",
            Some(1),
            Some(1),
            None,
            false
        ));
        assert!(matches(
            "Секс в большом городе 1 сезон 1 серия",
            "Секс в большом городе",
            Some(1),
            Some(1),
            None,
            false
        ));
        assert!(matches("Sex and the City S01E01", "Sex and the City", Some(1), Some(1), None, false));
        assert!(!matches("Sex and the City S01E10", "Sex and the City", Some(1), Some(1), None, false));
        assert!(!matches(
            "Секс в большом городе 1 сезон 10 серия",
            "Секс в большом городе",
            Some(1),
            Some(1),
            None,
            false
        ));
        assert!(!matches("Матрица 2021 трейлер", "Матрица", None, None, Some(1999), true));
        assert!(!matches("Матрица: Трейлер", "Матрица", None, None, Some(1999), false));
        assert!(!matches("Матрица 2021", "Матрица", None, None, Some(1999), false));
    }
}

#[cfg(test)]
mod trailer_tests {
    use super::*;

    #[test]
    fn only_this_films_trailer() {
        let t = ["Начало", "Inception"];
        assert!(trailer_exact("Начало (2010) - русский дублированный трейлер 1080p", &t));
        assert!(trailer_exact("Inception - Official Trailer HD", &t));
        assert!(!trailer_exact("Начало конца - трейлер 2024", &t));
        let d = ["Дюна", "Dune"];
        assert!(!trailer_exact("Дюна: Часть вторая - русский трейлер", &d));
        assert!(trailer_exact("Дюна — финальный русский трейлер (2021)", &d));
        assert!(!trailer_exact("Дюна обзор фильма трейлер", &d));
    }

    #[test]
    fn low_quality_labels() {
        assert!(low_quality("Трейлер 480p"));
        assert!(!low_quality("Трейлер 1080p"));
        assert!(!low_quality("Трейлер"));
    }
}
