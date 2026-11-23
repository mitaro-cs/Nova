//! Лента IMDb без сети: фильмы не с сервиса, «Буду смотреть», просьбы хосту, группы в списках,
//! ссылки на видео с других сайтов.

mod common;

use common::Client;
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread")]
async fn russian_episodes_include_old_fresh_seasons_without_replacing_source() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    let app = &running.app;
    fsfy::discover::upsert(app, &[ext("tt3543502", "Гангстерленд", true)]).unwrap();
    app.db.with(|c| {
        c.execute("UPDATE ext_titles SET extra='{\"seasons\":[1]}' WHERE id='tt3543502'",[])?;
        c.execute("INSERT INTO ext_seasons VALUES('tt3543502',1,?)",[fsfy::util::now_ms()])?;
        c.execute("INSERT INTO ext_episodes(ext_id,season,episode,name,overview) VALUES('tt3543502',1,1,'Stick or Twist','An English synopsis'),('tt3543502',1,2,'Jigsaw Puzzle',NULL)",[])?;
        Ok(())
    }).unwrap();
    app.db.set_setting(&fsfy::episode_locale::key("Stick or Twist"), "Пан или пропал").unwrap();
    // Do not make a real translation call in this offline contract test.
    for text in ["Jigsaw Puzzle", "An English synopsis"] {
        app.db
            .set_setting(&format!("{}.at", fsfy::episode_locale::key(text)), &fsfy::util::now_ms().to_string())
            .unwrap();
    }
    let response = host.json("/api/discover/title/tt3543502/episodes?season=1").await;
    assert_eq!(response["items"][0]["name"], "Пан или пропал");
    // Нет русского названия - номер серии, а не одно слово «Серия».
    assert_eq!(response["items"][1]["name"], "2 серия");
    assert!(response["items"][0]["overview"].is_null());
    let source: String = app
        .db
        .with(|c| c.query_row("SELECT name FROM ext_episodes WHERE ext_id='tt3543502' AND episode=1", [], |r| r.get(0)))
        .unwrap();
    assert_eq!(source, "Stick or Twist");
    let first = dir.path().join("first.mp4");
    let second = dir.path().join("second.mp4");
    std::fs::write(&first, []).unwrap();
    std::fs::write(&second, []).unwrap();
    app.db.with(|c| {
        c.execute("INSERT INTO titles(id,kind,key,title,meta_status,added_at,updated_at) VALUES(9000,'series','locale-test','Локальный сериал','manual',0,0)",[])?;
        for (id,episode,path,name) in [(9001,1,&first,"Stick or Twist"),(9002,2,&second,"Jigsaw Puzzle")] {
            c.execute("INSERT INTO files(id,title_id,path,size,mtime,season,episode,name,added_at) VALUES(?,9000,?,0,0,1,?,?,0)",rusqlite::params![id,path.to_string_lossy(),episode,name])?;
        }
        Ok(())
    }).unwrap();
    let info = host.json("/api/files/9001/info").await;
    assert_eq!(info["episode"], "1 сезон, 1 серия · Пан или пропал");
    assert_eq!(info["nextInfo"]["label"], "1 сезон, 2 серия");
    app.db.set_setting("meta.language", "source").unwrap();
    let response = host.json("/api/discover/title/tt3543502/episodes?season=1").await;
    assert_eq!(response["items"][0]["name"], "Stick or Twist");
    running.shutdown().await;
}

fn ext(id: &str, title: &str, series: bool) -> fsfy::discover::Ext {
    fsfy::discover::Ext {
        id: id.into(),
        series,
        title: title.into(),
        original: None,
        year: Some(2020),
        rating: Some(8.1),
        votes: Some(1000),
        genres: vec!["Драма".into()],
        poster_url: None,
        release: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires live LordFilm search"]
async fn lordfilm_automatic_search_and_save_without_host_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let mut film = ext("tt1160419", "Дюна", false);
    film.year = Some(2021);
    fsfy::discover::upsert(&running.app, &[film]).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&running.app.db.setting_or("video.sites", "[]")).unwrap(), json!([]));
    let found = host.json("/api/links/search?provider=lordfilm&extId=tt1160419&q=Dune&min=2400").await;
    let movie = found.as_array().unwrap().first().expect("automatic movie result");
    assert_eq!(movie["site"], "lordfilm");
    let response = host
        .send(
            "POST",
            "/api/links",
            json!({
                "url": movie["url"], "label": movie["label"], "extId":"tt1160419"
            }),
        )
        .await;
    assert!(response.status().is_success());
    let id = response.json::<Value>().await.unwrap()["id"].as_i64().unwrap();
    let kind: String =
        running.app.db.with(|c| c.query_row("SELECT kind FROM links WHERE id=?", [id], |r| r.get(0))).unwrap();
    assert_eq!(kind, "site");
    let trailer = host.json("/api/links/search?provider=lordfilm&q=Dune&mode=trailer").await;
    assert_eq!(trailer, json!([]));
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires live LordFilm series and CDN"]
async fn lordfilm_series_search_save_and_stream_exact_episode() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let mut series = ext("tt1234567", "Дюна: Пророчество", true);
    series.year = Some(2024);
    fsfy::discover::upsert(&running.app, &[series]).unwrap();
    let found =
        host.json("/api/links/search?provider=lordfilm&extId=tt1234567&q=Dune&min=600&season=1&episode=2").await;
    let episode = found.as_array().unwrap().first().expect("second episode");
    assert!(episode["url"].as_str().unwrap().ends_with("#nova-season=1&episode=2"));
    assert!(episode["label"].as_str().unwrap().ends_with("S01E02"));
    let wrong =
        host.send("POST", "/api/links", json!({"url":episode["url"],"extId":"tt1234567","season":1,"episode":1})).await;
    assert_eq!(wrong.status().as_u16(), 400);
    let response = host
        .send(
            "POST",
            "/api/links",
            json!({"url":episode["url"],"extId":"tt1234567","season":1,"episode":2,"label":episode["label"]}),
        )
        .await;
    assert!(response.status().is_success());
    let id = response.json::<Value>().await.unwrap()["id"].as_i64().unwrap();
    let source = host.json(&format!("/api/links/{id}/stream/source")).await;
    assert_eq!(source["hls"], true);
    let response = host.get(source["url"].as_str().unwrap()).await;
    assert!(response.status().is_success());
    let playlist = response.text().await.unwrap();
    assert!(playlist.starts_with("#EXTM3U"));
    let variant = playlist.lines().find(|l| !l.is_empty() && !l.starts_with('#')).unwrap();
    assert!(variant.starts_with("p?u="));
    let response = host.get(&format!("/api/links/{id}/stream/{variant}")).await;
    assert!(response.status().is_success());
    let playlist = response.text().await.unwrap();
    assert!(playlist.starts_with("#EXTM3U"));
    let segment = playlist.lines().find(|l| !l.is_empty() && !l.starts_with('#')).unwrap();
    let mut response = host
        .http
        .get(format!("{}/api/links/{id}/stream/{segment}", host.base))
        .header("cookie", &host.cookie)
        .header("range", "bytes=0-1023")
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success());
    assert!(!response.chunk().await.unwrap().unwrap().is_empty());
    let absent = host.json("/api/links/search?provider=lordfilm&extId=tt1234567&q=Dune&season=99&episode=999").await;
    assert_eq!(absent, json!([]));
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn localized_search_cache_and_fast_details() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let app = &running.app;
    fsfy::discover::upsert(app, &[ext("tt0133093", "The Matrix", false)]).unwrap();
    app.db
        .with(|c| {
            c.execute("INSERT INTO title_localizations VALUES ('tt0133093','Матрица','{}',0)", [])?;
            c.execute(
                "INSERT INTO catalog_search_cache VALUES ('матрица','[\"tt0133093\"]',?)",
                [fsfy::util::now_ms()],
            )?;
            c.execute("UPDATE ext_titles SET detailed=3 WHERE id='tt0133093'", [])?;
            Ok(())
        })
        .unwrap();
    let started = std::time::Instant::now();
    let results = host.json("/api/discover/search?q=%D0%BC%D0%B0%D1%82%D1%80%D0%B8%D1%86%D0%B0").await;
    assert_eq!(results[0]["title"], "Матрица");
    assert!(started.elapsed() < std::time::Duration::from_secs(1), "кэш поиска не ждёт сеть");
    let started = std::time::Instant::now();
    assert_eq!(host.json("/api/discover/title/tt0133093").await["card"]["title"], "Матрица");
    // Без кадра страница ждёт подробности IMDb до 2 с (а не пустая и опрос), картинки - не ждёт никогда.
    // 3,5 с вместо 2,6: сам ожидаемый предел 2 с, остальное - накладные расходы теста при параллельном прогоне.
    assert!(started.elapsed() < std::time::Duration::from_millis(3500), "карточка не ждёт внешние картинки");
    for path in [
        "/api/links/thumbnail?url=http://127.0.0.1/test.jpg",
        "/api/links/thumbnail?url=https://example.com/test.jpg",
        "/api/discover/poster/invalid",
    ] {
        assert_eq!(host.send("GET", path, Value::Null).await.status(), 404, "{path}");
    }
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "ручная проверка реального русского поиска"]
async fn russian_search_live() {
    let dir = tempfile::tempdir().unwrap();
    let (running, _) = common::start(dir.path()).await;
    let ids = fsfy::catalog_search::search(&running.app, "матрица").await.unwrap();
    println!("Найдено: {}", ids.len());
    assert!(ids.iter().any(|id| id == "tt0133093"), "находит Матрицу 1999: {ids:?}");
    let title: String = running
        .app
        .db
        .with(|c| c.query_row("SELECT title FROM title_localizations WHERE id='tt0133093'", [], |r| r.get(0)))
        .unwrap();
    assert_eq!(title, "Матрица");
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn episode_links_resume_and_dropped_feed() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let app = &running.app;
    fsfy::discover::upsert(app, &[ext("tt0903747", "Во все тяжкие", true)]).unwrap();
    app.db.with(|c| c.execute("INSERT INTO ext_charts VALUES ('popular_series', 0, 'tt0903747')", [])).unwrap();
    let body =
        json!({"url":"https://example.com/episode.mp4", "extId":"tt0903747", "season":2,"episode":3,"label":"Серия 3"});
    let first: Value = host.send("POST", "/api/links", body.clone()).await.json().await.unwrap();
    let again: Value = host.send("POST", "/api/links", body).await.json().await.unwrap();
    assert_eq!(first["id"], again["id"], "повторное открытие не теряет прогресс");
    let id = first["id"].as_i64().unwrap();
    assert!(
        host.send("PUT", &format!("/api/links/{id}/progress"), json!({"position":120,"duration":1200}))
            .await
            .status()
            .is_success()
    );
    // Просмотр по ссылке виден хосту: «Сейчас смотрят», активность и «Самое смотримое».
    let status = host.json("/api/admin/status").await;
    let now = &status["watching"][0];
    assert_eq!(now["title"], "Во все тяжкие");
    assert_eq!(now["position"], 120.0);
    let metrics = host.json("/api/admin/metrics").await;
    assert_eq!(metrics["people"]["activeWeek"], 1);
    assert_eq!(metrics["activity"]["top"][0]["href"], "/ext/tt0903747");
    assert_eq!(metrics["activity"]["days"].as_array().unwrap().last().unwrap()["views"], 1);
    // «Топ-10 Nova за неделю» на главной - тот же просмотр, карточкой фильма ленты и числом зрителей.
    let top = host.json("/api/top/week").await;
    assert_eq!(top[0]["ext"]["id"], "tt0903747");
    assert_eq!(top[0]["viewers"], 1);
    let link = host.json(&format!("/api/links/{id}")).await;
    assert_eq!(link["season"], 2);
    assert_eq!(link["episode"], 3);
    assert_eq!(link["progress"]["position"], 120.0);
    let resume = host.json("/api/continue").await;
    assert_eq!(resume[0]["position"], 120.0);
    assert_eq!(resume[0]["duration"], 1200.0);
    assert_eq!(resume[0]["label"], "2 сезон, 3 серия");
    assert_eq!(resume[0]["href"], format!("/watch/link/{id}"));
    assert!(
        host.send("PUT", &format!("/api/links/{id}/progress"), json!({"position":1190,"duration":1200}))
            .await
            .status()
            .is_success()
    );
    let marks = host.json("/api/me/marks").await;
    assert_eq!(host.json("/api/continue").await, json!([]), "досмотренная серия не предлагается как продолжение");
    assert_eq!(marks["watching"].as_array().unwrap().len(), 1, "одна серия не завершает сериал");
    let seen: i64 = app
        .db
        .with(|c| c.query_row("SELECT COUNT(*) FROM ext_seen WHERE season=2 AND episode=3", [], |r| r.get(0)))
        .unwrap();
    assert_eq!(seen, 1);
    let me = host.json("/api/me").await;
    let profile = me["user"]["profile"]["id"].as_i64().unwrap();
    let stats = host.json(&format!("/api/members/{profile}")).await["stats"].clone();
    assert_eq!(stats["episodes"], 1, "внешняя серия учитывается в профиле");
    assert_eq!(stats["series"], 1);
    assert!(stats["hours"].as_f64().unwrap() > 0.0, "часы внешнего просмотра учитываются");
    assert_eq!(stats["months"][11], 1, "текущий календарный месяц включает внешнюю серию");
    assert!(
        host.send("PUT", "/api/discover/title/tt0903747/mark", json!({"status":"dropped"})).await.status().is_success()
    );
    let feed = host.json("/api/discover").await;
    assert!(!feed.to_string().contains("tt0903747"));
    assert_eq!(host.json("/api/me/marks").await["watching"], json!([]));
    assert_eq!(host.json("/api/continue").await, json!([]));
    assert!(
        host.send("PUT", &format!("/api/links/{id}/progress"), json!({"position":125,"duration":1200}))
            .await
            .status()
            .is_success()
    );
    assert_eq!(host.json("/api/me/marks").await["watching"].as_array().unwrap().len(), 1);
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "ручная проверка реальных источников: Викиданные и Википедия"]
async fn person_information_live() {
    let dir = tempfile::tempdir().unwrap();
    let (running, _) = common::start(dir.path()).await;
    let info = fsfy::directors::information(&running.app, "nm1466859", "Owain Yeoman", "actor").await.unwrap();
    assert!(info["biography"].as_str().is_some_and(|s| s.chars().count() > 60), "русская биография: {info}");
    assert!(info["works"].as_array().is_some_and(|w| w.len() > 2), "фильмография: {info}");
    assert!(info["birthDate"].as_str().is_some(), "дата рождения");
    println!(
        "LIVE PERSON: {} · {} работ · биография {} символов",
        info["nameRu"],
        info["works"].as_array().unwrap().len(),
        info["biography"].as_str().unwrap().chars().count()
    );
    let director_id = fsfy::directors::resolve(&running.app, "Кристофер Нолан").await.unwrap().unwrap();
    assert_eq!(director_id, "nm0634240");
    let director =
        fsfy::directors::information(&running.app, &director_id, "Кристофер Нолан", "director").await.unwrap();
    assert!(director["biography"].as_str().is_some_and(|s| s.chars().count() > 60), "биография режиссёра: {director}");
    assert!(director["works"].as_array().is_some_and(|w| w.len() > 2), "работы режиссёра: {director}");
    println!("LIVE DIRECTOR: {} · {} работ", director["nameRu"], director["works"].as_array().unwrap().len());
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn feed_marks_wishes_groups_links() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let app = running.app.clone();
    let public = format!("http://127.0.0.1:{}", app.ports.0);
    fsfy::discover::upsert(
        &app,
        &[ext("tt0111161", "Побег из Шоушенка", false), ext("tt0903747", "Во все тяжкие", true)],
    )
    .unwrap();
    app.db
        .with(|c| {
            c.execute_batch(
                "INSERT INTO ext_charts VALUES ('top_movies', 0, 'tt0111161'), ('popular_series', 0, 'tt0903747');",
            )
        })
        .unwrap();

    let mut anna = Client::new(public.clone());
    let r = anna
        .send(
            "POST",
            "/api/auth/register",
            json!({"consent": 1,"name": "Анна", "login": "anna", "password": "пароль-12345"}),
        )
        .await;
    assert!(r.status().is_success());

    // Стена постеров на входе - без входа и только постеры подборок IMDb.
    let guest = Client::new(public.clone());
    assert_eq!(guest.json("/api/site/wall").await, json!([]), "постеров ещё нет");
    let poster = app.images.save(b"\x89PNG\r\n\x1a\n-test-poster").unwrap();
    let other = app.images.save(b"\x89PNG\r\n\x1a\n-host-image").unwrap();
    app.db.with(|c| c.execute("UPDATE ext_titles SET poster = ? WHERE id = 'tt0111161'", [&poster])).unwrap();
    let wall = guest.json("/api/site/wall").await;
    assert_eq!(wall, json!([format!("/api/site/wall/{poster}")]));
    assert_eq!(guest.get(wall[0].as_str().unwrap()).await.status(), 200);
    assert_eq!(guest.get(&format!("/api/site/wall/{other}")).await.status(), 404, "чужие картинки - нет");
    assert_eq!(guest.get(&format!("/img/{poster}")).await.status(), 401, "/img - только после входа");

    // Лента: подборки по порядку.
    let feed = anna.json("/api/discover").await;
    let keys: Vec<&str> = feed["rows"].as_array().unwrap().iter().map(|r| r["key"].as_str().unwrap()).collect();
    assert_eq!(keys, ["popular_series", "top_movies"]);
    assert_eq!(feed["rows"][1]["items"][0]["title"], "Побег из Шоушенка");
    assert_eq!(feed["rows"][1]["items"][0]["local"], Value::Null, "на сервисе нет");

    // «Буду смотреть» и «Просмотрел».
    let r = anna.send("PUT", "/api/discover/title/tt0111161/mark", json!({"status": "want"})).await;
    assert!(r.status().is_success());
    anna.send("PUT", "/api/discover/title/tt0903747/mark", json!({"status": "watched"})).await;
    let marks = anna.json("/api/me/marks").await;
    assert_eq!(marks["want"][0]["ext"]["id"], "tt0111161");
    assert_eq!(marks["watched"][0]["ext"]["mark"], "watched");
    assert_eq!(anna.send("PUT", "/api/discover/title/tt0111161/mark", json!({"status": "bad"})).await.status(), 400);
    assert_eq!(anna.send("PUT", "/api/discover/title/tt9999999/mark", json!({"status": "want"})).await.status(), 404);

    // «Хочу на сервисе» - хост видит просьбу, значок, закрывает.
    anna.send("PUT", "/api/discover/title/tt0111161/wish", json!({"value": true})).await;
    let w = host.json("/api/admin/wishes").await;
    assert_eq!(
        (w[0]["card"]["id"].as_str(), w[0]["count"].as_i64(), w[0]["people"].as_str()),
        (Some("tt0111161"), Some(1), Some("Анна"))
    );
    assert_eq!(host.json("/api/badges").await["wishes"], 1);
    assert_eq!(anna.json("/api/badges").await["wishes"], 0, "зрителю - не показываем");
    assert_eq!(anna.get("/api/admin/wishes").await.status(), 403);
    host.send("DELETE", "/api/admin/wishes/tt0111161", json!({})).await;
    assert_eq!(host.json("/api/badges").await["wishes"], 0);
    assert_eq!(anna.json("/api/badges").await["ready"], json!([]), "«Не будем» - без уведомления");

    // «Выполнено»: просьба уходит у хоста, а тому, кто просил, приходит уведомление - пока не откроет.
    anna.send("PUT", "/api/discover/title/tt0111161/wish", json!({"value": true})).await;
    assert_eq!(anna.json("/api/badges").await["ready"], json!([]), "пока не выполнено - тишина");
    assert_eq!(anna.send("POST", "/api/admin/wishes/tt0111161/done", json!({})).await.status(), 403);
    assert_eq!(host.send("POST", "/api/admin/wishes/tt0111161/done", json!({})).await.status(), 200);
    assert_eq!(host.json("/api/admin/wishes").await, json!([]));
    assert_eq!(host.json("/api/badges").await["wishes"], 0);
    let ready = anna.json("/api/badges").await["ready"].clone();
    assert_eq!(ready.as_array().map(Vec::len), Some(1));
    assert_eq!(ready[0]["id"], "tt0111161");
    assert!(ready[0]["title"].as_str().is_some_and(|t| !t.is_empty()));
    assert_eq!(host.json("/api/badges").await["ready"], json!([]), "хост ничего не просил");
    anna.send("POST", "/api/me/wishes/tt0111161/seen", json!({})).await;
    assert_eq!(anna.json("/api/badges").await["ready"], json!([]));

    // Свой список с группами и фильмом из ленты.
    let l: Value = anna.send("POST", "/api/lists", json!({"name": "Хоррор на Хэллоуин"})).await.json().await.unwrap();
    let lid = l["id"].as_i64().unwrap();
    assert!(
        anna.send("PUT", &format!("/api/lists/{lid}/ext/tt0111161"), json!({"value": true}))
            .await
            .status()
            .is_success()
    );
    let g: Value =
        anna.send("POST", &format!("/api/lists/{lid}/groups"), json!({"name": "Сначала"})).await.json().await.unwrap();
    let gid = g["id"].as_i64().unwrap();
    let r = anna.send("PUT", &format!("/api/lists/{lid}/move"), json!({"extId": "tt0111161", "group": gid})).await;
    assert!(r.status().is_success());
    let full = anna.json(&format!("/api/lists/{lid}")).await;
    assert_eq!(full["groups"][0]["name"], "Сначала");
    assert_eq!(
        (full["items"][0]["ext"]["id"].as_str(), full["items"][0]["groupId"].as_i64()),
        (Some("tt0111161"), Some(gid))
    );
    assert_eq!(anna.json("/api/lists").await[0]["count"], 1);
    assert_eq!(anna.json("/api/discover/title/tt0111161/lists").await[0]["has"], true);
    anna.send("DELETE", &format!("/api/lists/{lid}/groups/{gid}"), json!({})).await;
    assert_eq!(
        anna.json(&format!("/api/lists/{lid}")).await["items"][0]["groupId"],
        Value::Null,
        "группа ушла, фильм остался"
    );
    assert_eq!(
        host.send("PUT", &format!("/api/lists/{lid}/ext/tt0111161"), json!({"value": false})).await.status(),
        404
    );

    // Ссылки: к фильму из ленты и сами по себе; чужую удалить нельзя.
    let r = anna
        .send(
            "POST",
            "/api/links",
            json!({"url": "https://vk.com/video-1_2", "extId": "tt0111161", "label": "Фильм в VK"}),
        )
        .await;
    assert!(r.status().is_success(), "{}", r.status());
    let link: Value = r.json().await.unwrap();
    let r = anna.send("POST", "/api/links", json!({"url": "https://example.com/page"})).await;
    assert_eq!(r.status(), 400);
    anna.send("POST", "/api/links", json!({"url": "https://cdn.example.com/trailer.mp4", "label": "Трейлер"})).await;
    let mine = anna.json("/api/links?extId=tt0111161").await;
    assert_eq!(
        (mine[0]["kind"].as_str(), mine[0]["embed"].as_str()),
        (Some("vk"), Some("https://vkvideo.ru/video_ext.php?oid=-1&id=2&hd=2"))
    );
    assert_eq!(anna.json("/api/links").await[0]["label"], "Трейлер");
    let detail = anna.json("/api/discover/title/tt0111161").await;
    assert_eq!(detail["links"][0]["label"], "Фильм в VK");

    // Своя страница фильма из ленты: кадр, актёры, кадры и похожие - из базы, без IMDb.
    let extra = json!({"tagline": "Страх держит в плену", "people": [{"name": "Тим Роббинс", "job": "actor",
        "role": "Энди", "photo": poster}], "gallery": [poster, ""], "similar": ["tt0903747", "tt0000404"],
        "countries": ["США"], "poster": poster});
    app.db
        .with(|c| {
            c.execute(
                "UPDATE ext_titles SET backdrop = ?, extra = ?, detailed = ? WHERE id = 'tt0111161'",
                rusqlite::params![poster, extra.to_string(), fsfy::discover::DETAILED],
            )
        })
        .unwrap();
    let page = anna.json("/api/discover/title/tt0111161").await;
    assert_eq!(page["backdrop"], format!("/img/{poster}"));
    assert_eq!(page["posterLarge"], format!("/img/{poster}"));
    assert_eq!(page["tagline"], "Страх держит в плену");
    assert_eq!(page["people"][0]["photo"], format!("/img/{poster}"));
    assert_eq!(page["gallery"], json!([format!("/img/{poster}")]), "пустые имена - мимо");
    assert_eq!(page["similar"].as_array().unwrap().len(), 1, "неизвестный id - мимо");
    assert_eq!(page["similar"][0]["title"], "Во все тяжкие");
    assert_eq!(page["countries"], json!(["США"]));
    let lid2 = link["id"].as_i64().unwrap();
    let mut boris = Client::new(public.clone());
    boris
        .send(
            "POST",
            "/api/auth/register",
            json!({"consent": 1,"name": "Б", "login": "boris", "password": "пароль-12345"}),
        )
        .await;
    assert_eq!(boris.send("DELETE", &format!("/api/links/{lid2}"), json!({})).await.status(), 403);
    assert!(host.send("DELETE", &format!("/api/links/{lid2}"), json!({})).await.status().is_success(), "хост может");

    // Оформление профиля и статистика.
    let r = anna
        .send(
            "PATCH",
            "/api/me/profile",
            json!({"bio": "  Люблю драмы ", "accent": "#FF8800", "favorites": ["x:tt0903747", "x:tt0903747", "zz"]}),
        )
        .await;
    assert!(r.status().is_success());
    assert_eq!(anna.send("PATCH", "/api/me/profile", json!({"accent": "red"})).await.status(), 400);
    let pa = anna.json("/api/me").await["user"]["profile"]["id"].as_i64().unwrap();
    let page = host.json(&format!("/api/members/{pa}")).await;
    assert_eq!(
        (page["decor"]["bio"].as_str(), page["decor"]["accent"].as_str()),
        (Some("Люблю драмы"), Some("#ff8800"))
    );
    assert_eq!(page["decor"]["favorites"].as_array().unwrap().len(), 1, "повтор и мусор отброшены");
    assert_eq!(page["decor"]["favorites"][0]["ext"]["title"], "Во все тяжкие");
    assert_eq!(page["stats"]["outside"], 1, "просмотренное не с сервиса считается");

    // Любимые - как на IMDb, не больше пяти; с карточки фильма - по одному.
    let r: Value =
        anna.send("PUT", "/api/me/favorites", json!({"key": "x:tt0111161", "value": true})).await.json().await.unwrap();
    assert_eq!(r["favorites"], json!(["x:tt0903747", "x:tt0111161"]));
    assert_eq!(anna.json("/api/me").await["favorites"], json!(["x:tt0903747", "x:tt0111161"]));
    for i in 0..4 {
        let r = anna.send("PUT", "/api/me/favorites", json!({"key": format!("t:{}", 100 + i), "value": true})).await;
        assert!(r.status().is_success());
    }
    // По пять отдельно у фильмов и у сериалов: шестой фильм - нельзя, пока не убрать один.
    let sixth = anna.send("PUT", "/api/me/favorites", json!({"key": "t:200", "value": true})).await;
    assert_eq!(sixth.status(), 400, "шестой любимый фильм");
    let r: Value =
        anna.send("PUT", "/api/me/favorites", json!({"key": "t:100", "value": false})).await.json().await.unwrap();
    assert_eq!(r["favorites"].as_array().unwrap().len(), 5);
    let again = anna.send("PUT", "/api/me/favorites", json!({"key": "t:200", "value": true})).await;
    assert!(again.status().is_success(), "место освободилось");
    assert_eq!(anna.send("PUT", "/api/me/favorites", json!({"key": "zz", "value": true})).await.status(), 400);

    // Любимый режиссёр - один, и лента просмотренного на обложке. Два режиссёра - нельзя.
    let two = anna.send("PATCH", "/api/me/profile", json!({"directors": ["Нолан", "Финчер"]})).await;
    assert_eq!(two.status(), 400);
    let r =
        anna.send("PATCH", "/api/me/profile", json!({"directors": [" Нолан ", "нолан"], "coverKind": "ribbon"})).await;
    assert!(r.status().is_success());
    app.db.with(|c| c.execute("UPDATE ext_titles SET poster = ? WHERE id = 'tt0903747'", [&poster])).unwrap();
    let page = host.json(&format!("/api/members/{pa}")).await;
    assert_eq!(page["decor"]["directors"], json!(["Нолан"]));
    assert_eq!(page["decor"]["coverKind"], "ribbon");
    assert_eq!(page["decor"]["ribbon"], json!([format!("/img/{poster}")]));
    assert_eq!(anna.send("PATCH", "/api/me/profile", json!({"coverKind": "video"})).await.status(), 400);
    assert_eq!(page["stats"]["months"].as_array().unwrap().len(), 12);

    // Смена пароля закрывает остальные входы (украденная сессия перестаёт работать).
    let mut other = Client::new(public.clone());
    let r = other.send("POST", "/api/auth/login", json!({"login": "anna", "password": "пароль-12345"})).await;
    assert!(r.status().is_success());
    let r = anna.send("PUT", "/api/me/password", json!({"old": "пароль-12345", "new": "новый-пароль-1"})).await;
    assert!(r.status().is_success(), "{}", r.status());
    assert_eq!(anna.get("/api/me").await.status(), 200, "свой вход остаётся");
    assert_eq!(other.get("/api/me").await.status(), 401, "чужой — закрыт");

    // Путь к ffmpeg (программа, которую запустит сервер) меняет только хост из окна и не сетевой.
    let ida = anna.json("/api/me").await["user"]["id"].as_i64().unwrap();
    let r = host.send("PATCH", &format!("/api/admin/users/{ida}"), json!({"role": "admin"})).await;
    assert!(r.status().is_success(), "{}", r.status());
    let r = anna.send("PUT", "/api/admin/settings", json!({"ffmpegPath": "C:\\x"})).await;
    assert_eq!(r.status(), 403, "администратор — нет");
    let r = host.send("PUT", "/api/admin/settings", json!({"ffmpegPath": "\\\\evil\\share"})).await;
    assert_eq!(r.status(), 400, "сетевая папка — нет");
    drop(running);
}

#[tokio::test(flavor = "multi_thread")]
async fn next_episode_stays_in_series_and_moves_to_next_season() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    fsfy::discover::upsert(&running.app, &[ext("tt0903747", "Во все тяжкие", true)]).unwrap();
    running
        .app
        .db
        .with(|c| {
            c.execute("UPDATE ext_titles SET extra='{\"seasons\":[1,2]}' WHERE id='tt0903747'", [])?;
            for (s, e) in [(1, 1), (1, 2), (2, 1)] {
                c.execute(
                    "INSERT INTO ext_episodes(ext_id,season,episode) VALUES('tt0903747',?,?)",
                    rusqlite::params![s, e],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let mut ids = vec![];
    for (s, e) in [(1, 1), (1, 2), (2, 1)] {
        let result:Value=host.send("POST","/api/links",json!({"url":format!("https://example.com/{s}-{e}.mp4"),"extId":"tt0903747","season":s,"episode":e,"label":format!("Сезон {s}, серия {e}")})).await.json().await.unwrap();
        ids.push(result["id"].as_i64().unwrap());
    }
    assert_eq!(host.json(&format!("/api/links/{}/next", ids[0])).await["next"]["id"], ids[1]);
    assert_eq!(host.json(&format!("/api/links/{}/next", ids[1])).await["next"]["id"], ids[2]);
    assert!(host.json(&format!("/api/links/{}/next", ids[2])).await["next"].is_null());
    let native: Value = host.send("POST", "/api/host/fullscreen", json!({"enabled":true})).await.json().await.unwrap();
    assert_eq!(native["native"], false, "headless-сервер не меняет окно");
    let mut public = common::Client::new(format!("http://127.0.0.1:{}", running.app.ports.0));
    public.cookie = host.cookie.clone();
    assert!(!public.send("POST", "/api/host/fullscreen", json!({"enabled":true})).await.status().is_success());
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn seen_episode_marks_previous_seasons_too() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let app = &running.app;
    fsfy::discover::upsert(app, &[ext("tt0460681", "Сверхъестественное", true)]).unwrap();
    app.db
        .with(|c| {
            for s in 1..=3 {
                c.execute(
                    "INSERT INTO ext_seasons VALUES('tt0460681',?,?)",
                    rusqlite::params![s, fsfy::util::now_ms()],
                )?;
            }
            for (s, e) in [(0, 1), (1, 1), (1, 2), (2, 1), (2, 2), (3, 1), (3, 2)] {
                c.execute(
                    "INSERT INTO ext_episodes(ext_id,season,episode) VALUES('tt0460681',?,?)",
                    rusqlite::params![s, e],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let r =
        host.send("PUT", "/api/discover/title/tt0460681/seen", json!({"season": 3, "episode": 1, "value": true})).await;
    assert!(r.status().is_success(), "{}", r.status());
    let seen: Vec<(i32, i32)> = app
        .db
        .with(|c| {
            let mut st = c.prepare("SELECT season, episode FROM ext_seen WHERE ext_id='tt0460681' ORDER BY 1, 2")?;
            let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            r.collect()
        })
        .unwrap();
    assert_eq!(
        seen,
        vec![(1, 1), (1, 2), (2, 1), (2, 2), (3, 1)],
        "прошлые сезоны - тоже, спецвыпуски и следующие - нет"
    );
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_import_and_priority_are_shared_and_viewer_cannot_change_them() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    fsfy::discover::upsert(&running.app, &[ext("tt1234567", "Тестовый фильм", false)]).unwrap();
    let public = format!("http://127.0.0.1:{}", running.app.ports.0);
    let mut viewer = Client::new(public);
    assert!(
        viewer
            .send(
                "POST",
                "/api/auth/register",
                json!({"consent": 1,"name":"Зритель", "login":"prioritytest", "password":"пароль-12345"})
            )
            .await
            .status()
            .is_success()
    );
    let payload = json!({"extId":"tt1234567", "url":"https://example.com/video.mp4"});
    assert_eq!(viewer.send("POST", "/api/admin/video-source", payload.clone()).await.status(), 403);
    let r = host.send("POST", "/api/admin/video-source", payload).await;
    assert!(r.status().is_success(), "{}", r.status());
    assert_eq!(r.json::<Value>().await.unwrap()["count"], 1);
    let links = viewer.json("/api/links?extId=tt1234567").await;
    let id = links[0]["id"].as_i64().unwrap();
    assert_eq!(links[0]["preferred"], true);
    assert_eq!(viewer.send("PUT", &format!("/api/links/{id}/priority"), json!({})).await.status(), 403);
    assert!(host.send("PUT", &format!("/api/links/{id}/priority"), json!({})).await.status().is_success());
    assert_eq!(
        host.send("POST", "/api/admin/video-source", json!({"extId":"tt1234567", "url":"http://127.0.0.1/a.mp4"}))
            .await
            .status(),
        400
    );
    running.shutdown().await;
}

/// Состояние источников видео: зрителю - без подробностей, администратору - с числом ошибок.
#[tokio::test(flavor = "multi_thread")]
async fn sources_status_for_viewers_and_health_for_managers() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    let public = format!("http://127.0.0.1:{}", running.app.ports.0);
    let mut anna = Client::new(public);
    anna.send(
        "POST",
        "/api/auth/register",
        json!({"consent": 1,"name": "Анна", "login": "anna", "password": "пароль-12345"}),
    )
    .await;
    let list = anna.json("/api/sources/status").await;
    let sites: Vec<&str> = list.as_array().unwrap().iter().map(|s| s["site"].as_str().unwrap()).collect();
    assert_eq!(sites, ["lordfilm", "rutube", "vk", "archive", "peertube", "commons"]);
    assert_eq!(list[0]["label"], "LordFilm");
    assert!(["ok", "down", "unknown"].contains(&list[0]["state"].as_str().unwrap()));
    assert!(list[0].get("fails").is_none(), "зрителю - без подробностей");
    let health = host.json("/api/admin/health").await;
    assert!(health["sources"][0].get("fails").is_some(), "администратору - с числом ошибок");
    assert_eq!(anna.get("/api/sources/status").await.status(), 200);
    running.shutdown().await;
}

/// «Ещё не вышел»: отметку ставит и снимает только администратор, у зрителя - 403.
#[tokio::test(flavor = "multi_thread")]
async fn manager_marks_feed_film_as_not_released_yet() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    fsfy::discover::upsert(&running.app, &[ext("tt0000777", "Будущий фильм", false)]).unwrap();
    let flag = |app: &fsfy::App| -> bool {
        app.db.with(|c| c.query_row("SELECT soon FROM ext_titles WHERE id = 'tt0000777'", [], |r| r.get(0))).unwrap()
    };
    assert!(!flag(&running.app));
    let public = format!("http://127.0.0.1:{}", running.app.ports.0);
    let mut anna = Client::new(public);
    anna.send(
        "POST",
        "/api/auth/register",
        json!({"consent": 1,"name": "Анна", "login": "anna", "password": "пароль-12345"}),
    )
    .await;
    let r = anna.send("PUT", "/api/admin/ext/tt0000777/soon", json!({"on": true})).await;
    assert_eq!(r.status(), 403, "зрителю - нельзя");
    assert!(!flag(&running.app));
    let r = host.send("PUT", "/api/admin/ext/tt0000777/soon", json!({"on": true})).await;
    assert!(r.status().is_success());
    assert!(flag(&running.app));
    // Обновление ленты IMDb отметку не стирает.
    fsfy::discover::upsert(&running.app, &[ext("tt0000777", "Будущий фильм", false)]).unwrap();
    assert!(flag(&running.app));
    assert!(host.send("PUT", "/api/admin/ext/tt0000777/soon", json!({"on": false})).await.status().is_success());
    assert!(!flag(&running.app));
    assert_eq!(host.send("PUT", "/api/admin/ext/tt9999999/soon", json!({"on": true})).await.status(), 404);
    running.shutdown().await;
}

/// Фон и обложка, выбранные хостом среди картинок IMDb, не заменяются своими картинками IMDb и обновлением ленты.
#[tokio::test(flavor = "multi_thread")]
async fn picked_feed_art_survives_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    let id = "tt0000888";
    fsfy::discover::upsert(&running.app, &[ext(id, "Выбранный фон", false)]).unwrap();
    let imdb_back = fsfy::media_meta::art("https://m.media-amazon.com/images/imdb-back.jpg");
    let extra = json!({"imdbBackdrop": imdb_back, "poster": imdb_back}).to_string();
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "UPDATE ext_titles SET detailed = ?1, extra = ?2, backdrop = ?3, poster = 'old.jpg' WHERE id = ?4",
                rusqlite::params![fsfy::discover::DETAILED, extra, imdb_back, id],
            )
        })
        .unwrap();
    let page = host.json(&format!("/api/discover/title/{id}")).await;
    assert_eq!(page["backdrop"], imdb_back, "пока ничего не выбрано - картинка IMDb");
    // Хост выбрал: то, что записывает `set_ext_imdb_image` (скачивание из сети в тесте не нужно).
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "UPDATE ext_titles SET backdrop = 'mine.jpg', poster = 'mine-p.jpg',
                 extra = json_set(extra, '$.picked.backdrop', 'mine.jpg', '$.picked.poster', 'mine-p.jpg') WHERE id = ?1",
                [id],
            )
        })
        .unwrap();
    let page = host.json(&format!("/api/discover/title/{id}")).await;
    assert_eq!(page["backdrop"], "/img/mine.jpg");
    assert_eq!(page["posterLarge"], "/img/mine-p.jpg");
    // Обновление ленты с другим постером обложку не сбрасывает.
    let mut again = ext(id, "Выбранный фон", false);
    again.poster_url = Some("https://m.media-amazon.com/images/other.jpg".into());
    fsfy::discover::upsert(&running.app, &[again]).unwrap();
    let kept: Option<String> = running
        .app
        .db
        .with(|c| c.query_row("SELECT poster FROM ext_titles WHERE id = ?1", [id], |r| r.get(0)))
        .unwrap();
    assert_eq!(kept.as_deref(), Some("mine-p.jpg"));
    running.shutdown().await;
}

/// Режиссёр на карточке своего фильма - по-русски: из кэша имён, иначе транслитерация.
#[tokio::test(flavor = "multi_thread")]
async fn own_card_director_is_russian() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO person_names (id, ru, source, at, orig) VALUES ('nm2467416', 'Дэмьен Шазелл', 'wikidata', 0, 'Damien Chazelle')",
                [],
            )
        })
        .unwrap();
    let mut ids = vec![];
    for (key, director) in [("la-la", "Damien Chazelle"), ("other", "Kyle Chandler")] {
        let people = json!([{"name": director, "job": "director"}]);
        let id: i64 = running
            .app
            .db
            .with(|c| {
                c.execute(
                    "INSERT INTO titles(kind,key,title,people,added_at,updated_at) VALUES('movie',?,?,?,0,0)",
                    rusqlite::params![key, key, people.to_string()],
                )?;
                let id = c.last_insert_rowid();
                c.execute(
                    "INSERT INTO files(title_id,path,size,mtime,added_at) VALUES(?,?,1048576,0,0)",
                    rusqlite::params![id, format!("{key}.mp4")],
                )?;
                Ok(id)
            })
            .unwrap();
        ids.push(id);
    }
    let a = host.json(&format!("/api/titles/{}", ids[0])).await;
    assert_eq!(a["card"]["director"], "Дэмьен Шазелл", "из кэша");
    let b = host.json(&format!("/api/titles/{}", ids[1])).await;
    assert_eq!(b["card"]["director"], "Кайл Чандлер", "транслитерация без кэша");
    running.shutdown().await;
}

/// Первая загрузка «Обзора» лёгкая: в ряду не больше 20 карточек, вся подборка - отдельным запросом.
#[tokio::test(flavor = "multi_thread")]
async fn feed_rows_are_capped_and_full_chart_stays_available() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    let items: Vec<_> = (0..60).map(|i| ext(&format!("tt90000{i:02}"), &format!("Фильм {i}"), false)).collect();
    fsfy::discover::upsert(&running.app, &items).unwrap();
    running
        .app
        .db
        .with(|c| {
            for i in 0..60 {
                c.execute(
                    "INSERT INTO ext_charts VALUES ('top_movies', ?, ?)",
                    rusqlite::params![i, format!("tt90000{i:02}")],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let feed = host.json("/api/discover").await;
    let row = feed["rows"].as_array().unwrap().iter().find(|r| r["key"] == "top_movies").expect("ряд");
    assert_eq!(row["items"].as_array().unwrap().len(), 20, "в ряду - первые 20");
    assert_eq!(row["items"][0]["title"], "Фильм 0", "порядок подборки сохранён");
    let full = host.json("/api/discover/chart/top_movies").await;
    assert_eq!(full["items"].as_array().unwrap().len(), 60, "вся подборка - отдельным запросом");
    running.shutdown().await;
}

/// Фильма из ленты ещё нет в базе (страницу только открыли): отзывы - пустой список, а не 404.
#[tokio::test(flavor = "multi_thread")]
async fn ext_reviews_of_unknown_title_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    let r = host.get("/api/discover/title/tt0000001/reviews").await;
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["items"], json!([]));
    assert_eq!(v["votes"], 0);
    assert_eq!(host.get("/api/discover/title/bad/reviews").await.status(), 404, "неверный id - по-прежнему 404");
    running.shutdown().await;
}

/// Имена людей и персонажей на страницах фильма - по-русски, в базе остаются оригинальные.
#[tokio::test(flavor = "multi_thread")]
async fn people_on_film_pages_are_russian() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    let app = running.app.clone();
    app.db.set_setting("names.wikidata.sparql", "").unwrap();
    app.db
        .with(|c| {
            c.execute(
                "INSERT INTO person_names (id, ru, source, at) VALUES ('nm0331516', 'Райан Гослинг', 'wikidata', 0)",
                [],
            )
        })
        .unwrap();
    let people = json!([
        {"name": "Ryan Gosling", "job": "actor", "role": "Neil Armstrong", "id": "nm0331516"},
        {"name": "Kyle Chandler", "job": "actor", "role": "Deke Slayton", "id": "nm0151419"},
    ]);

    // Фильм из ленты: люди лежат в `ext_titles.extra`.
    fsfy::discover::upsert(&app, &[ext("tt2543164", "Первый человек", false)]).unwrap();
    app.db
        .with(|c| {
            c.execute(
                "UPDATE ext_titles SET extra = ?, detailed = ? WHERE id = 'tt2543164'",
                rusqlite::params![json!({"people": people}).to_string(), fsfy::discover::DETAILED],
            )
        })
        .unwrap();
    let page = host.json("/api/discover/title/tt2543164").await;
    assert_eq!(page["people"][0]["name"], "Райан Гослинг");
    assert_eq!(page["people"][0]["orig"], "Ryan Gosling");
    assert_eq!(page["people"][0]["role"], "Нил Армстронг");
    assert_eq!(page["people"][1]["name"], "Кайл Чандлер");
    let stored: String =
        app.db.with(|c| c.query_row("SELECT extra FROM ext_titles WHERE id = 'tt2543164'", [], |r| r.get(0))).unwrap();
    assert!(stored.contains("Ryan Gosling"), "в базе - оригинал");

    // Свой фильм: люди лежат в `titles.people`.
    let id: i64 = app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO titles(kind,key,title,people,added_at,updated_at) VALUES('movie','first-man','Первый человек',?,0,0)",
                [people.to_string()],
            )?;
            let id = c.last_insert_rowid();
            c.execute(
                "INSERT INTO files(title_id,path,size,mtime,added_at) VALUES(?,'first-man.mp4',1048576,0,0)",
                [id],
            )?;
            Ok(id)
        })
        .unwrap();
    let own = host.json(&format!("/api/titles/{id}")).await;
    assert_eq!(own["people"][0]["name"], "Райан Гослинг");
    assert_eq!(own["people"][0]["orig"], "Ryan Gosling");
    assert_eq!(own["people"][1]["role"], "Дик Слейтон");
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires live series page and external player"]
async fn admin_imports_real_series_playlist() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    fsfy::discover::upsert(&running.app, &[ext("tt1234567", "Дюна: Пророчество", true)]).unwrap();
    let response = host
        .send(
            "POST",
            "/api/admin/video-source",
            json!({"extId":"tt1234567", "url":"https://sg.lordfilm5.pro/serialy/51890-djuna-prorochestvo-2024.html"}),
        )
        .await;
    assert!(response.status().is_success(), "{}", response.status());
    assert!(response.json::<Value>().await.unwrap()["count"].as_u64().unwrap() > 1);
    let links = host.json("/api/links?extId=tt1234567").await;
    assert!(
        links
            .as_array()
            .unwrap()
            .iter()
            .any(|link| link["season"] == 1 && link["episode"] == 2 && link["preferred"] == true)
    );
    running.shutdown().await;
}
