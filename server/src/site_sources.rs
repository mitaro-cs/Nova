//! Именованные источники и публичные структурированные плейлисты. Сетевые отказы не маскируются.
use crate::{App, api::stream_link, outbound, util};
use parking_lot::Mutex;

pub const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0 Safari/537.36";
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
    time::Duration,
};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: &'static str,
    pub name: &'static str,
    pub url: &'static str,
    pub kind: &'static str,
    pub title: Option<&'static str>,
    pub imdb: Option<&'static str>,
}

pub const SOURCES: &[Source] = &[
    Source {
        id: "lordfilm",
        name: "LordFilm - фильмы",
        url: "https://sr.lordfilm5.pro/filmy/",
        kind: "movie",
        title: None,
        imdb: None,
    },
    Source {
        id: "lordserials",
        name: "Lordserials - сериалы",
        url: "https://lordserials.fan/",
        kind: "series",
        title: None,
        imdb: None,
    },
    Source {
        id: "sopranos",
        name: "Сопрано",
        url: "https://www.watch-the-sopranos-online.net/",
        kind: "series",
        title: Some("Клан Сопрано"),
        imdb: Some("tt0141842"),
    },
    Source {
        id: "friends",
        name: "Друзья",
        url: "https://serialfriends.org/",
        kind: "series",
        title: Some("Друзья"),
        imdb: Some("tt0108778"),
    },
    Source {
        id: "supernatural",
        name: "Сверхъестественное",
        url: "https://supernatural-spn.net/",
        kind: "series",
        title: Some("Сверхъестественное"),
        imdb: Some("tt0460681"),
    },
    Source {
        id: "bear",
        name: "Медведь",
        url: "https://the-bear-lordfilms.ru/",
        kind: "series",
        title: Some("Медведь"),
        imdb: Some("tt14452776"),
    },
    Source {
        id: "southpark",
        name: "Южный парк",
        url: "https://south-park.mult-fan.tv/page.php?id=101",
        kind: "series",
        title: Some("Южный парк"),
        imdb: Some("tt0121955"),
    },
    Source {
        id: "yummy",
        name: "YummyAnime - аниме",
        url: "https://old.yummyani.me/catalog/item/dororo-2026-11-06",
        kind: "anime",
        title: None,
        imdb: None,
    },
    Source {
        id: "potter",
        name: "Гарри Поттер",
        url: "https://hd1.garri-potter-lordfilms.com/",
        kind: "movie",
        title: None,
        imdb: None,
    },
];

pub fn by_domain(domain: &str) -> Option<&'static Source> {
    SOURCES.iter().find(|s| {
        Url::parse(s.url).ok().and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string())).as_deref()
            == Some(domain.trim_start_matches("www."))
    })
}

pub fn by_id(id: &str) -> Option<&'static Source> {
    SOURCES.iter().find(|s| s.id == id)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub url: String,
    pub title: String,
    pub kind: String,
    pub year: Option<i32>,
    pub imdb: Option<String>,
    pub remote_id: Option<i64>,
    #[serde(default)]
    pub season: Option<i64>,
    #[serde(default)]
    pub episode: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Variant {
    pub id: String,
    pub url: String,
    pub label: String,
    pub season: Option<i64>,
    pub episode: Option<i64>,
}

#[derive(Clone, Default, Debug)]
pub struct Media {
    pub url: String,
    pub audio: Vec<String>,
    pub subtitles: Vec<(String, String)>,
    pub duration: f64,
}

fn cache() -> &'static Mutex<HashMap<String, (i64, Media)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (i64, Media)>>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

pub fn metadata(url: &str) -> Option<Media> {
    cache().lock().get(url).map(|(_, m)| m.clone())
}

pub fn hls_manifest(text: &str, media: &Media, id: i64) -> String {
    let group = regex::Regex::new(r#"GROUP-ID="([^"]+)""#).unwrap();
    let name = regex::Regex::new(r#"NAME="[^"]*""#).unwrap();
    let track_index = regex::Regex::new(r#"NAME="[^"0-9]*(\d+)""#).unwrap();
    let stream_group = regex::Regex::new(r#"SUBTITLES="([^"]+)""#).unwrap();
    let clean = |s: &str| s.replace(['\r', '\n', '"'], " ");
    let mut audio_counts: HashMap<String, usize> = HashMap::new();
    let mut subtitle_groups = HashSet::new();
    for line in text.lines() {
        if line.starts_with("#EXT-X-MEDIA:TYPE=SUBTITLES")
            && let Some(c) = group.captures(line)
        {
            subtitle_groups.insert(c[1].to_string());
        }
        if let Some(c) = stream_group.captures(line) {
            subtitle_groups.insert(c[1].to_string());
        }
    }
    if subtitle_groups.is_empty() {
        subtitle_groups.insert("nova-subtitles".into());
    }
    let mut lines = Vec::new();
    for line in text.lines() {
        let mut line = line.to_string();
        if line.starts_with("#EXT-X-MEDIA:TYPE=AUDIO") {
            let key = group.captures(&line).map(|c| c[1].to_string()).unwrap_or_default();
            let n = audio_counts.entry(key).or_default();
            let index = track_index.captures(&line).and_then(|c| c[1].parse::<usize>().ok()).unwrap_or(*n);
            if let Some(label) = media.audio.get(index).filter(|s| !s.is_empty()) {
                line = name.replace(&line, format!("NAME=\"{}\"", clean(label))).into_owned();
            }
            *n += 1;
        }
        if !media.subtitles.is_empty() && line.starts_with("#EXT-X-STREAM-INF:") && !stream_group.is_match(&line) {
            line.push_str(",SUBTITLES=\"nova-subtitles\"");
        }
        lines.push(line);
        if lines.len() == 1 && !media.subtitles.is_empty() {
            subtitle_groups.insert("nova-subtitles".into());
            for group in &subtitle_groups {
                for (index, (label, _)) in media.subtitles.iter().enumerate() {
                    lines.push(format!("#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID=\"{}\",NAME=\"{}\",DEFAULT=NO,AUTOSELECT=NO,URI=\"/api/links/{id}/stream/subtitle/{index}.m3u8\"",clean(group),clean(label)));
                }
            }
        }
    }
    lines.join("\n")
}
fn remember(media: Media) -> Media {
    let now = util::now_ms();
    let mut c = cache().lock();
    c.retain(|_, (at, _)| now - *at < 15 * 60_000);
    if c.len() >= 3000 {
        c.clear();
    }
    c.insert(media.url.clone(), (now, media.clone()));
    media
}

pub fn decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&#038;", "&")
        .replace("&#38;", "&")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&nbsp;", " ")
}
fn plain(s: &str) -> String {
    let re = regex::Regex::new("<[^>]*>").unwrap();
    decode(&re.replace_all(s, " ")).split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn check_page(html: &str) -> anyhow::Result<()> {
    let h = html.to_lowercase();
    anyhow::ensure!(
        !h.contains("подтвердите, что вы человек") && !h.contains("cf-chl-") && !h.contains("captcha-container"),
        "Сайт требует проверку человека; импорт остановлен"
    );
    anyhow::ensure!(
        !h.contains("<title>404 not found") && !h.contains("sorry, your request has been denied"),
        "Сайт или встроенный плеер отказал в доступе"
    );
    Ok(())
}

pub async fn json(url: &str) -> anyhow::Result<Value> {
    let text = stream_link::page(url).await?;
    Ok(serde_json::from_str(&text)?)
}

/// Читает только JSON-значение после ключа JavaScript. Код страницы не исполняется.
pub fn json_after(html: &str, key: &str) -> Vec<Value> {
    let re =
        regex::Regex::new(&format!(r#"(?:\b{}|"{}")\s*:\s*([\[{{])"#, regex::escape(key), regex::escape(key))).unwrap();
    re.captures_iter(html)
        .filter_map(|c| {
            serde_json::Deserializer::from_str(&html[c.get(1)?.start()..]).into_iter::<Value>().next()?.ok()
        })
        .collect()
}

fn number(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.as_str()?.parse().ok())
}

fn movie_objects(html: &str) -> Vec<Value> {
    let re = regex::Regex::new(r"\bsource\s*:\s*\{").unwrap();
    let mut values = Vec::new();
    for m in re.find_iter(html) {
        let start = m.end() - 1;
        let mut depth = 0;
        let mut quoted = false;
        let mut escaped = false;
        let mut end = None;
        for (i, c) in html[start..].char_indices() {
            if quoted {
                if escaped {
                    escaped = false
                } else if c == '\\' {
                    escaped = true
                } else if c == '"' {
                    quoted = false
                }
                continue;
            }
            if c == '"' {
                quoted = true
            } else if c == '{' {
                depth += 1
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    end = Some(start + i + 1);
                    break;
                }
            }
        }
        let Some(end) = end else { continue };
        let body = &html[start..end];
        let mut value = serde_json::Map::new();
        for key in ["hls", "audio", "cc", "duration"] {
            let field = regex::Regex::new(&format!(r#"(?:\b{}|"{}")\s*:\s*"#, key, key)).unwrap();
            if let Some(m) = field.find(body)
                && let Some(Ok(v)) = serde_json::Deserializer::from_str(&body[m.end()..]).into_iter::<Value>().next()
            {
                value.insert(key.into(), v);
            }
        }
        values.push(Value::Object(value));
    }
    values
}
fn media_value(v: &Value) -> Option<Media> {
    let url = v["hls"].as_str()?;
    if !Url::parse(url).ok().is_some_and(|u| outbound::valid_url(&u) && u.path().ends_with(".m3u8")) {
        return None;
    }
    let names: Vec<String> =
        v["audio"]["names"].as_array().into_iter().flatten().filter_map(|n| n.as_str().map(String::from)).collect();
    let order = v["audio"]["order"].as_array();
    let audio = order
        .map(|o| {
            let max = o.iter().filter_map(number).filter(|n| *n >= 0 && *n < 256).max().unwrap_or(0) as usize;
            let mut labels = vec![String::new(); max + 1];
            for (i, n) in o.iter().enumerate() {
                if let Some(n) = number(n).filter(|n| *n >= 0 && *n < 256)
                    && let Some(name) = names.get(i)
                {
                    labels[n as usize] = name.clone();
                }
            }
            labels
        })
        .unwrap_or(names);
    let subtitles = v["cc"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| {
            let url = Url::parse(s["url"].as_str()?).ok()?;
            (outbound::valid_url(&url) && url.path().ends_with(".vtt"))
                .then(|| (s["name"].as_str().unwrap_or("Субтитры").to_string(), url.to_string()))
        })
        .collect();
    Some(Media { url: url.into(), audio, subtitles, duration: v["duration"].as_f64().unwrap_or(0.0) })
}

pub fn structured(html: &str, base: &Url) -> Vec<(Variant, Media)> {
    let mut out = Vec::new();
    for seasons in json_after(html, "seasons") {
        for s in seasons.as_array().into_iter().flatten() {
            if s["blocked"] == true {
                continue;
            }
            let Some(season) = number(&s["season"]).filter(|s| *s >= 0) else { continue };
            for e in s["episodes"].as_array().into_iter().flatten() {
                if e["blocked"] == true {
                    continue;
                }
                let Some(episode) = number(&e["episode"]).filter(|e| *e > 0) else { continue };
                let Some(media) = media_value(e) else { continue };
                let mut url = base.clone();
                let mut parts: Vec<(String, String)> =
                    url::form_urlencoded::parse(base.fragment().unwrap_or("").as_bytes())
                        .into_owned()
                        .filter(|(k, _)| k == "nova-origin")
                        .collect();
                parts.push(("nova-season".into(), season.to_string()));
                parts.push(("episode".into(), episode.to_string()));
                url.set_fragment(Some(
                    &url::form_urlencoded::Serializer::new(String::new()).extend_pairs(parts).finish(),
                ));
                out.push((
                    Variant {
                        id: format!("{}:{season}:{episode}", util::sha256_hex(base.as_str().as_bytes())),
                        url: url.into(),
                        label: e["title"]
                            .as_str()
                            .map(String::from)
                            .unwrap_or_else(|| format!("Сезон {season} · серия {episode}")),
                        season: Some(season),
                        episode: Some(episode),
                    },
                    media,
                ));
            }
        }
    }
    if out.is_empty() {
        for value in json_after(html, "source").into_iter().chain(movie_objects(html)) {
            if let Some(media) = media_value(&value) {
                out.push((
                    Variant {
                        id: util::sha256_hex(base.as_str().as_bytes()),
                        url: base.to_string(),
                        label: "Все качества и озвучки".into(),
                        season: None,
                        episode: None,
                    },
                    media,
                ));
            }
        }
    }
    let mut seen = HashSet::new();
    out.retain(|(v, _)| seen.insert(v.url.clone()));
    out
}

pub fn is_provider(u: &Url) -> bool {
    matches!(u.host_str(), Some("api.namy.ws" | "api.nextembed.ws" | "player.aksor.tv"))
        || (u.host_str() == Some("ru.yummyani.me") && u.path() == "/iframeCVH.html")
}

fn with_origin(mut url: Url, source: &Url) -> Url {
    if source.host_str().and_then(by_domain).is_some() {
        let mut values: Vec<(String, String)> = url::form_urlencoded::parse(url.fragment().unwrap_or("").as_bytes())
            .into_owned()
            .filter(|(k, _)| k != "nova-origin")
            .collect();
        values.push(("nova-origin".into(), format!("{}/", source.origin().ascii_serialization())));
        let value = url::form_urlencoded::Serializer::new(String::new()).extend_pairs(values).finish();
        url.set_fragment(Some(&value));
    }
    url
}

pub async fn resolve_player(url: &Url) -> anyhow::Result<Option<Media>> {
    if !is_provider(url) && !url.fragment().is_some_and(|f| f.contains("nova-season=")) {
        return Ok(None);
    }
    let fragment: HashMap<_, _> =
        url::form_urlencoded::parse(url.fragment().unwrap_or("").as_bytes()).into_owned().collect();
    if url.host_str() == Some("ru.yummyani.me") && url.path() == "/iframeCVH.html" {
        let id = fragment
            .get("nova-cvh")
            .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
            .ok_or_else(|| anyhow::anyhow!("У варианта CVH нет идентификатора видео; повторите импорт"))?;
        let data = json(&format!("https://plapi.cdnvideohub.com/api/v1/player/sv/video/{id}")).await?;
        let key = fragment.get("nova-format").map(String::as_str).unwrap_or("hlsUrl");
        anyhow::ensure!(cvh_formats().iter().any(|(k, _)| *k == key), "Неизвестный формат CVH");
        let raw = data["sources"][key]
            .as_str()
            .filter(|u| !u.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Выбранный поток CVH недоступен"))?;
        anyhow::ensure!(outbound::valid_url(&Url::parse(raw)?), "Непубличный адрес CVH");
        return Ok(Some(remember(Media {
            url: raw.into(),
            duration: data["duration"].as_f64().unwrap_or(0.0),
            ..Default::default()
        })));
    }
    if url.host_str() == Some("player.aksor.tv") {
        let id = url
            .path()
            .strip_prefix("/video/")
            .filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| anyhow::anyhow!("Некорректный идентификатор Aksor"))?;
        let data = json(&format!("https://player.aksor.tv/api/video/{id}")).await?;
        let wanted = fragment.get("nova-quality").map(String::as_str);
        let qualities = data["qualities"].as_object().ok_or_else(|| anyhow::anyhow!("Aksor не вернул качества"))?;
        let raw = if let Some(key) = wanted {
            qualities.get(key).and_then(Value::as_str)
        } else {
            ["q4k", "q2k", "q1080", "q720", "q480", "q360"]
                .into_iter()
                .find_map(|k| qualities.get(k)?.as_str().filter(|u| !u.is_empty()))
        }
        .ok_or_else(|| anyhow::anyhow!("Выбранное качество Aksor недоступно"))?;
        let parsed = Url::parse(raw)?;
        anyhow::ensure!(outbound::valid_url(&parsed), "Непубличный адрес Aksor");
        anyhow::ensure!(
            [".mpd", ".m3u8", ".mp4"].iter().any(|ext| parsed.path().ends_with(ext)),
            "Неподдерживаемый формат Aksor"
        );
        return Ok(Some(remember(Media { url: raw.into(), ..Default::default() })));
    }
    let page = url.clone();
    let html = stream_link::page(page.as_str()).await?;
    check_page(&html)?;
    let found = structured(&html, &page);
    let season = fragment.get("nova-season").and_then(|n| n.parse::<i64>().ok());
    let episode = fragment.get("episode").and_then(|n| n.parse::<i64>().ok());
    let selected = found
        .into_iter()
        .find(|(v, _)| v.season == season && v.episode == episode)
        .ok_or_else(|| anyhow::anyhow!("Выбранная серия не найдена в плейлисте источника"))?;
    Ok(Some(remember(selected.1)))
}

pub fn listing(html: &str, base: &Url, source: &Source) -> Vec<Entry> {
    let anchors = regex::Regex::new(r#"(?is)<a\b([^>]*)>(.*?)</a>"#).unwrap();
    let href = regex::Regex::new(r#"(?i)\bhref=["']([^"']+)["']"#).unwrap();
    let title = regex::Regex::new(r#"(?is)<[^>]+class=["'][^"']*th-title[^"']*["'][^>]*>(.*?)</[^>]+>"#).unwrap();
    let year = regex::Regex::new(r"\b((?:19|20)\d{2})\b").unwrap();
    let path_year = regex::Regex::new(r"(?i)-((?:19|20)\d{2})\.html$").unwrap();
    let mut seen = HashSet::new();
    anchors
        .captures_iter(html)
        .filter_map(|a| {
            let mut u = base.join(&decode(&href.captures(&a[1])?[1])).ok()?;
            u.set_fragment(None);
            if u.host_str() != base.host_str()
                || !u.path().ends_with(".html")
                || !outbound::valid_url(&u)
                || !seen.insert(u.to_string())
            {
                return None;
            }
            let name = if let Some(t) = title.captures(&a[2]) {
                plain(&t[1])
            } else if source.id == "potter" {
                plain(&a[2])
            } else {
                return None;
            };
            if name.is_empty() {
                return None;
            }
            let found_year =
                path_year.captures(u.path()).or_else(|| year.captures(&a[2])).and_then(|y| y[1].parse().ok());
            Some(Entry {
                url: u.into(),
                title: name,
                kind: source.kind.into(),
                year: found_year,
                imdb: None,
                remote_id: None,
                season: None,
                episode: None,
            })
        })
        .collect()
}

pub fn next_page(html: &str, base: &Url) -> Option<String> {
    let page_re = regex::Regex::new(r"/page/(\d+)/").unwrap();
    let n = page_re.captures(base.path()).and_then(|c| c[1].parse::<u32>().ok()).unwrap_or(1) + 1;
    let anchors = regex::Regex::new(r#"(?i)href=["']([^"']+)["']"#).unwrap();

    anchors
        .captures_iter(html)
        .filter_map(|c| base.join(&decode(&c[1])).ok())
        .find(|u| {
            u.host_str() == base.host_str()
                && page_re.captures(u.path()).is_some_and(|c| c[1].parse::<u32>().ok() == Some(n))
        })
        .map(String::from)
}

pub async fn branches(source: &Source, cursor: &str, app: Option<&App>) -> anyhow::Result<Vec<String>> {
    if source.title.is_some() && source.id != "bear" {
        let (base, html) = stream_link::page_for(app, cursor).await?;
        check_page(&html)?;
        return Ok(episode_links(&html, &base, source));
    }
    if source.id != "lordserials" || cursor != source.url {
        return Ok(Vec::new());
    }
    let (base, html) = stream_link::page_for(app, cursor).await?;
    let re = regex::Regex::new(r#"href=["']([^"']+)["']"#).unwrap();
    let mut result = Vec::new();
    for c in re.captures_iter(&html) {
        if let Ok(u) = base.join(&decode(&c[1]))
            && u.host_str() == base.host_str()
            && matches!(u.path(), "/zarubezhnye-serialy/" | "/multserialy/" | "/anime/" | "/doramy/" | "/dorama/")
            && !result.contains(&u.to_string())
        {
            result.push(u.to_string());
        }
    }
    Ok(result)
}

fn episode_links(html: &str, base: &Url, source: &Source) -> Vec<String> {
    let href = regex::Regex::new(r#"href=["']([^"']+)["']"#).unwrap();
    let mut result = Vec::new();
    for c in href.captures_iter(html) {
        if let Ok(mut u) = base.join(&decode(&c[1])) {
            u.set_fragment(None);
            let same = u.host_str().map(|h| h.trim_start_matches("www."))
                == base.host_str().map(|h| h.trim_start_matches("www."));
            let episode = u.path().starts_with("/episodes/") || u.path().starts_with("/seasons/");
            let park = source.id == "southpark"
                && u.path() == "/page.php"
                && u.query_pairs().any(|(k, v)| k == "id" && v.parse::<u32>().is_ok());
            if same && outbound::valid_url(&u) && (episode || park) && !result.contains(&u.to_string()) {
                result.push(u.to_string());
            }
        }
    }
    result
}

pub fn episode_entries(html: &str, base: &Url, source: &Source) -> Vec<Entry> {
    let episode = regex::Regex::new(r"/episodes/(\d+)-sezon-(\d+)-seri[^/]*").unwrap();
    let title = regex::Regex::new(r"(?is)<(?:h1|title)\b[^>]*>(.*?)</(?:h1|title)>").unwrap();
    let numbers = regex::Regex::new(r"(?i)(\d+)\s*сезон.*?(\d+)\s*сери").unwrap();
    let mut urls = episode_links(html, base, source);
    urls.insert(0, base.to_string());
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for raw in urls {
        let Ok(u) = Url::parse(&raw) else { continue };
        let pair = if let Some(c) = episode.captures(u.path()) {
            Some((c[1].parse::<i64>().unwrap_or(-1), c[2].parse::<i64>().unwrap_or(0)))
        } else if source.id == "southpark" && u == *base {
            if u.query_pairs().any(|(k, v)| k == "id" && v == "101") {
                Some((1, 1))
            } else {
                let labels = title.captures_iter(html).map(|c| plain(&c[1])).collect::<Vec<_>>().join(" ");
                numbers.captures(&labels).map(|c| (c[1].parse().unwrap_or(-1), c[2].parse().unwrap_or(0)))
            }
        } else {
            None
        };
        let Some((season, ep)) = pair.filter(|(s, e)| *s >= 0 && *e > 0) else { continue };
        if !seen.insert(u.to_string()) {
            continue;
        }
        out.push(Entry {
            url: u.into(),
            title: source.title.unwrap_or(source.name).into(),
            kind: "series".into(),
            year: None,
            imdb: source.imdb.map(String::from),
            remote_id: None,
            season: Some(season),
            episode: Some(ep),
        });
    }
    out
}

pub fn yummy_entries(value: &Value) -> Vec<Entry> {
    value["response"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| {
            let id = number(&v["anime_id"])?;
            let slug = v["anime_url"].as_str()?;
            let title = v["title"].as_str()?;
            let u = Url::parse("https://old.yummyani.me/").ok()?.join(&format!("catalog/item/{slug}")).ok()?;
            (u.host_str() == Some("old.yummyani.me") && u.path().starts_with("/catalog/item/")).then(|| Entry {
                url: u.into(),
                title: title.into(),
                kind: if v["type"]["alias"] == "movie" { "movie" } else { "series" }.into(),
                year: v["year"].as_i64().and_then(|y| i32::try_from(y).ok()),
                imdb: None,
                remote_id: Some(id),
                season: None,
                episode: None,
            })
        })
        .collect()
}

/// Найти только выбранный тайтл и серию, не обходя и не импортируя каталог.
pub async fn yummy_find(
    names: &[&str],
    year: Option<i64>,
    season: Option<i64>,
    episode: Option<i64>,
) -> Option<Vec<Value>> {
    use futures_util::StreamExt;
    if season.is_some_and(|s| s != 1) || season.is_some() != episode.is_some() {
        return Some(vec![]);
    }
    let mut found = None;
    for name in names.iter().filter(|n| !n.trim().is_empty()).take(2) {
        let url =
            Url::parse_with_params("https://old.yummyani.me/api/search", &[("q", *name), ("limit", "20")]).ok()?;
        let data = json(url.as_str()).await.ok()?;
        let candidates: Vec<Entry> = yummy_entries(&data)
            .into_iter()
            .filter(|entry| {
                names.iter().any(|n| util::norm(n) == util::norm(&entry.title))
                    && year.is_none_or(|y| entry.year == Some(y as i32))
            })
            .collect();
        if candidates.len() == 1 {
            found = candidates.into_iter().next();
            break;
        }
    }
    let Some(entry) = found else { return Some(vec![]) };
    // Без выбранной серии не раскрываем сотни записей сериала и не сканируем весь каталог.
    if entry.kind == "series" && episode.is_none() {
        return Some(vec![]);
    }
    let data = json(&format!("https://old.yummyani.me/api/anime/{}/videos", entry.remote_id?)).await.ok()?;
    let videos = data["response"].as_array()?;
    let candidates: Vec<Value> = videos
        .iter()
        .filter(|v| {
            episode.is_none_or(|wanted| v["number"].as_str().and_then(|v| v.parse::<i64>().ok()) == Some(wanted))
        })
        .cloned()
        .collect();
    let each = futures_util::stream::iter(candidates.into_iter().map(|video| {
        let entry = entry.clone();
        async move { expand_yummy(&entry, &video, &[]).await.ok() }
    }))
    .buffer_unordered(4)
    .collect::<Vec<_>>()
    .await;
    let mut out = Vec::new();
    for (variants, _) in each.into_iter().flatten() {
        for variant in variants {
            let Ok(url) = Url::parse(&variant.url) else { continue };
            if !is_provider(&url) {
                continue;
            }
            let ep = match (season, episode) {
                (Some(s), Some(e)) => format!(" {s} сезон {e} серия"),
                _ => String::new(),
            };
            let year = entry.year.map(|y| format!(" ({y})")).unwrap_or_default();
            out.push(serde_json::json!({"site":"own","url":variant.url,"label":format!("{}{year}{ep} · {}",entry.title,variant.label),"thumbUrl":null,"duration":null,"author":"old.yummyani.me"}));
        }
    }
    Some(out)
}

pub async fn list(
    source: &Source,
    cursor: Option<&str>,
    app: Option<&App>,
) -> anyhow::Result<(Vec<Entry>, Option<String>)> {
    if source.id == "yummy" {
        let url = cursor.unwrap_or("https://old.yummyani.me/api/anime?limit=50&offset=0");
        let value = json(url).await?;
        let entries = yummy_entries(&value);
        let offset = Url::parse(url)?
            .query_pairs()
            .find(|(k, _)| k == "offset")
            .and_then(|(_, v)| v.parse::<usize>().ok())
            .unwrap_or(0);
        let next =
            (entries.len() == 50).then(|| format!("https://old.yummyani.me/api/anime?limit=50&offset={}", offset + 50));
        return Ok((entries, next));
    }
    let (base, html) = stream_link::page_for(app, cursor.unwrap_or(source.url)).await?;
    check_page(&html)?;
    if let Some(title) = source.title {
        if source.id != "bear" {
            let entries = episode_entries(&html, &base, source);
            if !entries.is_empty() {
                return Ok((entries, None));
            }
            if !episode_links(&html, &base, source).is_empty() {
                return Ok((Vec::new(), None));
            }
        }
        let mut target = base.clone();
        if source.id == "bear" {
            let re = regex::Regex::new(r#"href=["']([^"']*/film/medved-1-sezon[^"']*)["']"#).unwrap();
            if let Some(c) = re.captures(&html) {
                target = base.join(&decode(&c[1]))?;
            }
        }
        return Ok((
            vec![Entry {
                url: target.into(),
                title: title.into(),
                kind: source.kind.into(),
                year: None,
                imdb: source.imdb.map(String::from),
                remote_id: None,
                season: None,
                episode: None,
            }],
            None,
        ));
    }
    let entries = listing(&html, &base, source);
    anyhow::ensure!(!entries.is_empty(), "Страница открылась, но записи каталога не найдены");
    let next = next_page(&html, &base);
    Ok((entries, next))
}

pub async fn variants(entry: &Entry, app: Option<&App>) -> anyhow::Result<(Vec<Variant>, Vec<String>)> {
    if let Some(id) = entry.remote_id {
        use futures_util::StreamExt;
        let data = json(&format!("https://old.yummyani.me/api/anime/{id}/videos")).await?;
        let videos = data["response"].as_array().ok_or_else(|| anyhow::anyhow!("YummyAnime не вернул массив видео"))?;
        let mut specials: Vec<String> = videos
            .iter()
            .filter_map(|v| v["number"].as_str())
            .filter(|s| s.parse::<i64>().ok().is_none_or(|n| n < 1))
            .map(String::from)
            .collect();
        specials.sort();
        specials.dedup();
        let each = futures_util::stream::iter(videos.clone().into_iter().map(|v| {
            let entry = entry.clone();
            let specials = specials.clone();
            async move { expand_yummy(&entry, &v, &specials).await }
        }))
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;
        let mut out = Vec::new();
        let mut notes = Vec::new();
        for result in each {
            match result {
                Ok((variants, errors)) => {
                    out.extend(variants);
                    notes.extend(errors);
                }
                Err(e) => notes.push(e.to_string()),
            }
        }
        notes.sort();
        notes.dedup();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        anyhow::ensure!(!out.is_empty(), "В YummyAnime не найдены варианты видео: {}", notes.join("; "));
        return Ok((out, notes));
    }
    let (base, html) = stream_link::page_for(app, &entry.url).await?;
    check_page(&html)?;
    let mut frames = stream_link::find_iframes(&html, &base);
    if is_provider(&base) {
        frames = vec![base.clone()]
    }
    let mut variants = Vec::new();
    for (v, m) in structured(&html, &base) {
        remember(m);
        variants.push(v);
    }
    let mut notes = Vec::new();
    let mut seen = HashSet::new();
    for frame in frames {
        let frame = with_origin(frame, &base);
        if !seen.insert(frame.to_string()) {
            continue;
        }
        if frame.host_str().is_some_and(|h| h.contains("youtube") || h == "vk.com") {
            continue;
        }
        match stream_link::page(frame.as_str()).await {
            Ok(html) => {
                if let Err(e) = check_page(&html) {
                    notes.push(format!("{}: {e}", frame.host_str().unwrap_or("плеер")));
                    continue;
                }
                let found = structured(&html, &frame);
                if found.is_empty() {
                    if entry.kind == "movie" || entry.episode.is_some() {
                        variants.push(Variant {
                            id: util::sha256_hex(frame.as_str().as_bytes()),
                            url: frame.to_string(),
                            label: "Плеер источника - поток не подтверждён".into(),
                            season: entry.season,
                            episode: entry.episode,
                        });
                    }
                    notes
                        .push(format!("{}: структурированный плейлист не найден", frame.host_str().unwrap_or("плеер")));
                } else {
                    for (v, m) in found {
                        remember(m);
                        variants.push(v);
                    }
                }
            }
            Err(e) => notes.push(format!("{}: {e}", frame.host_str().unwrap_or("плеер"))),
        }
    }
    if variants.is_empty()
        && (entry.kind == "movie" || entry.episode.is_some())
        && stream_link::find_media(&html).is_some()
    {
        variants.push(Variant {
            id: "page".into(),
            url: base.into(),
            label: "Все качества и озвучки".into(),
            season: entry.season,
            episode: entry.episode,
        })
    }
    if entry.episode.is_some() {
        variants.retain_mut(|v| {
            if v.episode.is_none() {
                v.season = entry.season;
                v.episode = entry.episode;
                true
            } else {
                v.season == entry.season && v.episode == entry.episode
            }
        });
    }
    anyhow::ensure!(!variants.is_empty(), "Не найден список доступных серий или потоков: {}", notes.join("; "));
    Ok((variants, notes))
}

async fn expand_yummy(entry: &Entry, v: &Value, specials: &[String]) -> anyhow::Result<(Vec<Variant>, Vec<String>)> {
    let video = number(&v["video_id"]).ok_or_else(|| anyhow::anyhow!("У записи YummyAnime нет video_id"))?;
    let raw = v["iframe_url"].as_str().ok_or_else(|| anyhow::anyhow!("У видео {video} нет адреса плеера"))?;
    let u = Url::parse(&entry.url)?.join(raw)?;
    anyhow::ensure!(outbound::valid_url(&u), "Непубличный адрес плеера YummyAnime");
    let num = v["number"].as_str().unwrap_or("");
    let normal = num.parse::<i64>().ok().filter(|n| *n > 0);
    let (season, episode) = if entry.kind == "movie" {
        (None, None)
    } else if let Some(n) = normal {
        (Some(1), Some(n))
    } else {
        (Some(0), Some(specials.iter().position(|s| s == num).unwrap_or(0) as i64 + 1))
    };
    let label = format!(
        "{} · {}{}",
        v["data"]["dubbing"].as_str().unwrap_or("Озвучка"),
        v["data"]["player"].as_str().unwrap_or("Плеер"),
        if season == Some(0) { format!(" · спецвыпуск {num}") } else { String::new() }
    );
    let fallback = Variant { id: format!("yummy:{video}"), url: u.to_string(), label: label.clone(), season, episode };
    if u.host_str() == Some("ru.yummyani.me") && u.path() == "/iframeCVH.html" {
        return match cvh_variants(&u, &fallback).await {
            Ok(variants) => Ok((variants, Vec::new())),
            Err(e) => Ok((vec![fallback], vec![format!("CVH: {e}")])),
        };
    }
    if u.host_str() != Some("player.aksor.tv") {
        return Ok((
            vec![fallback],
            vec![format!(
                "{}: собственный поток этого плеера пока не подтверждён",
                v["data"]["player"].as_str().unwrap_or("Плеер")
            )],
        ));
    }
    let id = u.path().strip_prefix("/video/").ok_or_else(|| anyhow::anyhow!("Неподдерживаемый адрес Aksor"))?;
    let data = match json(&format!("https://player.aksor.tv/api/video/{id}")).await {
        Ok(v) => v,
        Err(e) => return Ok((vec![fallback], vec![format!("Aksor: {e}")])),
    };
    let mut result = Vec::new();
    for (key, value) in data["qualities"].as_object().into_iter().flatten() {
        let Some(raw) = value.as_str().filter(|s| !s.is_empty()) else { continue };
        let media = Url::parse(raw)?;
        anyhow::ensure!(outbound::valid_url(&media), "Aksor вернул непубличный адрес видео");
        let mut link = u.clone();
        link.set_fragment(Some(&format!("nova-quality={key}")));
        let quality = match key.as_str() {
            "q4k" => "2160p",
            "q2k" => "1440p",
            "q1080" => "1080p",
            "q720" => "720p",
            "q480" => "480p",
            "q360" => "360p",
            s => s,
        };
        result.push(Variant {
            id: format!("yummy:{video}:{key}"),
            url: link.to_string(),
            label: format!("{label} · {quality}"),
            season,
            episode,
        });
    }
    if result.is_empty() {
        return Ok((vec![fallback], vec!["Aksor не вернул доступное качество".into()]));
    }
    Ok((result, Vec::new()))
}

fn cvh_formats() -> &'static [(&'static str, &'static str)] {
    &[
        ("hlsUrl", "Все качества HLS"),
        ("mpegTinyUrl", "Минимальное"),
        ("mpegLowestUrl", "Низкое"),
        ("mpegLowUrl", "Ниже среднего"),
        ("mpegMediumUrl", "Среднее"),
        ("mpegHighUrl", "Высокое"),
        ("mpegFullHdUrl", "Full HD"),
        ("mpegQhdUrl", "QHD"),
        ("mpeg2kUrl", "2K"),
        ("mpeg4kUrl", "4K"),
    ]
}

/// Сопоставляем именно номер серии и код озвучки, а не первый элемент ответа API.
pub fn cvh_video<'a>(value: &'a Value, voice: &str, episode: i64, season: Option<i64>) -> anyhow::Result<&'a str> {
    let matches: Vec<_> = value["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|v| {
            v["voiceStudio"].as_str().is_some_and(|s| s.eq_ignore_ascii_case(voice))
                && number(&v["episode"]) == Some(episode)
                && season.is_none_or(|s| number(&v["season"]) == Some(s))
        })
        .collect();
    anyhow::ensure!(matches.len() == 1, "Нет однозначного видео для выбранной серии и озвучки CVH");
    matches[0]["vkId"]
        .as_str()
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| anyhow::anyhow!("CVH не вернул идентификатор видео"))
}

pub async fn cvh_variants(url: &Url, fallback: &Variant) -> anyhow::Result<Vec<Variant>> {
    let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
    let anime = params
        .get("anime_id")
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| anyhow::anyhow!("У CVH нет anime_id"))?;
    let voice = params.get("dubbing_code").ok_or_else(|| anyhow::anyhow!("У CVH нет кода озвучки"))?;
    let episode = params
        .get("episode")
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|n| *n > 0)
        .ok_or_else(|| anyhow::anyhow!("У CVH нет номера серии"))?;
    let season = params.get("season").and_then(|s| s.parse::<i64>().ok());
    let mut api = Url::parse("https://plapi.cdnvideohub.com/api/v1/player/sv/playlist")?;
    api.query_pairs_mut().extend_pairs([("pub", "745"), ("aggr", "mali"), ("id", anime), ("ep", &episode.to_string())]);
    if let Some(season) = season {
        api.query_pairs_mut().append_pair("sn", &season.to_string());
    }
    let value = json(api.as_str()).await?;
    let video = cvh_video(&value, voice, episode, season)?;
    let data = json(&format!("https://plapi.cdnvideohub.com/api/v1/player/sv/video/{video}")).await?;
    let mut variants = Vec::new();
    for (key, name) in cvh_formats() {
        let Some(raw) = data["sources"][key].as_str().filter(|s| !s.is_empty()) else { continue };
        anyhow::ensure!(outbound::valid_url(&Url::parse(raw)?), "Непубличный адрес видео CVH");
        let mut link = url.clone();
        link.set_fragment(Some(&format!("nova-cvh={video}&nova-format={key}")));
        variants.push(Variant {
            id: format!("{}:cvh:{video}:{key}", fallback.id),
            url: link.into(),
            label: format!("{} · {name}", fallback.label),
            season: fallback.season,
            episode: fallback.episode,
        });
    }
    anyhow::ensure!(!variants.is_empty(), "CVH не вернул доступных потоков");
    Ok(variants)
}

pub async fn match_imdb(app: &App, entry: &Entry) -> anyhow::Result<String> {
    if let Some(id) = &entry.imdb {
        return Ok(id.clone());
    }
    let local = app.db.with(|c| {
        let mut statement = c.prepare(
            "SELECT id FROM ext_titles WHERE id LIKE 'tt%' AND title=?1 AND kind=?2 AND (?3 IS NULL OR year=?3) LIMIT 2",
        )?;
        let ids = statement.query_map(rusqlite::params![entry.title, entry.kind, entry.year], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(ids)
    })?;
    anyhow::ensure!(local.len() < 2, "Несколько совпадений IMDb - назначьте тайтл вручную");
    if let Some(id) = local.first() {
        return Ok(id.clone());
    }
    let candidates = tokio::time::timeout(
        Duration::from_secs(10),
        crate::meta::imdb::Imdb::new(app.http.clone()).search(&entry.title, entry.year, entry.kind == "series"),
    )
    .await??;
    let title = util::norm(&entry.title);
    let mut matches = candidates.into_iter().filter(|c| {
        c.series == (entry.kind == "series")
            && entry.year.is_none_or(|y| c.year == Some(y))
            && (util::norm(&c.title) == title || c.original.as_ref().is_some_and(|n| util::norm(n) == title))
    });
    let first =
        matches.next().ok_or_else(|| anyhow::anyhow!("Нет точного совпадения IMDb - назначьте тайтл вручную"))?;
    anyhow::ensure!(matches.next().is_none(), "Несколько совпадений IMDb - назначьте тайтл вручную");
    Ok(first.id)
}
