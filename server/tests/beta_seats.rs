mod common;
use serde_json::{Value, json};

/// Места беты: когда все заняты, регистрация не проходит (ни открытая, ни по приглашению); хост заводит вручную,
/// удаление освобождает место; 0 - без лимита.
#[tokio::test(flavor = "multi_thread")]
async fn beta_seats_close_registration_when_full() {
    let dir = tempfile::tempdir().unwrap();
    let (running, mut host) = common::start(dir.path()).await;
    let base = format!("http://127.0.0.1:{}", running.app.ports.0);
    let version = host.json("/api/site").await["legalVersion"].as_i64().unwrap();
    // По умолчанию - 100 мест, никто не занял.
    let site = host.json("/api/site").await;
    assert_eq!(site["seats"]["total"], 100);
    assert_eq!(site["seats"]["used"], 0);
    assert!(host.send("PUT", "/api/admin/settings", json!({"betaSeats": 2})).await.status().is_success());
    assert_eq!(host.json("/api/admin/settings").await["betaSeats"], 2);
    assert_eq!(host.send("PUT", "/api/admin/settings", json!({"betaSeats": -1})).await.status(), 400);

    let reg = |login: &str| json!({"name": login, "login": login, "password": "пароль-12345", "consent": version});
    for login in ["seat1", "seat2"] {
        let mut c = common::Client::new(base.clone());
        assert!(c.send("POST", "/api/auth/register", reg(login)).await.status().is_success(), "{login}");
    }
    let mut late = common::Client::new(base.clone());
    let r = late.send("POST", "/api/auth/register", reg("seat3")).await;
    assert_eq!(r.status(), 403);
    let body: Value = r.json().await.unwrap();
    assert_eq!(body["error"], "seats_full");
    assert!(body["message"].as_str().unwrap().contains("2 мест"));
    let status = host.json("/api/admin/status").await;
    assert_eq!(status["seats"]["used"], 2);
    assert_eq!(status["seats"]["left"], 0);
    assert_eq!(host.json("/api/site").await["seats"]["left"], 0);

    // Хост может завести вручную даже сверх лимита.
    assert!(
        host.send(
            "POST",
            "/api/admin/users",
            json!({"login": "byhost", "name": "Гость", "password": "пароль-12345", "role": "viewer"})
        )
        .await
        .status()
        .is_success()
    );
    // Без лимита - снова можно.
    assert!(host.send("PUT", "/api/admin/settings", json!({"betaSeats": 0})).await.status().is_success());
    assert!(host.json("/api/site").await["seats"].is_null());
    assert!(late.send("POST", "/api/auth/register", reg("seat3")).await.status().is_success());
}
