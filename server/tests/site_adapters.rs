mod common;
use fsfy::site_sources;
use reqwest::Url;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живой поиск серии на lordserials.fan"]
async fn live_manual_lordserials_episode() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO ext_titles(id,kind,title,year,updated_at) VALUES('tt0108778','series','Друзья',1994,0)",
                [],
            )
        })
        .unwrap();
    let response=host.send("PUT","/api/admin/catalogs",json!({"items":[{"domain":"lordserials.fan","name":"Lordserials","kind":"series","search":"https://lordserials.fan/index.php?do=search&subaction=search&story={q}","enabled":true}]})).await;
    assert!(response.status().is_success());
    let query: String = url::form_urlencoded::byte_serialize("Друзья".as_bytes()).collect();
    let results =
        host.json(&format!("/api/links/search?q={query}&extId=tt0108778&provider=own&season=1&episode=1")).await;
    let first = results.as_array().expect("массив").first().expect("доступная первая серия");
    let response=host.send("POST","/api/links",json!({"url":first["url"],"label":first["label"],"extId":"tt0108778","season":1,"episode":1,"sourceDomain":"lordserials.fan"})).await;
    assert!(response.status().is_success(), "{}", response.status());
    let id = response.json::<serde_json::Value>().await.unwrap()["id"].as_i64().unwrap();
    let response = host.get(&format!("/api/links/{id}/stream/source")).await;
    assert!(response.status().is_success(), "поток: {}", response.status());
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живой поиск на mg.lordfilm.md без импорта"]
async fn live_manual_lordfilm_on_demand() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO ext_titles(id,kind,title,year,updated_at) VALUES('tt0133093','movie','Матрица',1999,0)",
                [],
            )
        })
        .unwrap();
    let response=host.send("PUT","/api/admin/catalogs",json!({"items":[{"domain":"mg.lordfilm.md","name":"LordFilm","kind":"movie","search":"https://mg.lordfilm.md/index.php?do=search&subaction=search&story={q}","enabled":true}]})).await;
    assert!(response.status().is_success());
    let query: String = url::form_urlencoded::byte_serialize("Матрица".as_bytes()).collect();
    let results = host.json(&format!("/api/links/search?q={query}&extId=tt0133093&provider=own")).await;
    let first = results.as_array().expect("массив").first().expect("найден фильм");
    let response = host
        .send(
            "POST",
            "/api/links",
            json!({"url":first["url"],"label":first["label"],"extId":"tt0133093","sourceDomain":"mg.lordfilm.md"}),
        )
        .await;
    assert!(response.status().is_success(), "{}", response.status());
    let id = response.json::<serde_json::Value>().await.unwrap()["id"].as_i64().unwrap();
    let response = host.get(&format!("/api/links/{id}/stream/source")).await;
    assert!(response.status().is_success(), "поток: {}", response.status());
    let count: i64 =
        running.app.db.with(|c| c.query_row("SELECT COUNT(*) FROM site_import_entries", [], |r| r.get(0))).unwrap();
    assert_eq!(count, 0);
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живые изображения старых карточек источников"]
async fn live_source_posters_for_unmatched_cards() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    let poster = running.app.images.save(b"\x89PNG\r\n\x1a\nfixture").unwrap();
    running.app.db.with(|c|c.execute("INSERT INTO ext_titles(id,kind,title,year,poster,updated_at) VALUES('tt0120737','movie','Властелин колец: Братство кольца',2001,?1,0)",[poster])).unwrap();
    for (domain, url, title, kind, remote) in [
        (
            "sr.lordfilm5.pro",
            "https://ss.lordfilm5.pro/filmy/29439-vlastelin-kolets-bratstvo-koltsa-2001.html",
            "Властелин колец: Братство кольца",
            "movie",
            None,
        ),
        ("old.yummyani.me", "https://old.yummyani.me/catalog/item/dororo", "Дороро", "series", Some(258)),
    ] {
        let id = format!("nv{}", fsfy::util::sha256_hex(format!("{domain}:{url}").as_bytes()));
        let entry = json!({"url":url,"title":title,"kind":kind,"year":2020,"imdb":null,"remote_id":remote,"season":null,"episode":null});
        running
            .app
            .db
            .tx(|c| {
                c.execute(
                    "INSERT INTO ext_titles(id,kind,title,updated_at) VALUES(?1,?2,?3,0)",
                    rusqlite::params![id, kind, title],
                )?;
                c.execute(
                    "INSERT INTO site_import_entries(domain,url,metadata,updated_at) VALUES(?1,?2,?3,0)",
                    rusqlite::params![domain, url, entry.to_string()],
                )?;
                Ok(())
            })
            .unwrap();
        let mut response = host.get(&format!("/api/discover/poster/{id}")).await;
        for _ in 0..2 {
            if !response.status().is_redirection() {
                break;
            }
            let location = response.headers().get("location").unwrap().to_str().unwrap().to_owned();
            response = host.get(&location).await;
        }
        assert!(response.status().is_success(), "{title}: {}", response.status());
        assert!(!response.bytes().await.unwrap().is_empty(), "{title}");
    }
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живой поиск конкретной серии YummyAnime без импорта"]
async fn live_yummy_on_demand_without_import() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO ext_titles(id,kind,title,year,updated_at) VALUES('tt9458304','series','Дороро',2019,0)",
                [],
            )
        })
        .unwrap();
    let r = host
        .send(
            "PUT",
            "/api/admin/catalogs",
            json!({"items":[{"domain":"old.yummyani.me","name":"YummyAnime","kind":"anime","enabled":true}]}),
        )
        .await;
    assert!(r.status().is_success());
    let q: String = url::form_urlencoded::byte_serialize("Дороро".as_bytes()).collect();
    let results = host.json(&format!("/api/links/search?q={q}&extId=tt9458304&provider=own&season=1&episode=18")).await;
    let items = results.as_array().expect("массив результатов");
    let playable = items
        .iter()
        .find(|v| v["url"].as_str().is_some_and(|u| u.contains("player.aksor.tv")))
        .expect("Aksor для серии 18");
    let link = host
        .send(
            "POST",
            "/api/links",
            json!({"url":playable["url"],"label":playable["label"],"extId":"tt9458304","season":1,"episode":18,"sourceDomain":"old.yummyani.me"}),
        )
        .await;
    assert!(link.status().is_success(), "{}", link.status());
    let id = link.json::<serde_json::Value>().await.unwrap()["id"].as_i64().unwrap();
    let source = host.json(&format!("/api/links/{id}/stream/source")).await;
    assert_eq!(source["dash"], true);
    let manifest = host.get(source["url"].as_str().unwrap()).await.error_for_status().unwrap().text().await.unwrap();
    let field = |pattern: &str| regex::Regex::new(pattern).unwrap().captures(&manifest).unwrap()[1].to_owned();
    let base = field(r"<BaseURL>([^<]+)</BaseURL>");
    let repr = field(r#"<Representation\s+id="([^"]+)""#);
    let first = field(r#"media="([^"]+)""#).replace("$RepresentationID$", &repr).replace("$Number%05d$", "00001");
    let segment = host.get(&format!("{base}{first}")).await.error_for_status().unwrap();
    assert!(!segment.bytes().await.unwrap().is_empty());
    let imported: i64 =
        running.app.db.with(|c| c.query_row("SELECT COUNT(*) FROM site_import_entries", [], |r| r.get(0))).unwrap();
    assert_eq!(imported, 0);
    let response = host.send("PUT", "/api/admin/catalogs", json!({"items":[]})).await;
    assert!(response.status().is_success());
    assert!(
        !host.get(&format!("/api/links/{id}/stream/source")).await.status().is_success(),
        "удалённый источник не должен продолжать выдавать поток"
    );
    running.shutdown().await;
}

#[test]
fn parses_real_provider_shapes_without_executing_javascript() {
    let movie = r#"new Player({source:{hls:"https://cdn.example.com/master.m3u8",audio:{"names":["Русский","English"],"order":[0,2]},cc:[{"url":"https://cdn.example.com/en.vtt","name":"English CC"}],duration:9000},ad:()=>{throw 'do not run'}})"#;
    let base = Url::parse("https://api.nextembed.ws/embed/movie/383").unwrap();
    let found = site_sources::structured(movie, &base);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.audio, vec!["Русский", "", "English"]);
    assert_eq!(found[0].1.subtitles.len(), 1);
    let master = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"rus0\",URI=\"a0.m3u8\"\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"rus1\",URI=\"a1.m3u8\"\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"eng2\",URI=\"a2.m3u8\"\n#EXT-X-STREAM-INF:RESOLUTION=3840x2160,AUDIO=\"a\"\n4k.m3u8";
    let enriched = site_sources::hls_manifest(master, &found[0].1, 7);
    assert!(enriched.contains("NAME=\"Русский\""));
    assert!(enriched.contains("NAME=\"English\""));
    assert!(enriched.contains("NAME=\"rus1\""));
    assert!(enriched.contains("URI=\"/api/links/7/stream/subtitle/0.m3u8\""));
    assert!(enriched.contains("SUBTITLES=\"nova-subtitles\""));
    assert!(enriched.contains("4k.m3u8"));
    let episodes = r#"seasons:[{"season":0,"episodes":[{"episode":1,"hls":"https://cdn.example.com/special.m3u8"}]},{"season":1,"episodes":[{"episode":"1","hls":"https://cdn.example.com/one.m3u8"},{"episode":"2","hls":"https://cdn.example.com/two.m3u8"},{"episode":3,"blocked":true,"hls":"https://cdn.example.com/blocked.m3u8"}]}]"#;
    let found = site_sources::structured(episodes, &Url::parse("https://api.namy.ws/embed/movie/57493").unwrap());
    assert_eq!(found.len(), 3);
    assert_eq!(found[0].0.season, Some(0));
    assert_eq!(found[2].0.episode, Some(2));
    assert!(found[2].0.url.ends_with("#nova-season=1&episode=2"));
    let other = site_sources::structured(episodes, &Url::parse("https://other.example/embed/movie/57493").unwrap());
    assert_ne!(found[0].0.id, other[0].0.id, "Одинаковые пути разных плееров не заменяют варианты друг друга");
}

#[test]
fn dash_only_and_audio_labels_without_numeric_names() {
    let dash = r#"new Player({source:{dash:"https://cdn.example.com/manifest.mpd",audio:{"names":["Русский","English"],"order":[0,2]},duration:5400}})"#;
    let found = site_sources::structured(dash, &Url::parse("https://player.example.com/embed/1").unwrap());
    assert_eq!(found.len(), 1, "источник только с DASH должен попадать в каталог");
    assert_eq!(found[0].1.url, "https://cdn.example.com/manifest.mpd");
    let master = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"Русский\",URI=\"a0.m3u8\"\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"Original\",URI=\"a1.m3u8\"\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"English\",URI=\"a2.m3u8\"\n#EXT-X-STREAM-INF:RESOLUTION=1280x720,AUDIO=\"a\"\n720.m3u8";
    let enriched = site_sources::hls_manifest(master, &found[0].1, 9);
    assert!(enriched.contains("NAME=\"Русский\""), "первая дорожка: {enriched}");
    assert!(enriched.contains("NAME=\"Original\""), "вторая дорожка без подписи не трогается: {enriched}");
    assert!(enriched.contains("NAME=\"English\""), "третья дорожка: {enriched}");
}

#[test]
fn original_audio_names_survive_and_repeats_get_numbers() {
    let master = "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"Русский (Studio)\",URI=\"a0.m3u8\"\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"Русский (Studio)\",URI=\"a1.m3u8\"\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"rus0\",URI=\"a2.m3u8\"\n#EXT-X-STREAM-INF:RESOLUTION=1280x720,AUDIO=\"a\"\n720.m3u8";
    let media = site_sources::Media {
        url: "https://cdn.example.com/master.m3u8".into(),
        audio: vec!["Голос".into()],
        subtitles: vec![],
        duration: 0.0,
    };
    let enriched = site_sources::hls_manifest(master, &media, 11);
    assert!(enriched.contains("NAME=\"Русский (Studio)\""), "название озвучки из источника: {enriched}");
    assert!(enriched.contains("NAME=\"Русский (Studio) 2\""), "повтор должен отличаться: {enriched}");
    assert!(enriched.contains("NAME=\"Голос\""), "позиционный псевдоним заменяется: {enriched}");
}

#[test]
fn catalog_pages_stay_on_source_and_preserve_all_entries() {
    let source = site_sources::by_id("lordfilm").unwrap();
    let base = Url::parse(source.url).unwrap();
    let html = r#"<a class="th-in" href="/filmy/10-film-2020.html"><div class="th-title">Фильм &amp; фильм</div><div class="th-series">2020</div></a><a class="th-in" href="/filmy/11-film-2021.html"><div class="th-title">Другой фильм</div></a><a class="th-in" href="https://evil.example/filmy/12.html"><div class="th-title">Чужой сайт</div></a><a href="/filmy/page/635/">635</a><a href="/filmy/page/2/">Далее</a>"#;
    let entries = site_sources::listing(html, &base, source);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].title, "Фильм & фильм");
    assert_eq!(site_sources::next_page(html, &base).as_deref(), Some("https://sr.lordfilm5.pro/filmy/page/2/"));
    assert!(site_sources::check_page("<title>Подтвердите, что вы человек:</title>").is_err());
    assert_eq!(site_sources::SOURCES.len(), 9);
    assert!(site_sources::by_domain("lordserials.fan.evil.example").is_none());
    assert!(site_sources::by_domain("www.watch-the-sopranos-online.net").is_some());
    let ns = format!("nv{}", "a".repeat(64));
    assert!(fsfy::discover::valid_id(&ns));
    assert!(!fsfy::discover::imdb_id(&ns));
}

#[tokio::test(flavor = "multi_thread")]
async fn sources_require_host_opt_in_and_do_not_expose_cookies() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let sources = host.json("/api/admin/site-sources").await;
    let items = sources["items"].as_array().unwrap();
    assert_eq!(items.len(), 9);
    // Каталоги из миграции 35 включены при установке. «bear» среди них нет - именно его
    // подключение по запросу хоста и проверяем. Остальные источники остались без сеансов.
    let bear = items.iter().find(|s| s["source"]["id"] == "bear").unwrap();
    assert_eq!(bear["configured"], false);
    assert_eq!(items.iter().filter(|s| s["configured"] == true).count(), 6);
    assert_eq!(host.send("POST", "/api/admin/site-sources/bear/import", json!({})).await.status(), 400);
    assert!(host.send("POST", "/api/admin/site-sources/bear/connect", json!({})).await.status().is_success());
    assert!(host.send("POST", "/api/admin/site-sources/bear/connect", json!({})).await.status().is_success());
    let catalogs = host.json("/api/admin/catalogs").await;
    assert_eq!(catalogs["items"].as_array().unwrap().len(), 8);
    running.app.source_sessions.lock().insert("bear".into(), vec![("antibot".into(), "opaque-secret-value".into())]);
    let sources = host.json("/api/admin/site-sources").await;
    assert!(!sources.to_string().contains("opaque-secret-value"));
    assert!(
        host.send("POST", "/api/admin/site-sources/bear/verify", json!({"action":"forget"}))
            .await
            .status()
            .is_success()
    );
    assert!(running.app.source_sessions.lock().is_empty());
    let mut viewer = common::Client::new(format!("http://127.0.0.1:{}", running.app.ports.0));
    assert!(
        viewer
            .send(
                "POST",
                "/api/auth/register",
                json!({"name":"Зритель","login":"sites-viewer","password":"пароль-12345"})
            )
            .await
            .status()
            .is_success()
    );
    for route in ["connect", "import", "stop", "verify", "match"] {
        assert_eq!(
            viewer
                .send(
                    "POST",
                    &format!("/api/admin/site-sources/bear/{route}"),
                    json!({"action":"use","url":"x","imdbId":"tt1234567"})
                )
                .await
                .status(),
            403
        );
    }
    running.shutdown().await;
}

#[test]
fn episodes_use_observed_urls_and_explicit_numbers() {
    let source = site_sources::by_id("supernatural").unwrap();
    let base = Url::parse("https://supernatural-spn.net/seasons/1-sezon/").unwrap();
    let html = r#"<a href="/episodes/1-sezon-1-seria/">Первая</a><a href="/episodes/2-sezon-3-seriya/">Другая</a><a href="https://other.example/episodes/9-sezon-9-seria/">Чужая</a>"#;
    let entries = site_sources::episode_entries(html, &base, source);
    assert_eq!(entries.len(), 2);
    assert_eq!((entries[0].season, entries[0].episode), (Some(1), Some(1)));
    assert_eq!((entries[1].season, entries[1].episode), (Some(2), Some(3)));
    let park = site_sources::by_id("southpark").unwrap();
    let base = Url::parse(park.url).unwrap();
    let entries = site_sources::episode_entries(r#"<a href="/page.php?id=102">Следующая</a>"#, &base, park);
    assert_eq!(entries.len(), 1);
    assert_eq!((entries[0].season, entries[0].episode), (Some(1), Some(1)));
    let next = Url::parse("https://south-park.mult-fan.tv/page.php?id=102").unwrap();
    assert!(site_sources::episode_entries("<h1>Южный парк</h1>", &next, park).is_empty());
    let entries = site_sources::episode_entries("<h1>Южный парк — 2 сезон, 7 серия</h1>", &next, park);
    assert_eq!((entries[0].season, entries[0].episode), (Some(2), Some(7)));
}

#[test]
fn cvh_requires_the_exact_voice_episode_and_season() {
    let value = json!({"items":[
        {"voiceStudio":"AniLibria","season":1,"episode":1,"vkId":"1001"},
        {"voiceStudio":"AniDUB","season":1,"episode":1,"vkId":"1002"},
        {"voiceStudio":"AniLibria","season":1,"episode":2,"vkId":"1003"},
        {"voiceStudio":"AniLibria","season":2,"episode":1,"vkId":"1004"}
    ]});
    assert_eq!(site_sources::cvh_video(&value, "anilibria", 2, None).unwrap(), "1003");
    assert_eq!(site_sources::cvh_video(&value, "AniLibria", 1, Some(2)).unwrap(), "1004");
    assert!(site_sources::cvh_video(&value, "AniLibria", 1, None).is_err());
    assert!(site_sources::cvh_video(&value, "Unknown", 1, Some(1)).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn ambiguous_local_titles_require_manual_match() {
    let dir = tempfile::tempdir().unwrap();
    let (running, _) = common::start(dir.path()).await;
    running.app.db.with(|c| {
        for (id, year) in [("tt1234567", 1980), ("tt1234568", 2020)] {
            c.execute("INSERT INTO ext_titles(id,kind,title,year,updated_at) VALUES(?1,'movie','Одинаковое название',?2,0)", rusqlite::params![id, year])?;
        }
        Ok(())
    }).unwrap();
    let mut entry = site_sources::Entry {
        url: "https://example.com/movie".into(),
        title: "Одинаковое название".into(),
        kind: "movie".into(),
        year: None,
        imdb: None,
        remote_id: None,
        season: None,
        episode: None,
    };
    assert!(site_sources::match_imdb(&running.app, &entry).await.unwrap_err().to_string().contains("Несколько"));
    entry.year = Some(2020);
    assert_eq!(site_sources::match_imdb(&running.app, &entry).await.unwrap(), "tt1234568");
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живая проверка публичных страниц, плейлистов и сегментов"]
async fn live_requested_sources_and_real_streams() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    for id in ["lordfilm", "lordserials", "sopranos", "friends", "supernatural", "bear", "southpark", "yummy", "potter"]
    {
        let source = site_sources::by_id(id).unwrap();
        let result = site_sources::list(source, None, Some(&running.app)).await;
        match result {
            Ok((entries, next)) => println!("{id}: найдено {}, следующая страница {}", entries.len(), next.is_some()),
            Err(e) => println!("{id}: недоступен: {e}"),
        }
    }
    assert!(host.send("POST", "/api/admin/site-sources/bear/connect", json!({})).await.status().is_success());
    assert!(host.send("POST", "/api/admin/site-sources/bear/import", json!({})).await.status().is_success());
    let complete = tokio::time::timeout(std::time::Duration::from_secs(150), async {
        loop {
            let status = host.json("/api/admin/site-sources").await;
            let s = status["items"].as_array().unwrap().iter().find(|s| s["source"]["id"] == "bear").unwrap();
            if s["running"] == false {
                return s.clone();
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    })
    .await
    .unwrap();
    println!("bear import: {complete}");
    let links = host.json("/api/links?extId=tt14452776").await;
    let first = links
        .as_array()
        .expect("links")
        .iter()
        .find(|l| l["season"] == 1 && l["episode"] == 1)
        .expect("Первая серия Медведя");
    let id = first["id"].as_i64().unwrap();
    assert_eq!(host.json(&format!("/api/links/{id}/stream/source")).await["hls"], true);
    let master = host.get(&format!("/api/links/{id}/stream/index.m3u8")).await.text().await.unwrap();
    assert!(master.starts_with("#EXTM3U"), "{master}");
    assert!(master.contains("HDRezka"));
    assert!(master.contains("/stream/subtitle/"));
    let playlist = master.lines().find(|l| !l.is_empty() && !l.starts_with('#')).unwrap();
    let child = host.get(&format!("/api/links/{id}/stream/{playlist}")).await.text().await.unwrap();
    assert!(child.contains("#EXTINF"));
    let segments: Vec<_> = child.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
    for part in [segments[0], segments[segments.len() - 1]] {
        let r = host.get(&format!("/api/links/{id}/stream/{part}")).await;
        assert!(r.status().is_success());
        assert!(!r.bytes().await.unwrap().is_empty());
    }
    let sub = host.get(&format!("/api/links/{id}/stream/subtitle/0.m3u8")).await;
    assert!(sub.status().is_success(), "{}", sub.status());
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живой DASH Aksor из YummyAnime, MPD, первый и последний сегменты"]
async fn live_yummy_dash_runs_through_nova_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    assert!(host.send("POST", "/api/admin/site-sources/yummy/connect", json!({})).await.status().is_success());
    let videos = site_sources::json("https://old.yummyani.me/api/anime/258/videos").await.unwrap();
    let frame = videos["response"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["iframe_url"].as_str())
        .find(|u| u.starts_with("https://player.aksor.tv/video/"))
        .unwrap();
    let url = format!("{frame}#nova-quality=q1080");
    let target = format!("nv{}", "a".repeat(64));
    let id=running.app.db.with(|c|{
        c.execute("INSERT INTO ext_titles(id,kind,title,detailed,updated_at) VALUES(?1,'series','Дороро',?2,0)",rusqlite::params![target,fsfy::discover::DETAILED])?;
        c.execute("INSERT INTO links(ext_id,kind,url,label,added_at,season,episode) VALUES(?1,'site',?2,'Дороро - Aksor',0,1,1)",rusqlite::params![target,url])?;let id=c.last_insert_rowid();
        c.execute("INSERT INTO catalog_links(link_id,domain,item_key,stream_key) VALUES(?1,'old.yummyani.me',?2,'live')",rusqlite::params![id,target])?;Ok(id)
    }).unwrap();
    let s = host.json(&format!("/api/links/{id}/stream/source")).await;
    assert_eq!(s["dash"], true);
    let response = host.get(s["url"].as_str().unwrap()).await;
    assert!(response.status().is_success());
    let mpd = response.text().await.unwrap();
    let base = regex::Regex::new(r"<BaseURL>([^<]+)</BaseURL>").unwrap().captures(&mpd).unwrap()[1].to_string();
    assert!(base.starts_with(&format!("/api/links/{id}/stream/dash/")));
    let repr = regex::Regex::new(r#"<Representation\s+id="([^"]+)""#).unwrap().captures(&mpd).unwrap()[1].to_string();
    let init = regex::Regex::new(r#"initialization="([^"]+)""#).unwrap().captures(&mpd).unwrap()[1]
        .replace("$RepresentationID$", &repr);
    let media = regex::Regex::new(r#"media="([^"]+)""#).unwrap().captures(&mpd).unwrap()[1].to_string();
    let timeline = regex::Regex::new(r"(?s)<SegmentTimeline>(.*?)</SegmentTimeline>").unwrap().captures(&mpd).unwrap()
        [1]
    .to_string();
    let count: usize = regex::Regex::new(r"<S\b([^>]*)/>")
        .unwrap()
        .captures_iter(&timeline)
        .map(|c| {
            regex::Regex::new(r#"\br="(\d+)""#)
                .unwrap()
                .captures(&c[1])
                .and_then(|r| r[1].parse::<usize>().ok())
                .unwrap_or(0)
                + 1
        })
        .sum();
    for path in [
        init,
        media.replace("$RepresentationID$", &repr).replace("$Number%05d$", "00001"),
        media.replace("$RepresentationID$", &repr).replace("$Number%05d$", &format!("{count:05}")),
    ] {
        let r = host
            .http
            .get(format!("{}{}{}", host.base, base, path))
            .header("cookie", &host.cookie)
            .header("Range", "bytes=0-4095")
            .send()
            .await
            .unwrap();
        assert!(r.status().is_success(), "{}", r.status());
        assert!(r.bytes().await.unwrap().len() > 16);
    }
    let detail = host.json(&format!("/api/discover/title/{target}")).await;
    assert_eq!(detail["card"]["title"], "Дороро");
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живой фильм LordFilm через найденный на странице плеер и прокси Nova"]
async fn live_lordfilm_movie_runs_through_nova_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let entry = site_sources::Entry {
        url: "https://sr.lordfilm5.pro/filmy/40583-garri-potter-i-filosofskij-kamen-2001-3339-32232.html".into(),
        title: "Гарри Поттер и философский камень".into(),
        kind: "movie".into(),
        year: Some(2001),
        imdb: Some("tt0241527".into()),
        remote_id: None,
        season: None,
        episode: None,
    };
    let (variants, notes) = site_sources::variants(&entry, Some(&running.app)).await.unwrap();
    println!("lordfilm movie: {} вариантов, замечания: {notes:?}", variants.len());
    let variant = variants.iter().find(|v| v.url.contains("api.nextembed.ws")).expect("Плеер страницы фильма");
    assert!(host.send("POST", "/api/admin/site-sources/lordfilm/connect", json!({})).await.status().is_success());
    let id = running.app.db.with(|c| {
        c.execute("INSERT INTO ext_titles(id,kind,title,year,updated_at) VALUES('tt0241527','movie','Гарри Поттер и философский камень',2001,0)", [])?;
        c.execute("INSERT INTO links(ext_id,kind,url,label,added_at) VALUES('tt0241527','site',?1,'Фильм LordFilm',0)", [&variant.url])?;
        let id = c.last_insert_rowid();
        c.execute("INSERT INTO catalog_links(link_id,domain,item_key,stream_key) VALUES(?1,'sr.lordfilm5.pro','tt0241527','live')", [id])?;
        Ok(id)
    }).unwrap();
    assert_eq!(host.json(&format!("/api/links/{id}/stream/source")).await["hls"], true);
    let master = host.get(&format!("/api/links/{id}/stream/index.m3u8")).await.text().await.unwrap();
    assert!(master.starts_with("#EXTM3U"), "{master}");
    let playlist = master.lines().find(|l| !l.is_empty() && !l.starts_with('#')).unwrap();
    let child = host.get(&format!("/api/links/{id}/stream/{playlist}")).await.text().await.unwrap();
    assert!(child.contains("#EXTINF"), "{child}");
    let segments: Vec<_> = child.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
    for part in [segments[0], segments[segments.len() - 1]] {
        let r = host.get(&format!("/api/links/{id}/stream/{part}")).await;
        assert!(r.status().is_success(), "{}", r.status());
        assert!(!r.bytes().await.unwrap().is_empty());
    }
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живой CVH YummyAnime: точная озвучка, все объявленные варианты, HLS и Range файла"]
async fn live_cvh_runs_through_nova_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let url =
        Url::parse("https://ru.yummyani.me/iframeCVH.html?dubbing_code=AniLibria&anime_id=37520&episode=1").unwrap();
    let fallback = site_sources::Variant {
        id: "cvh-live".into(),
        url: url.to_string(),
        label: "AniLibria · CVH".into(),
        season: Some(1),
        episode: Some(1),
    };
    let variants = site_sources::cvh_variants(&url, &fallback).await.unwrap();
    println!("CVH: {} вариантов", variants.len());
    let hls = variants.iter().find(|v| v.url.contains("nova-format=hlsUrl")).unwrap();
    let mp4 = variants.iter().find(|v| v.url.contains("nova-format=mpegFullHdUrl")).unwrap();
    assert!(hls.url.contains("nova-cvh=10079801858774"));
    assert!(host.send("POST", "/api/admin/site-sources/yummy/connect", json!({})).await.status().is_success());
    let ids = running.app.db.with(|c| {
        c.execute("INSERT INTO ext_titles(id,kind,title,updated_at) VALUES('tt9458304','series','Дороро',0)", [])?;
        let mut ids = Vec::new();
        for (i, v) in [hls, mp4].iter().enumerate() {
            c.execute("INSERT INTO links(ext_id,kind,url,label,season,episode,added_at) VALUES('tt9458304','site',?1,?2,1,1,0)", rusqlite::params![v.url,v.label])?;
            let id=c.last_insert_rowid();
            c.execute("INSERT INTO catalog_links(link_id,domain,item_key,stream_key) VALUES(?1,'old.yummyani.me','tt9458304',?2)", rusqlite::params![id,format!("cvh-live-{i}")])?;
            ids.push(id);
        }
        Ok(ids)
    }).unwrap();
    let id = ids[0];
    assert_eq!(host.json(&format!("/api/links/{id}/stream/source")).await["hls"], true);
    let master = host.get(&format!("/api/links/{id}/stream/index.m3u8")).await.text().await.unwrap();
    assert!(master.contains("RESOLUTION=1920x1080"), "{master}");
    let playlist = master.lines().find(|l| !l.is_empty() && !l.starts_with('#')).unwrap();
    let child = host.get(&format!("/api/links/{id}/stream/{playlist}")).await.text().await.unwrap();
    assert!(child.contains("#EXTINF"), "{child}");
    let parts: Vec<_> = child.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
    for part in [parts[0], parts[parts.len() - 1]] {
        let response = host
            .http
            .get(format!("{}/api/links/{id}/stream/{part}", host.base))
            .header("cookie", &host.cookie)
            .header("Range", "bytes=0-4095")
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        assert!(response.bytes().await.unwrap().len() > 16);
    }
    let source = host.json(&format!("/api/links/{}/stream/source", ids[1])).await;
    assert_eq!(source["hls"], false);
    let response = host
        .http
        .get(format!("{}{}", host.base, source["url"].as_str().unwrap()))
        .header("cookie", &host.cookie)
        .header("Range", "bytes=0-4095")
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success(), "{}", response.status());
    assert!(response.bytes().await.unwrap().len() > 16);
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живая проверка всех приложенных сайтов через поиск Nova"]
async fn live_all_attached_sites() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO ext_titles(id,kind,title,year,updated_at) VALUES('tt9458304','series','Дороро',2019,0)",
                [],
            )
        })
        .unwrap();
    let cases = [
        (
            "sx.lordfilm5.pro",
            "Матрица",
            "https://sx.lordfilm5.pro/index.php?do=search&subaction=search&story={q}",
            "movie",
            None,
        ),
        (
            "lordserials.fan",
            "Друзья",
            "https://lordserials.fan/index.php?do=search&subaction=search&story={q}",
            "series",
            Some((1, 1)),
        ),
        (
            "watch-the-sopranos-online.net",
            "Клан Сопрано",
            "https://www.watch-the-sopranos-online.net/?s={q}",
            "series",
            Some((1, 1)),
        ),
        ("serialfriends.org", "Друзья", "https://serialfriends.org/?s={q}", "series", Some((1, 1))),
        ("supernatural-spn.net", "Сверхъестественное", "https://supernatural-spn.net/?s={q}", "series", Some((1, 1))),
        ("old.yummyani.me", "Дороро", "", "anime", Some((1, 1))),
    ];
    for (domain, name, search, kind, episode) in cases {
        let response = host
            .send(
                "PUT",
                "/api/admin/catalogs",
                json!({"items":[{"domain":domain,"name":name,"kind":kind,"enabled":true,"search":if search.is_empty(){serde_json::Value::Null}else{json!(search)} }]}),
            )
            .await;
        assert!(response.status().is_success(), "{domain}: {}", response.status());
        let q: String = url::form_urlencoded::byte_serialize(name.as_bytes()).collect();
        let mut path = format!("/api/links/search?q={q}&provider=own");
        if let Some((season, ep)) = episode {
            path.push_str(&format!("&season={season}&episode={ep}"));
        }
        let response = host.get(&path).await;
        let status = response.status();
        let text = response.text().await.unwrap();
        let found: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
        let count = found.as_array().map(Vec::len).unwrap_or(0);
        println!("{domain}: {status}, найдено {count}, первый {:?}", found[0]["url"]);
        assert!(status.is_success(), "{domain}: {text}");
        assert!(count > 0, "{domain}: пустой поиск ({text})");
        if domain == "old.yummyani.me" {
            let first = &found[0];
            let link = host
                .send(
                    "POST",
                    "/api/links",
                    json!({"url":first["url"],"label":first["label"],"extId":"tt9458304","season":1,"episode":1,"sourceDomain":domain}),
                )
                .await;
            assert!(link.status().is_success(), "{}", link.status());
            let id = link.json::<serde_json::Value>().await.unwrap()["id"].as_i64().unwrap();
            let source = host.get(&format!("/api/links/{id}/stream/source")).await;
            let status = source.status();
            let text = source.text().await.unwrap();
            println!("Дороро stream/source: {status} {text}");
            assert!(status.is_success(), "{text}");
        }
    }
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живая проверка полной цепочки Сопрано через API Nova"]
async fn live_sopranos_full_api_chain() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO ext_titles(id,kind,title,year,updated_at) VALUES('tt0141842','series','Клан Сопрано',1999,0)",
                [],
            )
        })
        .unwrap();
    let response = host
        .send(
            "PUT",
            "/api/admin/catalogs",
            json!({"items":[{"domain":"watch-the-sopranos-online.net","name":"Клан Сопрано", "kind":"series","enabled":true,"search":"https://www.watch-the-sopranos-online.net/?s={q}"}]}),
        )
        .await;
    assert!(response.status().is_success(), "{}", response.status());
    let q: String = url::form_urlencoded::byte_serialize("Клан Сопрано".as_bytes()).collect();
    let response = host.get(&format!("/api/links/search?q={q}&extId=tt0141842&provider=own&season=1&episode=1")).await;
    let status = response.status();
    let text = response.text().await.unwrap();
    println!("поиск: {status} {text}");
    assert!(status.is_success());
    let results: serde_json::Value = serde_json::from_str(&text).unwrap();
    let items = results.as_array().expect("массив результатов");
    let playable = items
        .iter()
        .find(|v| v["label"].as_str().is_some_and(|l| l.contains("Гоблин")))
        .or_else(|| items.first())
        .expect("вариант первой серии");
    let link = host
        .send(
            "POST",
            "/api/links",
            json!({"url":playable["url"],"label":playable["label"],"extId":"tt0141842","season":1,"episode":1,"sourceDomain":"watch-the-sopranos-online.net"}),
        )
        .await;
    assert!(link.status().is_success(), "{}", link.status());
    let id = link.json::<serde_json::Value>().await.unwrap()["id"].as_i64().unwrap();
    let source = host.get(&format!("/api/links/{id}/stream/source")).await;
    println!("stream/source: {}", source.status());
    println!("{}", source.text().await.unwrap());
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Живая проверка обхода антибота, серий и озвучек Сопрано/Друзей/Сверхъестественного"]
async fn live_dooplay_series_pages_and_variants() {
    let dir = tempfile::tempdir().unwrap();
    let (running, _) = common::start(dir.path()).await;
    for id in ["sopranos", "friends", "supernatural"] {
        let source = site_sources::by_id(id).unwrap();
        let (entries, _) = site_sources::list(source, None, Some(&running.app)).await.unwrap();
        assert!(!entries.is_empty(), "{id}: список серий пуст");
        assert!(entries.iter().any(|e| e.season == Some(1) && e.episode == Some(1)), "{id}: нет первой серии");
        let first = entries.iter().find(|e| e.season == Some(1) && e.episode == Some(1)).unwrap();
        let (variants, notes) = site_sources::variants(first, Some(&running.app)).await.unwrap();
        assert!(!variants.is_empty(), "{id}: не найдены варианты плеера: {notes:?}");
        let labels = variants.iter().map(|v| v.label.as_str()).collect::<Vec<_>>().join(" | ");
        let urls = variants.iter().map(|v| v.url.as_str()).collect::<Vec<_>>().join(" | ");
        assert!(urls.contains("plplayer.online") || urls.contains("online-serial.com"), "{id}: {urls}");
        if id == "sopranos" {
            assert!(labels.contains("Гоблин"), "{labels}");
            assert!(labels.contains("Украинский"), "{labels}");
        }
        println!("{id}: серий {}, вариантов {}, озвучки {}", entries.len(), variants.len(), labels);
    }
    let dororo = site_sources::Entry {
        url: "https://old.yummyani.me/catalog/item/dororo-2026-11-06".into(),
        title: "Дороро".into(),
        kind: "series".into(),
        year: Some(2026),
        imdb: None,
        remote_id: Some(258),
        season: None,
        episode: None,
    };
    let (variants, notes) = site_sources::variants(&dororo, Some(&running.app)).await.unwrap();
    assert!(!variants.is_empty(), "Дороро: варианты не найдены: {notes:?}");
    assert!(variants.iter().all(|v| v.season == Some(1) && v.episode.is_some_and(|e| e > 0)));
    println!("Дороро: вариантов {}", variants.len());
    running.shutdown().await;
}
