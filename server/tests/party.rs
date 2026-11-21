mod common;
use serde_json::{Value, json};

struct Events {
    response: reqwest::Response,
    buffer: String,
}
impl Events {
    async fn event(&mut self, kind: &str) -> Value {
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                if let Some(end) = self.buffer.find("\n\n") {
                    let packet = self.buffer[..end].to_string();
                    self.buffer.drain(..end + 2);
                    if let Some(data) = packet.lines().find_map(|l| l.strip_prefix("data: ")) {
                        let v: Value = serde_json::from_str(data).unwrap();
                        if v["type"] == kind {
                            return v;
                        }
                    }
                } else {
                    let b = self.response.chunk().await.unwrap().expect("SSE ended");
                    self.buffer.push_str(&String::from_utf8_lossy(&b));
                }
            }
        })
        .await
        .expect("room event did not arrive")
    }
}

/// Создатель «позвал» человека: в тестах - прямо в комнате, без дружбы (она проверяется отдельно).
async fn invite(app: &fsfy::App, room: &str, user: &mut common::Client, name: &str) {
    user.get("/api/me").await; // профиль появляется с первым запросом
    let id: i64 = app
        .db
        .with(|c| c.query_row("SELECT id FROM profiles WHERE name = ?", [name], |r| r.get(0)))
        .unwrap_or_else(|e| panic!("профиль «{name}»: {e}"));
    app.rooms.with(room, |r| r.invited.insert(id)).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn ten_viewers_receive_ordered_controls_and_rejoin_at_current_position() {
    exercise_viewers(10).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn fifty_active_viewers_receive_controls_and_query_without_errors() {
    exercise_viewers(50).await;
}

async fn exercise_viewers(count: usize) {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let media = dir.path().join("range-test.mp4");
    std::fs::write(&media, vec![42u8; 1024 * 1024]).unwrap();
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO titles(id,kind,key,title,added_at,updated_at) VALUES(9900,'movie','load-test','Load',0,0)",
                [],
            )?;
            c.execute(
                "INSERT INTO files(id,title_id,path,size,mtime,added_at) VALUES(9900,9900,?,1048576,0,0)",
                [media.to_string_lossy().as_ref()],
            )?;
            Ok(())
        })
        .unwrap();
    let link = host
        .send("POST", "/api/links", json!({"url":"https://example.com/party.webm","label":"Party","duration":600}))
        .await
        .json::<Value>()
        .await
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let room = host
        .send("POST", "/api/rooms", json!({"linkId":link,"position":120}))
        .await
        .json::<Value>()
        .await
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut clients = Vec::new();
    let mut streams = Vec::new();
    for n in 0..count {
        if n > 0 {
            let path = dir.path().join(format!("range-{n}.mp4"));
            std::fs::copy(&media, &path).unwrap();
            running
                .app
                .db
                .with(|c| {
                    c.execute(
                        "INSERT INTO files(id,title_id,path,size,mtime,added_at) VALUES(?,9900,?,1048576,0,0)",
                        rusqlite::params![9900 + n as i64, path.to_string_lossy().as_ref()],
                    )
                })
                .unwrap();
        }
        let login = format!("party{n}");
        assert!(
            host.send(
                "POST",
                "/api/admin/users",
                json!({"login":login,"name":format!("Зритель {n}"),"password":"PartyPass123","role":"viewer"})
            )
            .await
            .status()
            .is_success()
        );
        let mut user = common::Client::new(format!("http://127.0.0.1:{}", running.app.ports.0));
        assert!(
            user.send("POST", "/api/auth/login", json!({"login":login,"password":"PartyPass123"}))
                .await
                .status()
                .is_success()
        );
        invite(&running.app, &room, &mut user, &format!("Зритель {n}")).await;
        let response = user.get(&format!("/api/rooms/{room}/events")).await;
        let mut stream = Events { response, buffer: String::new() };
        stream.event("snapshot").await;
        clients.push(user);
        streams.push(stream);
    }
    assert_eq!(host.json(&format!("/api/rooms/{room}")).await["members"].as_array().unwrap().len(), count);
    let load_seconds: u64 = std::env::var("NOVA_STREAM_LOAD_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let video_load: Vec<_> = clients
        .iter()
        .enumerate()
        .map(|(index, user)| {
            let http = user.http.clone();
            let cookie = user.cookie.clone();
            let url = format!("{}/api/files/{}/raw", user.base, 9900 + index);
            tokio::spawn(async move {
                let mut timings = Vec::new();
                let expected = vec![42u8; 262144];
                let mut clock = tokio::time::interval(std::time::Duration::from_millis(250));
                // Fixed offered bitrate: catch up after scheduler jitter instead of
                // silently reducing the workload by shifting every subsequent tick.
                clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Burst);
                let started = std::time::Instant::now();
                for _ in 0..load_seconds * 4 {
                    clock.tick().await;
                    let tick = std::time::Instant::now();
                    let response = http
                        .get(&url)
                        .header("cookie", &cookie)
                        .header("range", "bytes=0-262143")
                        .send()
                        .await
                        .unwrap();
                    assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);
                    let bytes = response.bytes().await.unwrap();
                    assert_eq!(bytes.len(), 262144);
                    assert_eq!(bytes.as_ref(), expected.as_slice());
                    timings.push(tick.elapsed().as_secs_f64() * 1000.0);
                }
                (timings, started.elapsed().as_secs_f64())
            })
        })
        .collect();
    let started = std::time::Instant::now();
    let durations = futures_util::future::join_all(clients.iter().map(|user| async {
        let mut times = Vec::new();
        for _ in 0..20 {
            let tick = std::time::Instant::now();
            let value = user.json("/api/badges").await;
            assert_eq!(value["unread"], 0);
            times.push(tick.elapsed().as_secs_f64() * 1000.0);
        }
        for _ in 0..8 {
            let response = user
                .http
                .get(format!("{}/api/files/9900/raw", user.base))
                .header("cookie", &user.cookie)
                .header("range", "bytes=0-262143")
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);
            assert_eq!(response.headers()["content-range"], "bytes 0-262143/1048576");
            let bytes = response.bytes().await.unwrap();
            assert_eq!(bytes.len(), 262144);
            assert!(bytes.iter().all(|b| *b == 42));
        }
        times
    }))
    .await;
    let mut durations: Vec<_> = durations.into_iter().flatten().collect();
    durations.sort_by(f64::total_cmp);
    println!(
        "{count} users, {} requests, {:.2}s, p50 {:.2}ms, p95 {:.2}ms, max {:.2}ms",
        durations.len(),
        started.elapsed().as_secs_f64(),
        durations[durations.len() / 2],
        durations[durations.len() * 95 / 100],
        durations.last().unwrap()
    );
    for n in 0..20usize {
        let position = 120.0 + n as f64;
        let playing = n % 2 == 0;
        let state = clients[n % count]
            .send("POST", &format!("/api/rooms/{room}/state"), json!({"playing":playing,"position":position}))
            .await
            .json::<Value>()
            .await
            .unwrap();
        let events = futures_util::future::join_all(streams.iter_mut().map(|s| s.event("state"))).await;
        for event in events {
            assert_eq!(event["state"]["revision"], state["revision"]);
            assert_eq!(event["state"]["playing"], playing);
            assert_eq!(event["state"]["position"], position);
        }
    }
    // Десять участников нажимают одновременно: сервер задаёт единый порядок,
    // который видят все клиенты, независимо от порядка ответов HTTP.
    let writes = futures_util::future::join_all(clients.iter_mut().enumerate().map(|(n, user)| {
        let endpoint = format!("/api/rooms/{room}/state");
        async move { user.send("POST", &endpoint, json!({"playing":false,"position":200+n})).await }
    }))
    .await;
    let mut ordered = Vec::new();
    for response in writes {
        assert!(response.status().is_success());
        ordered.push(response.json::<Value>().await.unwrap());
    }
    ordered.sort_by_key(|state| state["revision"].as_u64().unwrap());
    for state in &ordered {
        let events = futures_util::future::join_all(streams.iter_mut().map(|s| s.event("state"))).await;
        for event in events {
            assert_eq!(event["state"]["revision"], state["revision"]);
            assert_eq!(event["state"]["position"], state["position"]);
        }
    }
    let reconnect = clients[0].get(&format!("/api/rooms/{room}/events")).await;
    let mut rejoined = Events { response: reconnect, buffer: String::new() };
    let snapshot = rejoined.event("snapshot").await;
    assert_eq!(snapshot["state"]["position"], ordered.last().unwrap()["position"]);
    assert_eq!(snapshot["state"]["revision"], 20 + count);
    let mut segment_times = Vec::new();
    let mut longest = 0.0_f64;
    for task in video_load {
        let (times, elapsed) = task.await.unwrap();
        segment_times.extend(times);
        longest = longest.max(elapsed);
    }
    if !segment_times.is_empty() {
        segment_times.sort_by(f64::total_cmp);
        println!(
            "Sustained {count} streams: {} ranges, {:.1} MiB, {:.2}s, {:.1} Mbps total, range p95 {:.2}ms, max {:.2}ms",
            segment_times.len(),
            segment_times.len() as f64 / 4.0,
            longest,
            segment_times.len() as f64 * 262144.0 * 8.0 / longest / 1_000_000.0,
            segment_times[segment_times.len() * 95 / 100],
            segment_times.last().unwrap()
        );
    }
    drop(rejoined);
    drop(streams);
    running.app.db.with(|c| c.execute("UPDATE users SET disabled=1 WHERE login='party0'", [])).unwrap();
    assert_eq!(clients[0].get("/api/me").await.status(), reqwest::StatusCode::UNAUTHORIZED);
    running.shutdown().await;
}

/// Смотрели вместе: у обоих «смотреть дальше» с места комнаты, даже если плеер гостя ничего не прислал.
#[tokio::test(flavor = "multi_thread")]
async fn room_place_is_remembered_for_everyone_in_the_room() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let media = dir.path().join("together.mp4");
    std::fs::write(&media, vec![1u8; 4096]).unwrap();
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO titles(id,kind,key,title,added_at,updated_at) VALUES(9910,'movie','together','Together',0,0)",
                [],
            )?;
            c.execute(
                "INSERT INTO files(id,title_id,path,size,mtime,added_at,duration) VALUES(9910,9910,?,4096,0,0,6000)",
                [media.to_string_lossy().as_ref()],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(
        host.send(
            "POST",
            "/api/admin/users",
            json!({"login":"guest","name":"Гость","password":"PartyPass123","role":"viewer"})
        )
        .await
        .status()
        .is_success()
    );
    let mut guest = common::Client::new(format!("http://127.0.0.1:{}", running.app.ports.0));
    assert!(
        guest
            .send("POST", "/api/auth/login", json!({"login":"guest","password":"PartyPass123"}))
            .await
            .status()
            .is_success()
    );
    let room = host.send("POST", "/api/rooms", json!({"fileId":9910})).await.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    invite(&running.app, &room, &mut guest, "Гость").await;
    let mut host_events =
        Events { response: host.get(&format!("/api/rooms/{room}/events")).await, buffer: String::new() };
    host_events.event("snapshot").await;
    let mut guest_events =
        Events { response: guest.get(&format!("/api/rooms/{room}/events")).await, buffer: String::new() };
    guest_events.event("snapshot").await;
    let count = |app: &fsfy::App| -> Vec<f64> {
        app.db
            .with(|c| {
                let mut s = c.prepare("SELECT position FROM progress WHERE file_id = 9910 ORDER BY profile_id")?;

                s.query_map([], |r| r.get::<_, f64>(0))?.collect::<Result<Vec<_>, _>>()
            })
            .unwrap()
    };
    assert!(count(&running.app).is_empty());
    // Хост ставит паузу на 20-й минуте: место появляется у обоих (гость ничего не слал сам).
    assert!(
        host.send("POST", &format!("/api/rooms/{room}/state"), json!({"playing":false,"position":1200}))
            .await
            .status()
            .is_success()
    );
    assert_eq!(count(&running.app), vec![1200.0, 1200.0]);
    // Начало (первые секунды) - не «смотреть дальше».
    assert!(
        guest
            .send("POST", &format!("/api/rooms/{room}/state"), json!({"playing":false,"position":2}))
            .await
            .status()
            .is_success()
    );
    assert_eq!(count(&running.app), vec![1200.0, 1200.0]);
    // Гость закрыл вкладку: его место - там, где комната была в ту минуту (на паузе - то же).
    assert!(
        host.send("POST", &format!("/api/rooms/{room}/state"), json!({"playing":false,"position":1500}))
            .await
            .status()
            .is_success()
    );
    assert_eq!(count(&running.app), vec![1500.0, 1500.0]);
    drop(guest_events);
    drop(host_events);
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn play_link_does_not_wait_for_remote_thumbnail_and_cloudpub_url_survives_offline() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let started = std::time::Instant::now();
    let response=host.send("POST","/api/links",json!({"url":"https://rutube.ru/video/0123456789abcdef0123456789abcdef/","label":"Ready now","thumb":"https://example.com/slow-thumbnail.jpg"})).await;
    assert!(response.status().is_success());
    assert!(started.elapsed() < std::time::Duration::from_millis(700));
    running.app.db.set_setting("access.mode", "cloudpub").unwrap();
    running.app.db.set_setting("access.cp.url", "https://saved-address.cloudpub.ru").unwrap();
    let view = running.app.access.view(&running.app);
    assert_eq!(view.public_url.as_deref(), Some("https://saved-address.cloudpub.ru"));
    running.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn room_is_closed_to_strangers_and_queue_belongs_to_the_owner() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let base = format!("http://127.0.0.1:{}", running.app.ports.0);
    let media = |n: i64| dir.path().join(format!("q{n}.mp4"));
    for n in [9950, 9951] {
        std::fs::write(media(n), vec![1u8; 1024]).unwrap();
    }
    running
        .app
        .db
        .with(|c| {
            for (id, name) in [(9950, "Первый"), (9951, "Второй")] {
                c.execute(
                    "INSERT INTO titles(id,kind,key,title,added_at,updated_at) VALUES(?1,'movie',?2,?2,0,0)",
                    rusqlite::params![id, name],
                )?;
                c.execute(
                    "INSERT INTO files(id,title_id,path,size,mtime,added_at) VALUES(?1,?1,?2,1024,0,0)",
                    rusqlite::params![id, media(id).to_string_lossy().as_ref()],
                )?;
            }
            Ok(())
        })
        .unwrap();
    let mut people = Vec::new();
    for (login, name) in [("friend", "Друг"), ("stranger", "Чужой")] {
        assert!(
            host.send(
                "POST",
                "/api/admin/users",
                json!({"login":login,"name":name,"password":"PartyPass123","role":"viewer"})
            )
            .await
            .status()
            .is_success()
        );
        let mut user = common::Client::new(base.clone());
        assert!(
            user.send("POST", "/api/auth/login", json!({"login":login,"password":"PartyPass123"}))
                .await
                .status()
                .is_success()
        );
        people.push(user);
    }
    let mut stranger = people.pop().unwrap();
    let mut friend = people.pop().unwrap();
    let room = host.send("POST", "/api/rooms", json!({"fileId":9950})).await.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let url = |tail: &str| format!("/api/rooms/{room}{tail}");

    // Код комнаты чужого не пускает: ни снимок, ни чат, ни управление, ни очередь.
    assert_eq!(stranger.get(&url("")).await.status(), 403);
    assert_eq!(stranger.send("POST", &url("/chat"), json!({"body":"привет"})).await.status(), 403);
    assert_eq!(stranger.send("POST", &url("/state"), json!({"playing":true,"position":1})).await.status(), 403);
    assert_eq!(stranger.send("POST", &url("/queue"), json!({"fileId":9951})).await.status(), 403);
    assert_eq!(stranger.get(&url("/events")).await.status(), 403);

    // Позвал - вошёл. Звать может только создатель.
    invite(&running.app, &room, &mut friend, "Друг").await;
    assert_eq!(friend.get(&url("")).await.status(), 200);
    assert_eq!(friend.send("POST", &url("/invite"), json!({"profileId":1})).await.status(), 403);

    // Гость может лишь предложить; в очереди оно ждёт создателя.
    let suggested = friend.send("POST", &url("/queue"), json!({"fileId":9951})).await.json::<Value>().await.unwrap();
    assert_eq!(suggested["approved"], false);
    let id = suggested["id"].as_u64().unwrap();
    assert_eq!(friend.send("POST", &url(&format!("/queue/{id}/approve")), json!({})).await.status(), 403);
    assert_eq!(friend.send("PUT", &url("/queue"), json!({"ids":[id]})).await.status(), 403);
    assert_eq!(friend.send("POST", &url(&format!("/queue/{id}/play")), json!({})).await.status(), 403);
    // Непринятое включить нельзя и создателю.
    assert_eq!(host.send("POST", &url(&format!("/queue/{id}/play")), json!({})).await.status(), 404);

    assert!(host.send("POST", &url(&format!("/queue/{id}/approve")), json!({})).await.status().is_success());
    // Порядок: второй выше первого.
    assert!(host.send("PUT", &url("/queue"), json!({"ids":[id, 1]})).await.status().is_success());
    let snap = host.json(&url("")).await;
    assert_eq!(snap["queue"][0]["id"], id);
    assert_eq!(snap["queue"][0]["label"], "Второй");
    assert_eq!(snap["queue"][0]["source"], "Nova");

    // Включил из очереди - у всех другой фильм и место с нуля.
    assert!(host.send("POST", &url(&format!("/queue/{id}/play")), json!({})).await.status().is_success());
    let snap = friend.json(&url("")).await;
    assert_eq!(snap["titleId"], 9951);
    assert_eq!(snap["current"], id);
    assert_eq!(snap["state"]["fileId"], 9951);

    // Дальше по очереди: после последнего - конец.
    let next = host.send("POST", &url("/queue/next"), json!({})).await.json::<Value>().await.unwrap();
    assert_eq!(next["ended"], false);
    assert_eq!(next["id"], 1);
    let end = host.send("POST", &url("/queue/next"), json!({})).await.json::<Value>().await.unwrap();
    assert_eq!(end["ended"], true);

    // Своё предложение гость убирает сам, чужое - нет.
    let own = friend.send("POST", &url("/queue"), json!({"fileId":9950})).await.json::<Value>().await.unwrap();
    let own = own["id"].as_u64().unwrap();
    assert_eq!(friend.send("DELETE", &url(&format!("/queue/{id}")), json!({})).await.status(), 403);
    assert!(friend.send("DELETE", &url(&format!("/queue/{own}")), json!({})).await.status().is_success());
    // Хост зовёт любого зрителя сервера, не только друга: после этого чужой входит.
    let id: i64 =
        running.app.db.with(|c| c.query_row("SELECT id FROM profiles WHERE name = 'Чужой'", [], |r| r.get(0))).unwrap();
    assert!(host.send("POST", &url("/invite"), json!({"profileId": id})).await.status().is_success());
    assert_eq!(stranger.get(&url("")).await.status(), 200);
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_room_starts_with_the_first_video_in_the_queue() {
    // Сначала комната, потом видео (21.11): «Комната по ссылке» создаёт пустую, первое добавленное хозяином включается.
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let file = dir.path().join("e.mp4");
    std::fs::write(&file, vec![1u8; 1024]).unwrap();
    running
        .app
        .db
        .with(|c| {
            c.execute(
                "INSERT INTO titles(id,kind,key,title,added_at,updated_at) VALUES(9960,'movie','Пустая','Пустая',0,0)",
                [],
            )?;
            c.execute(
                "INSERT INTO files(id,title_id,path,size,mtime,added_at) VALUES(9960,9960,?1,1024,0,0)",
                [file.to_string_lossy().as_ref()],
            )?;
            Ok(())
        })
        .unwrap();
    let res = host.send("POST", "/api/rooms", json!({})).await;
    assert!(res.status().is_success(), "пустая комната создаётся");
    let room = res.json::<Value>().await.unwrap()["id"].as_str().unwrap().to_owned();
    let info = host.json(&format!("/api/rooms/{room}")).await;
    assert!(info["current"].is_null());
    assert_eq!(info["queue"], json!([]));
    assert!(host.send("POST", &format!("/api/rooms/{room}/queue"), json!({"fileId":9960})).await.status().is_success());
    let info = host.json(&format!("/api/rooms/{room}")).await;
    assert_eq!(info["state"]["fileId"], 9960, "первое видео хозяина сразу текущее");
    assert_eq!(info["state"]["playing"], false);
    running.shutdown().await;
}
