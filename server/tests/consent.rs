mod common;
use serde_json::{Value, json};

/// Согласие с политикой и правилами: новый аккаунт - только с согласием, прежние принимают при входе.
#[tokio::test(flavor = "multi_thread")]
async fn registration_needs_consent_and_old_accounts_must_accept() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let base = format!("http://127.0.0.1:{}", running.app.ports.0);
    let version = host.json("/api/site").await["legalVersion"].as_i64().unwrap();
    assert!(version >= 1);

    // Без согласия аккаунт не создаётся, ни без поля, ни со старой редакцией.
    let mut guest = common::Client::new(base.clone());
    let body = json!({"name": "Новый", "login": "newbie", "password": "пароль-12345"});
    let r = guest.send("POST", "/api/auth/register", body.clone()).await;
    assert_eq!(r.status(), 400);
    assert_eq!(r.json::<Value>().await.unwrap()["error"], "consent_required");
    let mut stale = body.clone();
    stale["consent"] = json!(version - 1);
    assert_eq!(guest.send("POST", "/api/auth/register", stale).await.status(), 400);
    let n: i64 = running
        .app
        .db
        .with(|c| c.query_row("SELECT COUNT(*) FROM users WHERE login = 'newbie'", [], |r| r.get(0)))
        .unwrap();
    assert_eq!(n, 0);

    // С согласием - создаётся, и согласие записано.
    let mut ok = body.clone();
    ok["consent"] = json!(version);
    assert!(guest.send("POST", "/api/auth/register", ok).await.status().is_success());
    let me = guest.json("/api/me").await;
    assert_eq!(me["legal"]["needed"], false);
    let (v, at): (i64, Option<i64>) = running
        .app
        .db
        .with(|c| {
            c.query_row("SELECT consent_version, consent_at FROM users WHERE login = 'newbie'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
        })
        .unwrap();
    assert_eq!(v, version);
    assert!(at.is_some());

    // Прежний аккаунт (заведён хостом, не принимал): должен принять; чужую редакцию принять нельзя.
    assert!(
        host.send(
            "POST",
            "/api/admin/users",
            json!({"login": "old", "name": "Старый", "password": "пароль-12345", "role": "viewer"})
        )
        .await
        .status()
        .is_success()
    );
    let mut old = common::Client::new(base);
    assert!(
        old.send("POST", "/api/auth/login", json!({"login": "old", "password": "пароль-12345"}))
            .await
            .status()
            .is_success()
    );
    assert_eq!(old.json("/api/me").await["legal"]["needed"], true);
    assert_eq!(old.send("POST", "/api/me/consent", json!({"version": version + 1})).await.status(), 400);
    assert_eq!(old.json("/api/me").await["legal"]["needed"], true);
    assert!(old.send("POST", "/api/me/consent", json!({"version": version})).await.status().is_success());
    assert_eq!(old.json("/api/me").await["legal"]["needed"], false);
}
