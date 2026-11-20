use reqwest::Url;
use serde_json::{Value, json};

pub const HOST: &str = "sd.lordfilm5.pro";
const BASE: &str = "https://sd.lordfilm5.pro/";

pub fn movie_url(u: &Url) -> bool {
    source_url(u)
        && (u.path().starts_with("/filmy/") || u.path().starts_with("/mult/") || u.path().starts_with("/multfilmy/"))
}

fn source_url(u: &Url) -> bool {
    u.scheme() == "https"
        && u.host_str().is_some_and(|h| h == HOST || h.ends_with(".lordfilm5.pro"))
        && u.port().is_none()
        && u.username().is_empty()
        && u.password().is_none()
        && u.path().ends_with(".html")
}

pub fn series_url(u: &Url) -> bool {
    source_url(u) && u.path().starts_with("/serialy/")
}

/// The fragment identifies an episode locally; it is never sent to the source server.
pub fn episode_target(u: &Url) -> Option<(i64, i64)> {
    if !series_url(u) {
        return None;
    }
    let fragment = u.fragment()?;
    let (season, episode) = fragment.strip_prefix("nova-season=")?.split_once("&episode=")?;
    let (season, episode) = (season.parse::<i64>().ok()?, episode.parse::<i64>().ok()?);
    ((0..=1000).contains(&season) && (1..=10000).contains(&episode)).then_some((season, episode))
}

fn decode(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#039;", "'").replace("&nbsp;", " ").replace("&amp;", "&")
}

fn token_like(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let mut ai = a.chars();
    let mut bi = b.chars();
    let mut common = 0usize;
    for _ in 0..5 {
        match (ai.next(), bi.next()) {
            (Some(x), Some(y)) if x == y => common += 1,
            _ => break,
        }
    }
    let a_len = a.chars().count();
    let b_len = b.chars().count();
    // Common 5-char prefix handles Russian inflections (девятый/девятого); short stems need full prefix.
    (common >= 5 && a_len >= 5 && b_len >= 5) || (a_len >= 4 && b_len >= 4 && (a.starts_with(b) || b.starts_with(a)))
}

fn names_match(norm: &str, wanted: &[String], year: Option<i64>, found_year: Option<i64>) -> bool {
    if wanted.iter().any(|w| w == norm) {
        return true;
    }
    // Same or adjacent year: allow local/alternate spelling (sites and IMDb often differ by 1).
    // Card without year: allow strong overlap only when the expected year is known (catalog
    // often omits year), but never treat a clearly conflicting year as a match.
    let year_ok = match (year, found_year) {
        (Some(y), Some(fy)) => (y - fy).abs() <= 1,
        (Some(_), None) => true,
        (None, Some(_)) | (None, None) => false,
    };
    if !year_ok {
        return false;
    }
    let nt: Vec<&str> = norm.split_whitespace().collect();
    wanted.iter().any(|w| {
        let wt: Vec<&str> = w.split_whitespace().collect();
        if wt.is_empty() || nt.is_empty() {
            return false;
        }
        let cover = |a: &[&str], b: &[&str]| a.iter().all(|t| b.iter().any(|x| token_like(t, x)));
        // All significant words of one title must appear in the other, with almost no extra words.
        cover(&wt, &nt) && (wt.len() + 1 >= nt.len() || cover(&nt, &wt))
    })
}

fn parse(html: &str, names: &[&str], year: Option<i64>, series: bool) -> Vec<Value> {
    let base = Url::parse(BASE).expect("source URL");
    // href/class order varies across LordFilm mirrors and layouts.
    let anchors = regex::Regex::new(r#"(?is)<a\b([^>]*)>(.*?)</a>"#).unwrap();
    let href_re = regex::Regex::new(r#"(?is)\bhref\s*=\s*["']([^"']+)["']"#).unwrap();
    let class_re = regex::Regex::new(r#"(?is)\bclass\s*=\s*["']([^"']+)["']"#).unwrap();
    let titles = regex::Regex::new(r#"<div class="th-title">([^<]+)</div>"#).unwrap();
    let years = regex::Regex::new(r#"<div class="th-series">\s*((?:19|20)\d{2})\s*</div>"#).unwrap();
    let years_any = regex::Regex::new(r#"\b((?:19|20)\d{2})\b"#).unwrap();
    let images = regex::Regex::new(r#"<img[^>]+(?:data-src|src)="([^"]+)""#).unwrap();
    let wanted: Vec<String> = names.iter().map(|n| crate::util::norm(n)).filter(|n| !n.is_empty()).collect();
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for c in anchors.captures_iter(html) {
        let attrs = &c[1];
        let Some(class) = class_re.captures(attrs).and_then(|m| m.get(1)).map(|m| m.as_str()) else { continue };
        if !class.split_whitespace().any(|c| c == "th-in" || c.starts_with("th-in")) {
            continue;
        }
        let Some(href) = href_re.captures(attrs).and_then(|m| m.get(1)).map(|m| decode(m.as_str())) else {
            continue;
        };
        let Ok(url) = base.join(&href) else { continue };
        if !(if series { series_url(&url) } else { movie_url(&url) }) {
            continue;
        }
        if !seen.insert(url.to_string()) {
            continue;
        }
        let body = &c[2];
        let name = titles.captures(body).map(|m| decode(&m[1])).or_else(|| {
            // Fallbacks for alternative layouts.
            let img_alt = regex::Regex::new(r#"(?is)<img[^>]*\balt\s*=\s*["']([^"']+)["']"#).ok()?;
            if let Some(m) = img_alt.captures(body) {
                let t = decode(&m[1]);
                if !t.trim().is_empty() {
                    return Some(t);
                }
            }
            let title_attr = regex::Regex::new(r#"(?is)\btitle\s*=\s*["']([^"']+)["']"#).ok()?;
            title_attr.captures(attrs).map(|m| decode(&m[1])).filter(|t| !t.trim().is_empty())
        });
        let Some(name) = name else { continue };
        let found_year = years.captures(body).and_then(|m| m[1].parse::<i64>().ok()).or_else(|| {
            // Some layouts put year as plain text near the title.
            years_any.captures(body).and_then(|m| m[1].parse::<i64>().ok())
        });
        let norm = crate::util::norm(&name);
        // A sequel or a remake must never silently become this film's play button.
        if let (Some(y), Some(fy)) = (year, found_year)
            && (y - fy).abs() > 1
        {
            continue;
        }
        if !names_match(&norm, &wanted, year, found_year) {
            continue;
        }
        let thumb = images
            .captures(body)
            .and_then(|m| base.join(&decode(&m[1])).ok())
            .filter(crate::outbound::valid_url)
            .map(|u| u.to_string());
        let label = found_year.map_or(name.clone(), |y| format!("{name} ({y})"));
        out.push(json!({"site":"lordfilm", "url":url.as_str(), "label":label,
        "thumbUrl":thumb, "duration":null, "author":"LordFilm"}));
        if out.len() >= 8 {
            break;
        }
    }
    out
}
pub async fn search_names(names: &[&str], year: Option<i64>) -> Option<Vec<Value>> {
    let mut out = Vec::<Value>::new();
    let mut seen = std::collections::HashSet::new();
    let mut answered = false;
    for name in names.iter().filter(|n| !n.trim().is_empty()).take(4) {
        let url = match Url::parse_with_params(
            &format!("{BASE}index.php"),
            &[("do", "search"), ("subaction", "search"), ("story", name)],
        ) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let html = match crate::api::stream_link::page(url.as_str()).await {
            Ok(h) => h,
            Err(_) => continue,
        };
        answered = true;
        for item in parse(&html, names, year, false) {
            let Some(u) = item["url"].as_str() else { continue };
            if seen.insert(u.to_string()) {
                out.push(item);
            }
        }
        if out.len() >= 8 {
            break;
        }
    }
    answered.then(|| {
        out.truncate(8);
        out
    })
}

fn episode_media(html: &str, season: i64, episode: i64) -> Option<(String, Option<i64>)> {
    let re = regex::Regex::new(r#"\bseasons\s*:\s*(\[)"#).ok()?;
    for capture in re.captures_iter(html) {
        let start = capture.get(1)?.start();
        let Some(Ok(seasons)) = serde_json::Deserializer::from_str(&html[start..]).into_iter::<Value>().next() else {
            continue;
        };
        let number = |v: &Value| v.as_i64().or_else(|| v.as_str()?.parse::<i64>().ok());
        let Some(s) = seasons.as_array()?.iter().find(|s| number(&s["season"]) == Some(season)) else { continue };
        if s["blocked"] == true {
            continue;
        }
        let Some(e) = s["episodes"].as_array()?.iter().find(|e| number(&e["episode"]) == Some(episode)) else {
            continue;
        };
        if e["blocked"] == true {
            continue;
        }
        let Some(url) = e["hls"].as_str().and_then(|u| Url::parse(u).ok()) else { continue };
        if crate::outbound::valid_url(&url) && url.path().ends_with(".m3u8") {
            return Some((url.into(), e["duration"].as_i64().filter(|d| *d > 0)));
        }
    }
    None
}

pub async fn episode_stream(url: &Url) -> anyhow::Result<(String, Option<i64>)> {
    let (season, episode) = episode_target(url).ok_or_else(|| anyhow::anyhow!("Не выбрана серия LordFilm"))?;
    let mut page_url = url.clone();
    page_url.set_fragment(None);
    let (base, html) = crate::api::stream_link::page_with_url(page_url.as_str()).await?;
    // Only parse the structured playlist of the observed provider. Never pick an arbitrary
    // HLS URL from a series page: that could silently play a different episode.
    let frames = crate::api::stream_link::find_iframes(&html, &base);
    for frame in frames.iter().filter(|u| u.scheme() == "https" && u.host_str() == Some("api.nextembed.ws")) {
        let Ok(html) = crate::api::stream_link::page(frame.as_str()).await else { continue };
        if let Some(media) = episode_media(&html, season, episode) {
            return Ok(media);
        }
    }
    anyhow::bail!("LordFilm не предоставил выбранную серию: сезон {season}, серия {episode}")
}

/// Получить только подтверждённые серии из структурированного плейлиста источника.
fn episode_numbers(html: &str) -> Vec<(i64, i64)> {
    let Ok(re) = regex::Regex::new(r"\bseasons\s*:\s*(\[)") else { return vec![] };
    let mut result = Vec::new();
    for capture in re.captures_iter(html) {
        let Some(start) = capture.get(1).map(|m| m.start()) else { continue };
        let Some(Ok(seasons)) = serde_json::Deserializer::from_str(&html[start..]).into_iter::<Value>().next() else {
            continue;
        };
        for s in seasons.as_array().into_iter().flatten() {
            let number = |v: &Value| v.as_i64().or_else(|| v.as_str()?.parse::<i64>().ok());
            let Some(season) = number(&s["season"]) else { continue };
            if !(0..=1000).contains(&season) || s["blocked"] == true {
                continue;
            }
            for e in s["episodes"].as_array().into_iter().flatten() {
                let Some(episode) = number(&e["episode"]) else { continue };
                if !(1..=10000).contains(&episode) || e["blocked"] == true {
                    continue;
                }
                if e["hls"]
                    .as_str()
                    .and_then(|u| Url::parse(u).ok())
                    .is_some_and(|u| crate::outbound::valid_url(&u) && u.path().ends_with(".m3u8"))
                {
                    result.push((season, episode));
                }
            }
        }
    }
    result.sort_unstable();
    result.dedup();
    result.truncate(500);
    result
}

pub async fn episodes_on_page(url: &Url) -> anyhow::Result<Vec<(i64, i64)>> {
    anyhow::ensure!(series_url(url), "Этот сайт пока не поддерживает автоматический импорт серий");
    let mut page_url = url.clone();
    page_url.set_fragment(None);
    let (base, html) = crate::api::stream_link::page_with_url(page_url.as_str()).await?;
    for frame in crate::api::stream_link::find_iframes(&html, &base)
        .iter()
        .filter(|u| u.scheme() == "https" && u.host_str() == Some("api.nextembed.ws"))
    {
        if let Ok(html) = crate::api::stream_link::page(frame.as_str()).await {
            let episodes = episode_numbers(&html);
            if !episodes.is_empty() {
                return Ok(episodes);
            }
        }
    }
    anyhow::bail!("Список доступных серий не найден")
}

pub async fn search_episode_names(names: &[&str], year: Option<i64>, season: i64, episode: i64) -> Option<Vec<Value>> {
    let mut pages = Vec::<Value>::new();
    let mut seen_page = std::collections::HashSet::new();
    let mut answered = false;
    for name in names.iter().filter(|n| !n.trim().is_empty()).take(4) {
        let url = match Url::parse_with_params(
            &format!("{BASE}index.php"),
            &[("do", "search"), ("subaction", "search"), ("story", name)],
        ) {
            Ok(u) => u,
            Err(_) => continue,
        };
        let html = match crate::api::stream_link::page(url.as_str()).await {
            Ok(h) => h,
            Err(_) => continue,
        };
        answered = true;
        for item in parse(&html, names, year, true) {
            let Some(u) = item["url"].as_str() else { continue };
            if seen_page.insert(u.to_string()) {
                pages.push(item);
            }
        }
        if pages.len() >= 4 {
            break;
        }
    }
    if !answered {
        return None;
    }
    let mut found = Vec::new();
    for mut item in pages.into_iter().take(3) {
        let Some(raw) = item["url"].as_str() else { continue };
        let Ok(mut url) = Url::parse(raw) else { continue };
        url.set_fragment(Some(&format!("nova-season={season}&episode={episode}")));
        if let Ok((_, duration)) = episode_stream(&url).await {
            let Some(label) = item["label"].as_str().map(str::to_owned) else { continue };
            let href = url.to_string();
            item["url"] = json!(href);
            item["label"] = json!(format!("{label} S{season:02}E{episode:02}"));
            item["duration"] = json!(duration);
            found.push(item);
            if found.len() >= 2 {
                break;
            }
        }
    }
    Some(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_movie_and_year_without_sequels_or_foreign_links() {
        let item = |url: &str, name: &str, year: i64| {
            format!(
                r#"<a class="th-in with-mask" href="{url}"><img src="/poster.jpg"><div class="th-title">{name}</div><div class="th-series">{year}</div></a>"#
            )
        };
        let html = [
            item("/filmy/1.html", "Дюна", 2021),
            item("/filmy/2.html", "Дюна", 1984),
            item("/filmy/3.html", "Дюна: Часть вторая", 2021),
            item("/serialy/4.html", "Дюна", 2021),
            item("https://evil.example/filmy/5.html", "Дюна", 2021),
        ]
        .join("");
        let results = parse(&html, &["Дюна"], Some(2021), false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["url"], format!("{BASE}filmy/1.html"));
        assert!(results[0]["duration"].is_null());
        assert!(parse(&html, &["Дюна"], Some(2019), false).is_empty());
        assert!(series_url(&Url::parse("https://sg.lordfilm5.pro/serialy/1.html").unwrap()));
        assert!(movie_url(&Url::parse("https://sz.lordfilm5.pro/filmy/1.html").unwrap()));
        assert!(!movie_url(&Url::parse("https://sd.lordfilm5.pro.evil.example/filmy/1.html").unwrap()));
        assert!(!movie_url(&Url::parse("https://user@sd.lordfilm5.pro/filmy/1.html").unwrap()));
    }

    #[test]
    fn matches_original_name_and_cards_without_year() {
        let item = |url: &str, name: &str, year: &str| {
            format!(
                r#"<a class="th-in with-mask" href="{url}"><img src="/poster.jpg"><div class="th-title">{name}</div><div class="th-series">{year}</div></a>"#
            )
        };
        let html =
            [item("/filmy/10.html", "Девятый легион", ""), item("/filmy/11.html", "Интерстеллар", "2014")].join("");
        let legion = parse(&html, &["Девятый легион", "The Eagle"], Some(2011), false);
        assert_eq!(legion.len(), 1);
        assert_eq!(legion[0]["url"], format!("{BASE}filmy/10.html"));
        let inter = parse(&html, &["Interstellar", "Интерстеллар"], Some(2014), false);
        assert_eq!(inter.len(), 1);
        assert_eq!(inter[0]["url"], format!("{BASE}filmy/11.html"));
    }

    #[test]
    fn selects_exact_episode_and_rejects_missing_or_blocked() {
        let html = r#"playlist: {seasons:[{"season":1,"episodes":[{"episode":"1","hls":"https://cdn.example/one.m3u8"},{"episode":"2","hls":"https://cdn.example/two.m3u8","duration":1200},{"episode":"3","hls":"https://cdn.example/three.m3u8","blocked":true}]},{"season":2,"blocked":true,"episodes":[{"episode":"1","hls":"https://cdn.example/other.m3u8"}]}]}, other: true"#;
        assert_eq!(episode_media(html, 1, 2), Some(("https://cdn.example/two.m3u8".into(), Some(1200))));
        assert_eq!(episode_numbers(html), vec![(1, 1), (1, 2)]);
        for (s, e) in [(1, 3), (1, 10), (2, 1), (3, 1)] {
            assert!(episode_media(html, s, e).is_none());
        }
        assert!(episode_media("https://cdn.example/random.m3u8", 1, 1).is_none());
        let url = Url::parse(&format!("{BASE}serialy/1.html#nova-season=1&episode=2")).unwrap();
        assert_eq!(episode_target(&url), Some((1, 2)));
        assert!(
            episode_target(&Url::parse(&format!("{BASE}serialy/1.html#nova-season=1&episode=0")).unwrap()).is_none()
        );
        assert!(episode_target(&Url::parse(&format!("{BASE}filmy/1.html#nova-season=1&episode=2")).unwrap()).is_none());
    }

    #[test]
    fn allows_off_by_one_year_and_inflected_words() {
        let item = |url: &str, name: &str, year: i64| {
            format!(
                r#"<a class="th-in with-mask" href="{url}"><div class="th-title">{name}</div><div class="th-series">{year}</div></a>"#
            )
        };
        // Festival/release year can differ by one.
        let html = item("/filmy/20.html", "Девятый легион", 2010);
        let legion = parse(&html, &["Девятый легион", "The Eagle"], Some(2011), false);
        assert_eq!(legion.len(), 1);
        // Russian inflection in one word of a multi-word title.
        let html = item("/filmy/21.html", "Девятого легиона", 2011);
        let legion = parse(&html, &["Девятый легион"], Some(2011), false);
        assert_eq!(legion.len(), 1);
    }
}
