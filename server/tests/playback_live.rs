//! Живая проверка воспроизведения: не только поиск, но и настоящие байты видео.
//! Запуск: cargo test -p fsfy-server --test playback_live live_playback_everywhere -- --ignored --nocapture
//!
//! Диагностика выходит из Стокгольма: плееры plplayer.online / horsez.org режут этот адрес по
//! региону. Их варианты всё равно ищутся и добавляются - ошибку «подключитесь из России» видит
//! только эта машина, у зрителя с хоста в РФ поток открывается.
mod common;

use reqwest::StatusCode;
use serde_json::{Value, json};

struct Fetched {
    code: StatusCode,
    body: Vec<u8>,
}

type Case = (&'static str, &'static str, Option<&'static str>, &'static str, Option<i64>, Option<i64>, usize);

fn path_of(base: &str, child: &str) -> String {
    if child.starts_with('/') {
        return child.to_string();
    }
    let root = reqwest::Url::parse(&format!("http://localhost{base}")).unwrap();
    let joined = root.join(child).unwrap();
    let mut out = joined.path().to_string();
    if let Some(q) = joined.query() {
        out.push('?');
        out.push_str(q);
    }
    out
}

async fn get(host: &common::Client, path: &str, range: Option<&str>) -> Fetched {
    let mut req = host.http.get(format!("{}{path}", host.base)).header("cookie", &host.cookie);
    if let Some(r) = range {
        req = req.header("range", r);
    }
    let res = req.send().await.unwrap();
    let code = res.status();
    let body = res.bytes().await.unwrap_or_default().to_vec();
    Fetched { code, body }
}

fn first_media(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))
}

fn last_media(text: &str) -> Option<&str> {
    text.lines().map(str::trim).rfind(|l| !l.is_empty() && !l.starts_with('#'))
}

async fn probe_hls(host: &common::Client, path: &str) -> (bool, String) {
    let index = get(host, path, None).await;
    if !index.code.is_success() || index.body.is_empty() {
        return (false, format!("плейлист {} ({} байт)", index.code, index.body.len()));
    }
    let text = String::from_utf8_lossy(&index.body);
    let Some(child) = first_media(&text) else { return (false, "пустой главный плейлист".into()) };
    let child_path = path_of(path, child);
    let media = get(host, &child_path, None).await;
    let media_text = String::from_utf8_lossy(&media.body);
    let segments: Vec<&str> = if media_text.contains("#EXTM3U") {
        [first_media(&media_text), last_media(&media_text)].into_iter().flatten().collect()
    } else {
        vec![child]
    };
    let mut note = String::new();
    for (n, segment) in segments.iter().enumerate() {
        let seg_path = path_of(&child_path, segment);
        let r = get(host, &seg_path, Some("bytes=0-1023")).await;
        note.push_str(&format!("сегмент {n}: {} {} байт; ", r.code, r.body.len()));
        if !r.code.is_success() || r.body.len() < 128 {
            return (false, note);
        }
    }
    (true, note)
}

async fn probe_dash(host: &common::Client, path: &str) -> (bool, String) {
    let index = get(host, path, None).await;
    if !index.code.is_success() || index.body.is_empty() {
        return (false, format!("MPD {} ({} байт)", index.code, index.body.len()));
    }
    let text = String::from_utf8_lossy(&index.body);
    let segment = text
        .split_once("initialization=\"")
        .and_then(|(_, rest)| rest.split('"').next())
        .unwrap_or("")
        .replace("$RepresentationID$", "0")
        .replace("$Number$", "0")
        .replace("$Time$", "0")
        .replace("$Bandwidth$", "0");
    if segment.is_empty() {
        return (true, "только MPD, без сегмента".into());
    }
    // DASH-клиент берёт относительные имена сегментов из <BaseURL>, а не из адреса самого MPD:
    // сервер отдаёт плейлист с одним адресом (/stream/index.mpd), а BaseURL ведёт на прокси CDN.
    let base = text
        .split_once("<BaseURL>")
        .and_then(|(_, rest)| rest.split('<').next())
        .filter(|s| !s.is_empty())
        .unwrap_or(path);
    let seg_path = path_of(base, &segment);
    let r = get(host, &seg_path, Some("bytes=0-1023")).await;
    let ok = r.code.is_success() && r.body.len() >= 128;
    (ok, format!("сегмент {} {} байт", r.code, r.body.len()))
}

async fn probe(
    host: &mut common::Client,
    title: &str,
    found: &[Value],
    ext: Option<&str>,
    season: Option<i64>,
    episode: Option<i64>,
    domain: &str,
) -> (usize, usize) {
    let mut ok = 0;
    for item in found {
        let mut add = json!({"url":item["url"],"label":item["label"],"sourceDomain":domain});
        if let Some(ext) = ext {
            add["extId"] = json!(ext);
        }
        if let Some(s) = season {
            add["season"] = json!(s);
        }
        if let Some(e) = episode {
            add["episode"] = json!(e);
        }
        let r = host.send("POST", "/api/links", add).await;
        if !r.status().is_success() {
            println!(
                "  FAIL {title} [{}] добавление {}: {}",
                item["label"],
                r.status(),
                r.text().await.unwrap_or_default()
            );
            continue;
        }
        let id = r.json::<Value>().await.unwrap()["id"].as_i64().unwrap();
        let src = host.get(&format!("/api/links/{id}/stream/source")).await;
        let st = src.status();
        let body = src.text().await.unwrap_or_default();
        if !st.is_success() {
            println!("  FAIL {title} [{}] stream/source {}: {}", item["label"], st, body);
            continue;
        }
        let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
        let path = v["url"].as_str().unwrap_or("").to_string();
        let (good, note) = if v["hls"] == true {
            probe_hls(host, &path).await
        } else if v["dash"] == true {
            probe_dash(host, &path).await
        } else {
            let r = get(host, &path, Some("bytes=0-1023")).await;
            let good = r.code.is_success() && r.body.len() >= 128;
            (good, format!("{} {} байт", r.code, r.body.len()))
        };
        if good {
            ok += 1;
        }
        println!("  {} {title} [{}] {note}", if good { "OK  " } else { "FAIL" }, item["label"]);
    }
    (ok, found.len())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живая проверка настоящего видео по всем приложенным источникам"]
async fn live_playback_everywhere() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    for (id, kind, title, year) in [
        ("tt9458304", "series", "Дороро", 2019),
        ("tt0141842", "series", "Клан Сопрано", 1999),
        ("tt0108778", "series", "Друзья", 1994),
        ("tt0460681", "series", "Сверхъестественное", 2005),
        ("tt0133093", "movie", "Матрица", 1999),
    ] {
        running
            .app
            .db
            .with(|c| {
                c.execute(
                    "INSERT OR IGNORE INTO ext_titles(id,kind,title,year,updated_at) VALUES(?1,?2,?3,?4,0)",
                    rusqlite::params![id, kind, title, year],
                )
            })
            .unwrap();
    }
    // Название «Сопрано» в title_localizations специально: раньше из-за него сайт одного тайтла
    // не распознавался и серии Сопрано вообще не находились.
    running
        .app
        .db
        .with(|c| c.execute("INSERT OR IGNORE INTO title_localizations(id,title) VALUES('tt0141842','Сопрано')", []))
        .unwrap();
    let cases: Vec<Case> = vec![
        (
            "old.yummyani.me",
            "Дороро",
            Some("tt9458304"),
            "https://old.yummyani.me/search?word={q}",
            Some(1),
            Some(1),
            36,
        ),
        (
            "watch-the-sopranos-online.net",
            "Клан Сопрано",
            Some("tt0141842"),
            "https://www.watch-the-sopranos-online.net/?s={q}",
            Some(1),
            Some(1),
            4,
        ),
        ("serialfriends.org", "Друзья", Some("tt0108778"), "https://serialfriends.org/?s={q}", Some(1), Some(1), 4),
        (
            "supernatural-spn.net",
            "Сверхъестественное",
            Some("tt0460681"),
            "https://supernatural-spn.net/?s={q}",
            Some(1),
            Some(1),
            4,
        ),
        (
            "lordserials.fan",
            "Друзья",
            Some("tt0108778"),
            "https://lordserials.fan/index.php?do=search&subaction=search&story={q}",
            Some(1),
            Some(1),
            4,
        ),
        (
            "sx.lordfilm5.pro",
            "Матрица",
            Some("tt0133093"),
            "https://sx.lordfilm5.pro/index.php?do=search&subaction=search&story={q}",
            None,
            None,
            3,
        ),
        ("sr.lordfilm5.pro", "Матрица", Some("tt0133093"), "https://sr.lordfilm5.pro/filmy/?s={q}", None, None, 3),
    ];
    let catalogs: Vec<Value> = cases
        .iter()
        .map(|(domain, name, _, search, _, _, _)| json!({"domain":domain,"name":name,"kind":"all","search":search,"enabled":true}))
        .collect();
    let r = host.send("PUT", "/api/admin/catalogs", json!({"items":catalogs})).await;
    assert!(r.status().is_success(), "catalogs: {}", r.status());
    let mut overall = (0, 0);
    for (domain, q, ext, _search, season, episode, limit) in cases {
        let encoded: String = url::form_urlencoded::byte_serialize(q.as_bytes()).collect();
        let mut path = format!("/api/links/search?q={encoded}&provider=own");
        if let Some(e) = ext {
            path.push_str(&format!("&extId={e}"));
        }
        if let Some(s) = season {
            path.push_str(&format!("&season={s}&episode={}", episode.unwrap()));
        }
        let text = host.get(&path).await.text().await.unwrap_or_default();
        let found: Vec<Value> = serde_json::from_str(&text).unwrap_or_default();
        println!("=== {domain}: найдено {}", found.len());
        assert!(!found.is_empty(), "{domain}: поиск не нашёл ни одной ссылки: {text}");
        let probe_items: Vec<Value> = found.into_iter().take(limit).collect();
        let (ok, total) = probe(&mut host, domain, &probe_items, ext, season, episode, domain).await;
        println!("--- {domain}: играет {ok}/{total}");
        overall.0 += ok;
        overall.1 += total;
    }
    println!("ИТОГО играет {}/{}", overall.0, overall.1);
    running.shutdown().await;
}
