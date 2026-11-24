//! Заставка и титры по кадрам на настоящих видео (ffmpeg): титры - тёмный экран с полосой текста в конце фильма,
//! заставка - один и тот же ролик в двух сериях на разном месте. Без ffmpeg тест пропускается.

mod common;

use common::{ffmpeg, until};
use fsfy::library::markers;
use serde_json::json;
use std::path::Path;
use std::process::Command;

fn run(args: &[&str]) {
    let out = Command::new("ffmpeg").args(["-y", "-loglevel", "error"]).args(args).output().unwrap();
    assert!(out.status.success(), "ffmpeg: {}", String::from_utf8_lossy(&out.stderr));
}

const ENC: [&str; 8] = ["-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", "-g", "50"];

/// Серия: «жизнь» со своим зерном `pre` секунд, общая заставка, потом снова «жизнь».
fn episode(out: &Path, intro: &Path, pre: u32, seed: u32) {
    let a = format!("life=s=320x180:rate=10:seed={seed}:ratio=0.4:mold=10,format=yuv420p,trim=duration={pre}");
    let b = format!("life=s=320x180:rate=10:seed={}:ratio=0.4:mold=10,format=yuv420p,trim=duration=120", seed + 99);
    let mut args: Vec<String> = ["-f", "lavfi", "-i", &a, "-i"].iter().map(|s| s.to_string()).collect();
    args.push(intro.to_string_lossy().into());
    args.extend(["-f", "lavfi", "-i", &b].iter().map(|s| s.to_string()));
    args.extend(
        [
            "-filter_complex",
            "[0:v]setsar=1[a];[1:v]setsar=1,fps=10[i];[2:v]setsar=1[b];[a][i][b]concat=n=3:v=1:a=0[v]",
            "-map",
            "[v]",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    args.extend(ENC.iter().map(|s| s.to_string()));
    args.push(out.to_string_lossy().into());
    run(&args.iter().map(String::as_str).collect::<Vec<_>>());
}

/// Фильм 500 с: 400 с картинки, потом 100 с титров - чёрный экран с бегущей белой строкой.
fn make_movie(out: &Path) {
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let m = out.to_string_lossy();
    run(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc2=s=320x180:r=10:d=400",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=320x180:r=10:d=100,drawbox=x=90:y='170-t':w=140:h=6:color=white:t=fill",
        "-filter_complex",
        "[0:v][1:v]concat=n=2:v=1:a=0[v]",
        "-map",
        "[v]",
        ENC[0],
        ENC[1],
        ENC[2],
        ENC[3],
        ENC[4],
        ENC[5],
        ENC[6],
        ENC[7],
        &m,
    ]);
}

#[tokio::test]
async fn credits_and_intro_found_by_frames() {
    if !ffmpeg() {
        eprintln!("нет ffmpeg - пропуск");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let bin = Path::new("ffmpeg");

    // Фильм: 400 с картинки, потом 100 с титров (чёрный экран, бегущая белая строка).
    let movie = dir.path().join("movie.mp4");
    make_movie(&movie);
    let frames = markers::tail_frames(bin, &movie, 500.0 - 160.0).await.unwrap();
    assert!(frames.len() >= 10, "ключевые кадры конца: {}", frames.len());
    let credits = markers::credits_start(&frames, 500.0).expect("титры найдены");
    assert!((400.0..=406.0).contains(&credits), "титры с {credits}");

    // Две серии: одна заставка (40 с) с 30-й секунды в первой и с 60-й во второй.
    let intro = dir.path().join("intro.mp4");
    run(&[
        "-f",
        "lavfi",
        "-i",
        "testsrc2=s=320x180:r=10:d=40",
        ENC[0],
        ENC[1],
        ENC[2],
        ENC[3],
        ENC[4],
        ENC[5],
        &intro.to_string_lossy(),
    ]);
    let (e1, e2) = (dir.path().join("e1.mp4"), dir.path().join("e2.mp4"));
    episode(&e1, &intro, 30, 1);
    episode(&e2, &intro, 60, 7);
    let a = markers::head_hashes(bin, &e1, 200.0).await.unwrap();
    let b = markers::head_hashes(bin, &e2, 200.0).await.unwrap();
    let ((s1, f1), (s2, f2)) = markers::common_intro(&a, &b).expect("заставка найдена");
    assert!((29.0..=31.0).contains(&s1) && (69.0..=71.0).contains(&f1), "первая серия: {s1}-{f1}");
    assert!((59.0..=61.0).contains(&s2) && (99.0..=101.0).contains(&f2), "вторая серия: {s2}-{f2}");
}

/// Сквозь сервер: начали смотреть - фоновая задача сама находит титры, плеер получает их в `markers`, а закрытый на
/// титрах фильм сразу «Просмотрено».
#[tokio::test(flavor = "multi_thread")]
async fn started_movie_gets_credits_and_is_watched_on_them() {
    if !ffmpeg() {
        eprintln!("нет ffmpeg - пропуск");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let media = dir.path().join("media");
    make_movie(&media.join("Титры (2020)/titry.2020.mp4"));
    let (_running, mut host) = common::start(dir.path()).await;
    host.send("POST", "/api/admin/libraries", json!({"path": media.to_str().unwrap()})).await;
    assert!(until(|| async { host.json("/api/titles").await.as_array().unwrap().len() == 1 }).await);
    assert!(until(|| async { host.json("/api/admin/status").await["unprobed"] == 0 }).await);
    let title = host.json("/api/titles").await[0]["id"].as_i64().unwrap();
    let file = host.json(&format!("/api/titles/{title}")).await["files"][0]["id"].as_i64().unwrap();
    assert_eq!(host.json(&format!("/api/files/{file}/info")).await["markers"]["credits"], serde_json::Value::Null);
    host.send("PUT", &format!("/api/progress/{file}"), json!({"position": 10.0, "duration": 500.0})).await;
    // Обход, ffprobe, кадры перемотки, потом титры - не дольше 90 с.
    let mut credits = 0.0;
    for _ in 0..900 {
        credits = host.json(&format!("/api/files/{file}/info")).await["markers"]["credits"].as_f64().unwrap_or(0.0);
        if credits > 0.0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!((400.0..=406.0).contains(&credits), "титры с {credits}");
    let r: serde_json::Value = host
        .send("PUT", &format!("/api/progress/{file}"), json!({"position": credits + 1.0, "duration": 500.0}))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(
        (r["done"].clone(), r["watched"].clone()),
        (json!(true), json!(true)),
        "закрыли на титрах - просмотрено"
    );
}
