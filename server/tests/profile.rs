//! Личный профиль: новая статистика (когда смотрит, дни недели, серия дней, оценки, эпохи), приватность разделов
//! («все / друзья / только я»), любимые актёры и актрисы.

mod common;

use common::Client;
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread")]
async fn person_suggestions_use_russian_and_find_original_names() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    running.app.db.with(|c| c.execute(
        "INSERT INTO person_names(id,ru,source,at,orig) VALUES ('nm9999999','Мартин Скорсезе','wikidata',1,'Martin Scorsese')",
        [],
    )).unwrap();
    for query in ["Скорсезе", "Scorsese"] {
        let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        let result = host.json(&format!("/api/people/suggest?q={encoded}")).await;
        assert_eq!(result, json!(["Мартин Скорсезе"]));
    }
    running.shutdown().await;
}

fn ext(id: &str, title: &str, year: i32) -> fsfy::discover::Ext {
    fsfy::discover::Ext {
        id: id.into(),
        series: false,
        title: title.into(),
        original: None,
        year: Some(year),
        rating: Some(8.0),
        votes: Some(1000),
        genres: vec!["Драма".into()],
        poster_url: None,
        release: None,
    }
}

async fn signup(public: &str, login: &str) -> (Client, i64) {
    let mut c = Client::new(public.to_string());
    let r = c
        .send(
            "POST",
            "/api/auth/register",
            json!({"consent": 1,"name": login, "login": login, "password": "пароль-12345"}),
        )
        .await;
    assert!(r.status().is_success(), "регистрация {login}: {}", r.status());
    let id = c.json("/api/me").await["user"]["profile"]["id"].as_i64().unwrap();
    (c, id)
}

#[tokio::test(flavor = "multi_thread")]
async fn new_profile_gets_a_character_and_keeps_its_choice() {
    let dir = tempfile::tempdir().unwrap();
    let (running, host) = common::start(dir.path()).await;
    let public = format!("http://127.0.0.1:{}", running.app.ports.0);
    let (mut person, id) = signup(&public, "character-viewer").await;
    let me = person.json("/api/me").await;
    let avatar = me["user"]["profile"]["avatar"].as_i64().unwrap();
    assert!((0..12).contains(&avatar));
    assert!(me["user"]["profile"]["photo"].is_null());
    assert!(person.send("PATCH", "/api/me", json!({"avatar":8})).await.status().is_success());
    assert_eq!(person.json("/api/me").await["user"]["profile"]["avatar"], 8);
    assert_eq!(host.json(&format!("/api/members/{id}")).await["person"]["avatar"], 8);
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stats_widgets_and_privacy_levels() {
    let dir = tempfile::tempdir().unwrap();
    let (running, _host) = common::start(dir.path()).await;
    let app = running.app.clone();
    let public = format!("http://127.0.0.1:{}", app.ports.0);
    fsfy::discover::upsert(&app, &[ext("tt0017136", "Метрополис", 1927), ext("tt1979320", "Гонка", 2013)]).unwrap();

    let (mut anna, pa) = signup(&public, "anna").await;
    let (mut boris, pb) = signup(&public, "boris").await;
    let (vera, _) = signup(&public, "vera").await;

    anna.send("PUT", "/api/discover/title/tt0017136/mark", json!({"status": "watched"})).await;
    anna.send("PUT", "/api/discover/title/tt1979320/mark", json!({"status": "watched"})).await;
    anna.send("POST", "/api/discover/title/tt0017136/reviews", json!({"stars": 9, "body": "Город будущего"})).await;
    anna.send("POST", "/api/discover/title/tt1979320/reviews", json!({"stars": 7, "body": ""})).await;
    // Просмотры в известное время: воскресенье вечером (21:30 по UTC); на следующий день ещё один.
    let tue = 1_825_000_000_000_i64 - (1_825_000_000_000_i64 % 86_400_000) + 21 * 3_600_000 + 30 * 60_000;
    app.db
        .with(|c| {
            c.execute(
                "UPDATE ext_marks SET at = ?1 WHERE profile_id = ?2 AND ext_id = 'tt0017136'",
                rusqlite::params![tue, pa],
            )?;
            c.execute(
                "UPDATE ext_marks SET at = ?1 WHERE profile_id = ?2 AND ext_id = 'tt1979320'",
                rusqlite::params![tue + 86_400_000, pa],
            )
        })
        .unwrap();

    // Статистика: оценки, эпохи, дни; без уровней - видят все.
    let page = boris.json(&format!("/api/members/{pa}?tz=0")).await;
    assert_eq!(page["hidden"], json!({"stats": false, "watched": false, "reviews": false}));
    let s = &page["stats"];
    assert_eq!(s["ratings"]["count"], 2);
    assert_eq!(s["ratings"]["avg"], 8.0, "(9 + 7) / 2");
    assert_eq!(s["ratings"]["hist"][8], 1);
    assert_eq!(s["ratings"]["hist"][6], 1);
    let eras: Vec<(i64, i64)> = s["decades"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| (d["decade"].as_i64().unwrap(), d["count"].as_i64().unwrap()))
        .collect();
    assert_eq!(eras, [(1920, 1), (2010, 1)]);
    assert_eq!(s["dayparts"].as_array().unwrap().len(), 4);
    assert_eq!(s["weekdays"].as_array().unwrap().len(), 7);
    assert_eq!(s["streak"]["best"], 2, "два дня подряд");
    assert_eq!(s["streak"]["current"], 0, "сегодня и вчера не смотрел");
    // 1 825 000 000 с - воскресенье (индекс 6, понедельник = 0), следующие сутки - понедельник (0); вечер (21:30 UTC) - третья корзина.
    assert_eq!(s["weekdays"][6], 1);
    assert_eq!(s["weekdays"][0], 1);
    assert_eq!(s["dayparts"][2], 2);

    // Свой пояс: сдвиг на +4 часа переносит просмотр на следующие сутки и в ночь.
    let a = anna.json(&format!("/api/members/{pa}?tz=0")).await;
    let b = anna.json(&format!("/api/members/{pa}?tz=240")).await;
    assert_ne!(a["stats"]["dayparts"], b["stats"]["dayparts"], "часовой пояс меняет «когда смотрит»");
    assert_eq!(a["decor"]["visibility"], json!({"stats": "all", "watched": "all", "reviews": "all"}));
    assert_eq!(
        boris.json(&format!("/api/members/{pa}")).await["decor"]["visibility"],
        Value::Null,
        "чужому настройки не видны"
    );

    // Подружились: boris - друг anna.
    boris.send("POST", &format!("/api/friends/{pa}"), json!({})).await;
    anna.send("POST", &format!("/api/friends/{pb}"), json!({})).await;

    // Приватность: статистика - друзьям, просмотренное - только мне, отзывы - все.
    let r = anna
        .send("PATCH", "/api/me/profile", json!({"visStats": "friends", "visWatched": "me", "visReviews": "all"}))
        .await;
    assert!(r.status().is_success());
    assert_eq!(anna.send("PATCH", "/api/me/profile", json!({"visStats": "secret"})).await.status(), 400);

    let friend = boris.json(&format!("/api/members/{pa}")).await;
    assert_eq!(friend["hidden"], json!({"stats": false, "watched": true, "reviews": false}));
    assert_eq!(friend["watched"], json!([]));
    assert_eq!(friend["decor"]["ribbon"], json!([]), "лента обложки - это просмотренное");
    assert!(friend["stats"].is_object());

    let stranger = vera.json(&format!("/api/members/{pa}")).await;
    assert_eq!(stranger["hidden"], json!({"stats": true, "watched": true, "reviews": false}));
    assert_eq!(stranger["stats"], Value::Null, "числа закрытого раздела сервер не отдаёт");
    assert_eq!(stranger["reviews"].as_array().unwrap().len(), 2, "отзывы открыты");
    assert_eq!(
        vera.get(&format!("/api/people/{pa}/watched?kind=movie")).await.status(),
        403,
        "список просмотренного закрыт"
    );

    // Сам человек видит всё.
    let me = anna.json(&format!("/api/members/{pa}")).await;
    assert!(me["stats"].is_object());
    assert_eq!(me["hidden"], json!({"stats": false, "watched": false, "reviews": false}));
    assert_eq!(me["decor"]["visibility"], json!({"stats": "friends", "watched": "me", "reviews": "all"}));

    // Отзывы «только я»: нет ни в профиле, ни под фильмом у других.
    anna.send("PATCH", "/api/me/profile", json!({"visReviews": "me"})).await;
    assert_eq!(vera.json(&format!("/api/members/{pa}")).await["reviews"], json!([]));
    assert_eq!(boris.json(&format!("/api/members/{pa}")).await["reviews"], json!([]), "друг тоже не видит «только я»");
    let under = vera.json("/api/discover/title/tt0017136/reviews").await;
    assert_eq!(under["items"], json!([]), "и под фильмом чужие закрытые отзывы не видны");
    let own = anna.json("/api/discover/title/tt0017136/reviews").await;
    assert_eq!(own["items"].as_array().unwrap().len(), 1, "свои - всегда");

    // В списке людей закрытые числа - нулём.
    let list = vera.json("/api/members").await;
    let row = list.as_array().unwrap().iter().find(|m| m["id"] == pa).unwrap();
    assert_eq!((row["watched"].as_i64(), row["reviews"].as_i64()), (Some(0), Some(0)));
}

#[tokio::test(flavor = "multi_thread")]
async fn favorite_actors_limit_and_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let (running, _host) = common::start(dir.path()).await;
    let public = format!("http://127.0.0.1:{}", running.app.ports.0);
    let (mut anna, pa) = signup(&public, "anna").await;
    let names = ["Чарли Чаплин", "Кэри Грант", "Одри Хепберн", "Бригитте Хельм", "Макс Шрек"];
    let r = anna.send("PATCH", "/api/me/profile", json!({"actors": names})).await;
    assert!(r.status().is_success(), "{}", r.status());
    let page = anna.json(&format!("/api/members/{pa}")).await;
    let got: Vec<&str> =
        page["decor"]["actors"].as_array().unwrap().iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert_eq!(got, names);

    // Шестой - нельзя; повторы без учёта регистра и пробелов схлопываются; пустые убираются.
    let six: Vec<&str> = names.iter().copied().chain(["Боб Хоуп"]).collect();
    assert_eq!(anna.send("PATCH", "/api/me/profile", json!({"actors": six})).await.status(), 400);
    let r =
        anna.send("PATCH", "/api/me/profile", json!({"actors": ["  Кэри Грант ", "кэри грант", "", "Боб Хоуп"]})).await;
    assert!(r.status().is_success());
    let page = anna.json(&format!("/api/members/{pa}")).await;
    let got: Vec<&str> =
        page["decor"]["actors"].as_array().unwrap().iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert_eq!(got, ["Кэри Грант", "Боб Хоуп"]);
    let long = "я".repeat(81);
    assert_eq!(anna.send("PATCH", "/api/me/profile", json!({"actors": [long]})).await.status(), 400);
}
