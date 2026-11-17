mod common;

use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn imported_and_catalog_playback_do_not_use_personal_link_limit() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let profile = host.json("/api/me").await["user"]["profile"]["id"].as_i64().unwrap();
    running.app.db.tx(|c| {
        c.execute("INSERT INTO ext_titles(id,kind,title,updated_at) VALUES('tt7654325','movie','Фильм',0)", [])?;
        for i in 0..500 {
            c.execute("INSERT INTO links(kind,url,label,added_by,added_at) VALUES('direct',?1,'Импорт',?2,0)", rusqlite::params![format!("https://example.com/import-{i}.mp4"),profile])?;
            c.execute("INSERT INTO catalog_links(link_id,domain,item_key,stream_key) VALUES(?1,'example.com','tt7654325',?2)", rusqlite::params![c.last_insert_rowid(),format!("stream-{i}")])?;
        }
        Ok(())
    }).unwrap();
    let first = host.send("POST", "/api/links", json!({"url":"https://example.com/standalone.mp4"})).await;
    assert!(first.status().is_success(), "импорт не должен расходовать лимит: {}", first.status());
    running
        .app
        .db
        .tx(|c| {
            for i in 1..500 {
                c.execute(
                    "INSERT INTO links(kind,url,label,added_by,added_at) VALUES('direct',?1,'Личная',?2,0)",
                    rusqlite::params![format!("https://example.com/personal-{i}.mp4"), profile],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let personal = host.send("POST", "/api/links", json!({"url":"https://example.com/extra.mp4"})).await;
    assert_eq!(personal.status(), 400);
    let play =
        host.send("POST", "/api/links", json!({"url":"https://example.com/movie.mp4","extId":"tt7654325"})).await;
    assert!(play.status().is_success(), "просмотр тайтла не должен блокироваться: {}", play.status());
    running.shutdown().await;
}
