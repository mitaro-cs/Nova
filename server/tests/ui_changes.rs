mod common;
use common::Client;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn names_and_logins_report_conflicts_and_do_not_replace_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let public = format!("http://127.0.0.1:{}", running.app.ports.0);
    let mut first = Client::new(public.clone());
    assert!(first
        .send("POST", "/api/auth/register", json!({"name":"Омар","login":"omar-ui","password":"пароль123"}))
        .await
        .status()
        .is_success());
    let mut second = Client::new(public.clone());
    let r = second
        .send("POST", "/api/auth/register", json!({"name":"  ОМАР  ","login":"another-ui","password":"пароль123"}))
        .await;
    assert_eq!(r.status(), 409);
    assert_eq!(r.json::<serde_json::Value>().await.unwrap()["error"], "name_taken");
    let r = second
        .send("POST", "/api/auth/register", json!({"name":"Другое имя","login":"OMAR-UI","password":"пароль123"}))
        .await;
    assert_eq!(r.status(), 409);
    assert_eq!(r.json::<serde_json::Value>().await.unwrap()["error"], "login_taken");
    let available = second.json("/api/auth/availability?login=omar-ui&name=%D0%9E%D0%BC%D0%B0%D1%80").await;
    assert_eq!(available, json!({"loginTaken":true,"nameTaken":true}));
    assert!(second
        .send("POST", "/api/auth/register", json!({"name":"Анна UI","login":"anna-ui","password":"пароль123"}))
        .await
        .status()
        .is_success());
    assert_eq!(second.send("PATCH", "/api/me", json!({"name":"омар"})).await.status(), 409);
    assert_eq!(second.json("/api/me").await["user"]["name"], "Анна UI");
    assert_eq!(
        host.send("POST", "/api/admin/users", json!({"name":"Омар","login":"third-ui","password":"пароль123"}))
            .await
            .status(),
        409
    );
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn custom_images_are_stored_as_persistent_choices_and_require_manager() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    running.app.db.with(|c|c.execute("INSERT INTO ext_titles(id,kind,title,detailed,extra,updated_at) VALUES('tt9458304','series','Dororo',?1,'{}',0)",[fsfy::discover::DETAILED])).unwrap();
    if !common::ffmpeg() {
        running.shutdown().await;
        return;
    }
    let output = std::process::Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=32x48",
            "-frames:v",
            "1",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "pipe:1",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let bytes = output.stdout;
    for which in ["poster", "backdrop"] {
        let response = host
            .http
            .put(format!("{}/api/admin/ext/tt9458304/image/{which}", host.base))
            .header("cookie", &host.cookie)
            .header("x-fsfy", "1")
            .body(bytes.clone())
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "{}", response.status());
        let body = response.json::<serde_json::Value>().await.unwrap();
        let url = body["url"].as_str().unwrap();
        assert!(host.get(url).await.status().is_success());
        let name = running
            .app
            .db
            .with(|c| {
                c.query_row(
                    &format!("SELECT json_extract(extra,'$.picked.{which}') FROM ext_titles WHERE id='tt9458304'"),
                    [],
                    |r| r.get::<_, String>(0),
                )
            })
            .unwrap();
        assert_eq!(url, format!("/img/{name}"));
    }
    let detail = host.json("/api/discover/title/tt9458304").await;
    assert_eq!(detail["card"]["title"], "Дороро");
    let r = host
        .http
        .put(format!("{}/api/admin/ext/tt9458304/image/poster", host.base))
        .header("cookie", &host.cookie)
        .header("x-fsfy", "1")
        .body("не картинка")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 400);
    let mut viewer = Client::new(format!("http://127.0.0.1:{}", running.app.ports.0));
    assert!(viewer
        .send("POST", "/api/auth/register", json!({"name":"Зритель UI","login":"viewer-ui","password":"пароль123"}))
        .await
        .status()
        .is_success());
    let r = viewer
        .http
        .put(format!("{}/api/admin/ext/tt9458304/image/poster", viewer.base))
        .header("cookie", &viewer.cookie)
        .header("x-fsfy", "1")
        .body(bytes)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "Измерение выборки полного каталога и SQL LIMIT на 5000 тайтлов"]
async fn catalog_selection_benchmark() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    running
        .app
        .db
        .tx(|c| {
            for i in 0..5000 {
                c.execute(
                    "INSERT INTO ext_titles(id,kind,title,updated_at) VALUES(?1,'movie',?2,0)",
                    rusqlite::params![format!("tt{}", 8000000 + i), format!("Фильм {i:05}")],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let profile = host.json("/api/me").await["user"]["profile"]["id"].as_i64().unwrap();
    for (label, limit) in [("До: полная выборка", ""), ("После: двадцать карточек", " LIMIT 20")]
    {
        let start = std::time::Instant::now();
        let count = running
            .app
            .db
            .with(|c| {
                let mut st = c.prepare(&format!("{} ORDER BY e.title{limit}", fsfy::discover::card_sql(false)))?;
                let rows = st.query_map([profile], fsfy::discover::card)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .len();
        println!("{label}: {count} карточек, {:.2} мс", start.elapsed().as_secs_f64() * 1000.0);
    }
    running.shutdown().await;
}
