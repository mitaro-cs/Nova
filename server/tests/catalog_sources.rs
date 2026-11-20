mod common;
use serde_json::{Value, json};

fn catalog(domain: &str) -> Value {
    json!({"domain":domain,"name":domain,"kind":"all","enabled":true,"search":null,"manifest":null})
}

fn manifest() -> Value {
    json!({"version":1,"items":[
        {"imdbId":"tt7654321","title":"Тестовый фильм","kind":"movie","year":2020,"streams":[
            {"id":"ru-4k","url":"https://media.example.com/4k.mp4","label":"Фильм","quality":"2160p","voice":"Русский"},
            {"id":"en-4k","url":"https://media.example.com/en.m3u8","label":"Фильм","quality":"2160p","voice":"Оригинал"},
            {"id":"ru-hd","url":"https://media.example.com/hd.mp4","label":"Фильм","quality":"1080p","voice":"Русский"}]},
        {"imdbId":"tt7654322","title":"Тестовый сериал","kind":"series","streams":[
            {"id":"s1e1","url":"https://media.example.com/1.m3u8","label":"Серия 1","season":1,"episode":1},
            {"id":"s1e2","url":"https://media.example.com/2.m3u8","label":"Серия 2","season":1,"episode":2},
            {"id":"s2e1","url":"https://media.example.com/3.m3u8","label":"Серия 1","season":2,"episode":1}]}]})
}

#[tokio::test(flavor = "multi_thread")]
async fn imports_every_variant_orders_pins_and_disables_sources() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let a = catalog("a.example.com");
    let b = catalog("b.example.com");
    assert!(host.send("PUT", "/api/admin/catalogs", json!({"items":[a,b]})).await.status().is_success());
    for domain in ["a.example.com", "b.example.com"] {
        let r = host.send("POST", "/api/admin/catalogs/import", json!({"domain":domain,"manifest":manifest()})).await;
        assert!(r.status().is_success(), "{}", r.text().await.unwrap());
    }
    let r =
        host.send("POST", "/api/admin/catalogs/import", json!({"domain":"a.example.com","manifest":manifest()})).await;
    let counts: Value = r.json().await.unwrap();
    assert_eq!(counts["added"], 0);
    assert_eq!(counts["updated"], 6);
    let links = host.json("/api/links?extId=tt7654321").await;
    assert_eq!(links.as_array().unwrap().len(), 6);
    assert!(links.as_array().unwrap()[..3].iter().all(|l| l["catalogDomain"] == "a.example.com"));
    let labels: Vec<_> = links.as_array().unwrap().iter().map(|l| l["label"].as_str().unwrap()).collect();
    assert!(labels.iter().any(|l| l.contains("2160p") && l.contains("Оригинал")));
    assert!(labels.iter().any(|l| l.contains("1080p") && l.contains("Русский")));
    let pinned = links.as_array().unwrap().iter().find(|l| l["catalogDomain"] == "b.example.com").unwrap()["id"]
        .as_i64()
        .unwrap();
    assert!(host.send("PUT", &format!("/api/links/{pinned}/priority"), json!({})).await.status().is_success());
    assert_eq!(host.json("/api/links?extId=tt7654321").await[0]["id"], pinned);
    let series = host.json("/api/links?extId=tt7654322").await;
    let first = series.as_array().unwrap().iter().find(|l| l["season"] == 1 && l["episode"] == 1).unwrap()["id"]
        .as_i64()
        .unwrap();
    let next = host.json(&format!("/api/links/{first}/next")).await["next"]["id"].as_i64().unwrap();
    let next_link = host.json(&format!("/api/links/{next}")).await;
    assert_eq!(next_link["episode"], 2);
    let third = host.json(&format!("/api/links/{next}/next")).await["next"]["id"].as_i64().unwrap();
    assert_eq!(host.json(&format!("/api/links/{third}")).await["season"], 2);
    let feed = host.json("/api/discover").await;
    // В ленту попадает только IMDb: импортированное с сайтов (чужие названия и годы) подборкой не показывается.
    assert!(
        !feed["rows"].as_array().unwrap().iter().any(|r| r["key"] == "catalog_movies" || r["key"] == "catalog_series")
    );
    let mut off = b.clone();
    off["enabled"] = json!(false);
    assert!(host.send("PUT", "/api/admin/catalogs", json!({"items":[off,a]})).await.status().is_success());
    let links = host.json("/api/links?extId=tt7654321").await;
    assert_eq!(links.as_array().unwrap().len(), 3);
    for path in [
        format!("/api/links/{pinned}"),
        format!("/api/links/{pinned}/stream/source"),
        format!("/api/links/{pinned}/stream/index.m3u8"),
    ] {
        assert_eq!(host.get(&path).await.json::<Value>().await.unwrap()["error"], "source_disabled");
    }
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn import_is_atomic_validates_urls_types_and_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    assert!(
        host.send("PUT", "/api/admin/catalogs", json!({"items":[catalog("a.example.com")]}))
            .await
            .status()
            .is_success()
    );
    let mut bad = manifest();
    bad["items"][1]["streams"][0]["url"] = json!("http://127.0.0.1/private.mp4");
    assert_eq!(
        host.send("POST", "/api/admin/catalogs/import", json!({"domain":"a.example.com","manifest":bad}))
            .await
            .status(),
        400
    );
    assert!(host.json("/api/links?extId=tt7654321").await.as_array().unwrap().is_empty());
    let count: i64 = running
        .app
        .db
        .with(|c| c.query_row("SELECT COUNT(*) FROM ext_titles WHERE id='tt7654321'", [], |r| r.get(0)))
        .unwrap();
    assert_eq!(count, 0);
    for field in ["unsupported", "subtitles"] {
        let mut bad = manifest();
        bad["items"][0]["streams"][0][field] = json!([]);
        assert_eq!(
            host.send("POST", "/api/admin/catalogs/import", json!({"domain":"a.example.com","manifest":bad}))
                .await
                .status(),
            422
        );
    }
    let mut bad = manifest();
    bad["items"][0]["streams"][1]["id"] = json!("ru-4k");
    assert_eq!(
        host.send("POST", "/api/admin/catalogs/import", json!({"domain":"a.example.com","manifest":bad}))
            .await
            .status(),
        400
    );
    let mut viewer = common::Client::new(format!("http://127.0.0.1:{}", running.app.ports.0));
    assert!(
        viewer
            .send(
                "POST",
                "/api/auth/register",
                json!({"consent": 1,"name":"Зритель","login":"catalogviewer","password":"пароль-12345"})
            )
            .await
            .status()
            .is_success()
    );
    assert_eq!(
        viewer
            .send("POST", "/api/admin/catalogs/import", json!({"domain":"a.example.com","manifest":manifest()}))
            .await
            .status(),
        403
    );
    let mut bad_catalog = catalog("a.example.com");
    bad_catalog["manifest"] = json!("http://169.254.169.254/secret");
    assert_eq!(host.send("PUT", "/api/admin/catalogs", json!({"items":[bad_catalog]})).await.status(), 400);
    running.shutdown().await;
}
