//! Каталоги хоста: сайты, где много фильмов, серий-франшиз и сериалов. Хост добавляет сайт, Nova ищет на нём
//! фильм обычным поиском самого сайта (адрес с `{q}`) и отдаёт найденные страницы как ссылки; видео со страницы
//! открывается в плеере Nova так же, как у любой ссылки со «Своих сайтов» (`api::stream_link`).
use std::sync::OnceLock;

use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{video_search::any_name_matches, App};

const KEY: &str = "video.catalogs";
pub const MAX: usize = 20;

/// Сайт хоста и адрес его поиска (`https://сайт/?s={q}`); без адреса поиска сайт только разрешён для ссылок.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Catalog {
    pub domain: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "all_kinds")]
    pub kind: String,
    #[serde(default)]
    pub manifest: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

fn all_kinds() -> String {
    "all".into()
}

pub fn enabled_domains(app: &App) -> Vec<String> {
    load(app).into_iter().filter(|c| c.enabled).map(|c| c.domain).collect()
}

/// Проверка также действует для уже открытых ссылок после отключения источника.
pub fn link_enabled(app: &App, id: i64) -> crate::error::ApiResult<()> {
    use rusqlite::OptionalExtension;
    let domain: Option<String> = app
        .db
        .with(|c| c.query_row("SELECT domain FROM catalog_links WHERE link_id = ?", [id], |r| r.get(0)).optional())?;
    if domain.is_some_and(|d| !enabled_domains(app).contains(&d)) {
        return Err(crate::error::ApiError::bad("source_disabled", "Источник выключен хостом - выберите другой"));
    }
    Ok(())
}

fn enabled_by_default() -> bool {
    true
}

pub fn load(app: &App) -> Vec<Catalog> {
    serde_json::from_str(&app.db.setting_or(KEY, "[]")).unwrap_or_default()
}

pub fn store(app: &App, list: &[Catalog]) -> rusqlite::Result<()> {
    app.db.set_setting(KEY, &serde_json::to_string(list).unwrap_or_else(|_| "[]".into()))
}

fn same_site(host: &str, domain: &str) -> bool {
    let host = host.trim_start_matches("www.").to_lowercase();
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// Подставить запрос в адрес поиска хоста. Адрес должен вести на тот же сайт (иначе Nova сходит куда угодно).
pub fn search_url(domain: &str, template: &str, query: &str) -> Option<Url> {
    if !template.contains("{q}") {
        return None;
    }
    let encoded = Url::parse_with_params("http://h/", &[("q", query)]).ok()?.query()?.strip_prefix("q=")?.to_string();
    let url = Url::parse(&template.replace("{q}", &encoded)).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some_and(|h| same_site(h, domain))).then_some(url)
}

/// Проверка адреса, который хост вписал сам.
pub fn valid_template(domain: &str, template: &str) -> bool {
    search_url(domain, template, "x").is_some()
}

/// Распространённые адреса поиска: WordPress, DLE и похожие.
pub fn candidates(domain: &str) -> Vec<String> {
    vec![
        format!("https://{domain}/?s={{q}}"),
        format!("https://{domain}/index.php?do=search&subaction=search&story={{q}}"),
        format!("https://{domain}/search/?q={{q}}"),
        format!("https://{domain}/search?q={{q}}"),
        format!("https://{domain}/?q={{q}}"),
        format!("https://{domain}/search/{{q}}"),
    ]
}

fn re(cell: &'static OnceLock<regex::Regex>, pattern: &str) -> &'static regex::Regex {
    cell.get_or_init(|| regex::Regex::new(pattern).expect("static pattern"))
}

fn decode(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#039;", "'").replace("&#39;", "'").replace("&nbsp;", " ").replace("&amp;", "&")
}

fn plain(html: &str) -> String {
    static TAGS: OnceLock<regex::Regex> = OnceLock::new();
    let text = re(&TAGS, r"(?s)<[^>]*>").replace_all(html, " ");
    decode(&text).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Страницы сайта со страницы результатов: ссылки того же сайта, чья подпись (текст, `title`, `alt` картинки)
/// подходит под одно из названий. Год ищется и в подписи, и в самом адресе (`…/duna-2021.html`).
pub fn extract(html: &str, base: &Url, domain: &str, names: &[&str], year: Option<i64>) -> Vec<Value> {
    static ANCHOR: OnceLock<regex::Regex> = OnceLock::new();
    static HREF: OnceLock<regex::Regex> = OnceLock::new();
    static TITLE: OnceLock<regex::Regex> = OnceLock::new();
    static IMG: OnceLock<regex::Regex> = OnceLock::new();
    static ALT: OnceLock<regex::Regex> = OnceLock::new();
    let anchor = re(&ANCHOR, r#"(?is)<a\b([^>]*)>(.*?)</a>"#);
    let href = re(&HREF, r#"(?is)\bhref\s*=\s*["']([^"']+)["']"#);
    let title = re(&TITLE, r#"(?is)\btitle\s*=\s*["']([^"']+)["']"#);
    let img = re(&IMG, r#"(?is)<img\b[^>]*?\b(?:data-src|data-original|src)\s*=\s*["']([^"']+)["']"#);
    let alt = re(&ALT, r#"(?is)\balt\s*=\s*["']([^"']+)["']"#);
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in anchor.captures_iter(html) {
        let attrs = &c[1];
        let body = &c[2];
        let Some(h) = href.captures(attrs) else { continue };
        let Some(mut url) = base.join(&decode(&h[1])).ok() else { continue };
        url.set_fragment(None);
        if !matches!(url.scheme(), "http" | "https")
            || !url.host_str().is_some_and(|h| same_site(h, domain))
            || url.path().len() < 2
            || !seen.insert(url.to_string())
        {
            continue;
        }
        let mut labels = Vec::new();
        labels.extend(title.captures(attrs).map(|t| plain(&t[1])));
        labels.extend(alt.captures(body).map(|a| plain(&a[1])));
        labels.push(plain(body));
        let path = url.path().replace(['-', '_', '/', '.'], " ");
        let Some(label) = labels
            .into_iter()
            .filter(|l| !l.is_empty())
            .find(|l| any_name_matches(&format!("{l} {path}"), names, None, None, year, false))
        else {
            continue;
        };
        let thumb = img
            .captures(body)
            .and_then(|i| base.join(&decode(&i[1])).ok())
            .filter(crate::outbound::valid_url)
            .map(|u| u.to_string());
        // Год может присутствовать только в URL карточки. Сохраняем его в подписи,
        // чтобы последующая проверка результата не отбросила уже подтверждённое совпадение.
        let label = if let Some(year) = year.filter(|year| path.split_whitespace().any(|p| p == year.to_string())) {
            if !label.contains(&year.to_string()) {
                format!("{label} ({year})")
            } else {
                label
            }
        } else {
            label
        };
        out.push(json!({
            "site": "own", "url": url.as_str(), "label": label.chars().take(200).collect::<String>(),
            "thumbUrl": thumb, "duration": null, "author": domain,
        }));
        if out.len() >= 8 {
            break;
        }
    }
    out
}

/// Поиск на сайте хоста: по первому названию, а если ничего - по второму (обычно оригинальному).
pub async fn search(c: &Catalog, names: &[&str], year: Option<i64>) -> Option<Vec<Value>> {
    let template = c.search.as_deref()?;
    let mut answered = false;
    for name in names.iter().filter(|n| !n.trim().is_empty()).take(2) {
        let url = search_url(&c.domain, template, name)?;
        let Ok(html) = crate::api::stream_link::page(url.as_str()).await else { continue };
        answered = true;
        let found = extract(&html, &url, &c.domain, names, year);
        if !found.is_empty() {
            return Some(found);
        }
    }
    answered.then(Vec::new)
}

/// Поиск серии на подключённом сайте: показываем только явно найденный сезон и эпизод.
pub async fn search_episode(
    app: &App,
    c: &Catalog,
    names: &[&str],
    year: Option<i64>,
    season: i64,
    episode: i64,
) -> Option<Vec<Value>> {
    // Карточки некоторых сериалов не указывают год в результатах. Пропускаем только точное
    // название и проверяем год на самой странице до разбора плеера.
    let pages = search(c, names, None).await?;
    let mut result = Vec::new();
    for page in pages.into_iter().take(3) {
        let Some(url) = page["url"].as_str() else { continue };
        let label = page["label"].as_str().unwrap_or("");
        if !names.iter().any(|n| crate::util::norm(n) == crate::util::norm(label)) {
            continue;
        }
        if let Some(year) = year {
            let Ok((_, html)) = crate::api::stream_link::page_for(Some(app), url).await else { continue };
            let pattern =
                format!(r#"(?is)itemprop=["'](?:dateCreated|datePublished)["'][^>]*>(?:\s|<[^>]*>)*{year}\b"#);
            if !regex::Regex::new(&pattern).ok().is_some_and(|r| r.is_match(&html)) {
                continue;
            }
        }
        let entry = crate::site_sources::Entry {
            url: url.to_owned(),
            title: names.first().copied().unwrap_or("").to_owned(),
            kind: "series".into(),
            year: year.and_then(|y| i32::try_from(y).ok()),
            imdb: None,
            remote_id: None,
            season: None,
            episode: None,
        };
        let Ok((variants, _)) = crate::site_sources::variants(&entry, Some(app)).await else { continue };
        for variant in variants.into_iter().filter(|v| v.season == Some(season) && v.episode == Some(episode)) {
            let Ok(url) = Url::parse(&variant.url) else { continue };
            if !crate::site_sources::is_provider(&url) {
                continue;
            }
            let y = year.map(|v| format!(" ({v})")).unwrap_or_default();
            result.push(json!({"site":"own","url":variant.url,"label":format!("{}{y} {season} сезон {episode} серия · {}",entry.title,variant.label),"thumbUrl":page["thumbUrl"],"duration":null,"author":c.domain}));
        }
    }
    Some(result)
}

/// Какой из обычных адресов поиска у сайта работает: тот, чья страница откликается на известное название.
pub async fn probe(domain: &str) -> Option<String> {
    let tries = candidates(domain).into_iter().map(|template| async move {
        let url = search_url(domain, &template, "Матрица")?;
        let html =
            tokio::time::timeout(std::time::Duration::from_secs(10), crate::api::stream_link::page(url.as_str()))
                .await
                .ok()?
                .ok()?;
        let echoed = crate::util::norm(&plain(&html)).contains("матриц");
        (echoed && html.matches("<a ").count() > 3).then_some(template)
    });
    futures_util::future::join_all(tries).await.into_iter().flatten().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_address_stays_on_the_site() {
        let u = search_url("kino.example", "https://kino.example/?s={q}", "Начало 2").unwrap();
        assert_eq!(u.as_str(), "https://kino.example/?s=%D0%9D%D0%B0%D1%87%D0%B0%D0%BB%D0%BE+2");
        assert!(search_url("kino.example", "https://www.kino.example/?s={q}", "x").is_some());
        assert!(search_url("kino.example", "https://evil.example/?s={q}", "x").is_none(), "чужой сайт");
        assert!(search_url("kino.example", "https://kino.example.evil.example/?s={q}", "x").is_none());
        assert!(search_url("kino.example", "https://kino.example/?s=", "x").is_none(), "нет {{q}}");
        assert!(search_url("kino.example", "ftp://kino.example/{q}", "x").is_none());
        assert!(valid_template("kino.example", "https://kino.example/search/{q}"));
        assert!(candidates("kino.example").iter().all(|t| valid_template("kino.example", t)));
    }

    #[test]
    fn extracts_matching_pages_of_the_same_site() {
        let html = r#"
          <a href="/">Главная</a>
          <a href="/films/101-duna-2021.html" title="Дюна (2021) смотреть онлайн"><img data-src="/up/duna.jpg" alt="Дюна"></a>
          <a href="/films/102-duna-chast-vtoraya-2024.html">Дюна: Часть вторая</a>
          <a href="https://other.example/films/9-duna.html">Дюна у соседей</a>
          <a href="/films/101-duna-2021.html#comments">Дюна (2021)</a>
          <a href="/serials/55-prorochestvo.html">Дюна: Пророчество</a>
          <a href="/about">О сайте</a>"#;
        let base = Url::parse("https://kino.example/search/?q=x").unwrap();
        let found = extract(html, &base, "kino.example", &["Дюна"], Some(2021));
        let urls: Vec<&str> = found.iter().map(|v| v["url"].as_str().unwrap()).collect();
        assert_eq!(urls, vec!["https://kino.example/films/101-duna-2021.html"], "{found:?}");
        assert_eq!(found[0]["thumbUrl"], "https://kino.example/up/duna.jpg");
        assert_eq!(found[0]["site"], "own");
        assert_eq!(found[0]["author"], "kino.example");
        // Без года (сериал): подходят все страницы того же сайта с названием.
        let all = extract(html, &base, "kino.example", &["Дюна"], None);
        assert_eq!(all.len(), 3, "{all:?}");
    }
}
