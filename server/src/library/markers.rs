//! Заставка и титры без глав в файле - по кадрам (просьба владельца, 24.11: «пропустить вступление и титры - наладь»).
//!
//! Титры: конец фильма (последние 12 минут, не больше трети) по ключевым кадрам 64×36 в оттенках серого. Кадр
//! «похож на титры», если он почти весь тёмный (буквы на чёрном или чёрный экран). Начало титров - самый ранний такой
//! кадр, после которого до конца не меньше 40 с и 85 % кадров тоже «титры»: сцена после титров не мешает, а тёмная
//! сцена в середине фильма не проходит. Берём ключевой кадр, а не точное время - кнопка появится на секунды позже,
//! но никогда не раньше начала титров.
//!
//! Заставка сериала: отпечатки начала серии (первые 8 минут, кадр в секунду, разностный хеш 9×8) у соседних серий
//! сезона. Одинаковая заставка - длинный ряд совпадающих кадров с одним сдвигом: у этой серии с 0:52 по 2:10, у
//! соседней с 1:30 по 2:48. Пустые (одноцветные) кадры не считаются: чёрный экран есть в каждой серии.
//!
//! ffmpeg - фоновый (nice 19 / IDLE, один поток), только для того, что начали смотреть, один раз на файл.

use super::probe;
use crate::App;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

/// Сколько секунд начала серии сравниваем (и не больше трети серии).
const HEAD: f64 = 480.0;
/// Сколько секунд конца смотрим в поисках титров (и не больше трети фильма).
const TAIL: f64 = 720.0;
const TW: usize = 64;
const TH: usize = 36;

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Auto {
    pub intro: Option<(f64, f64)>,
    pub credits: Option<f64>,
}

pub fn parse(json: Option<&str>) -> Auto {
    json.and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default()
}

/// Кадр похож на титры: не меньше 80 % точек тёмные, ярких (буквы) не больше 15 %.
pub fn credits_like(gray: &[u8]) -> bool {
    if gray.is_empty() {
        return false;
    }
    let n = gray.len() as f64;
    let dark = gray.iter().filter(|&&p| p < 48).count() as f64;
    let bright = gray.iter().filter(|&&p| p > 150).count() as f64;
    dark / n >= 0.80 && bright / n <= 0.15
}

/// Начало титров по кадрам конца: (время, похож на титры), время по возрастанию; `end` - длина видео.
pub fn credits_start(frames: &[(f64, bool)], end: f64) -> Option<f64> {
    let n = frames.len();
    // Сколько «титров» от i до конца - суффиксные суммы, чтобы не считать заново.
    let mut tail = vec![0usize; n + 1];
    for i in (0..n).rev() {
        tail[i] = tail[i + 1] + usize::from(frames[i].1);
    }
    (0..n).find_map(|i| {
        let (t, like) = frames[i];
        let share = tail[i] as f64 / (n - i) as f64;
        (like && end - t >= 40.0 && n - i >= 4 && share >= 0.85).then_some(t)
    })
}

/// Разностный хеш 64 бит по картинке 9×8 и «пустой ли кадр» (почти один цвет - не сравниваем).
pub fn dhash(px: &[u8]) -> Option<u64> {
    if px.len() != 72 {
        return None;
    }
    let mean = px.iter().map(|&p| p as f64).sum::<f64>() / 72.0;
    let var = px.iter().map(|&p| (p as f64 - mean).powi(2)).sum::<f64>() / 72.0;
    if var.sqrt() < 8.0 {
        return None;
    }
    let mut h = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            h = (h << 1) | u64::from(px[y * 9 + x] > px[y * 9 + x + 1]);
        }
    }
    Some(h)
}

/// Отпечатки начала серии (кадр в секунду) в BLOB: по 8 байт, пустой кадр - 0.
pub fn pack(hashes: &[Option<u64>]) -> Vec<u8> {
    hashes.iter().flat_map(|h| h.unwrap_or(0).to_le_bytes()).collect()
}

pub fn unpack(blob: &[u8]) -> Vec<Option<u64>> {
    let (chunks, _) = blob.as_chunks::<8>();
    chunks.iter().map(|c| Some(u64::from_le_bytes(*c)).filter(|&h| h != 0)).collect()
}

/// Общая заставка двух серий: самый длинный ряд совпадающих кадров (допуск - 3 пропуска подряд) при одном сдвиге.
/// Вернуть (с, по) в первой серии и (с, по) во второй - секунды. Короче 15 с или длиннее 3 минут - не заставка;
/// заставка начинается в первые 6 минут.
pub fn common_intro(a: &[Option<u64>], b: &[Option<u64>]) -> Option<((f64, f64), (f64, f64))> {
    let same = |x: Option<u64>, y: Option<u64>| matches!((x, y), (Some(x), Some(y)) if (x ^ y).count_ones() <= 10);
    let (n, m) = (a.len() as i64, b.len() as i64);
    let mut best: Option<(i64, i64, i64)> = None; // (сдвиг, начало в a, конец в a включительно)
    for d in -(m - 1)..n {
        let (mut start, mut last, mut gap, mut hits) = (-1i64, -1i64, 0i64, 0i64);
        let lo = d.max(0);
        let hi = n.min(m + d);
        let mut i = lo;
        while i <= hi {
            let hit = i < hi && same(a[i as usize], b[(i - d) as usize]);
            if hit {
                if start < 0 {
                    start = i;
                    hits = 0;
                }
                last = i;
                gap = 0;
                hits += 1;
            } else if start >= 0 {
                gap += 1;
                if gap > 3 || i == hi {
                    let len = last - start + 1;
                    // Ряд засчитываем, только если совпала хотя бы половина кадров в нём.
                    if hits * 2 >= len && best.is_none_or(|(_, s, e)| len > e - s + 1) {
                        best = Some((d, start, last));
                    }
                    start = -1;
                }
            }
            i += 1;
        }
    }
    let (d, s, e) = best?;
    let len = (e - s + 1) as f64;
    if !(15.0..=180.0).contains(&len) || s > 360 || s - d > 360 {
        return None;
    }
    Some(((s as f64, (e + 1) as f64), ((s - d) as f64, (e + 1 - d) as f64)))
}

/// Кадры начала: кадр в секунду, 9×8 серым.
pub async fn head_hashes(ffmpeg: &Path, file: &Path, span: f64) -> anyhow::Result<Vec<Option<u64>>> {
    let out = tokio::time::timeout(
        Duration::from_secs(900),
        probe::background(ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-threads", "1", "-t", &format!("{span:.0}"), "-i"])
            .arg(file)
            .args(["-an", "-sn", "-dn", "-vf", "fps=1,scale=9:8:flags=area,format=gray", "-f", "rawvideo", "pipe:1"])
            .stderr(Stdio::null())
            .output(),
    )
    .await??;
    anyhow::ensure!(out.status.success(), "ffmpeg не прочитал начало");
    let (chunks, _) = out.stdout.as_chunks::<72>();
    Ok(chunks.iter().map(|c| dhash(c)).collect())
}

/// Ключевые кадры конца 64×36 серым и их время (из `showinfo`).
pub async fn tail_frames(ffmpeg: &Path, file: &Path, from: f64) -> anyhow::Result<Vec<(f64, bool)>> {
    let out = tokio::time::timeout(
        Duration::from_secs(900),
        probe::background(ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "info",
                "-threads",
                "1",
                "-skip_frame",
                "nokey",
                "-ss",
                &format!("{from:.2}"),
                "-i",
            ])
            .arg(file)
            .args(["-an", "-sn", "-dn", "-copyts", "-fps_mode", "passthrough", "-vf"])
            .arg(format!("scale={TW}:{TH}:flags=area,format=gray,showinfo"))
            .args(["-f", "rawvideo", "pipe:1"])
            .stderr(Stdio::piped())
            .output(),
    )
    .await??;
    anyhow::ensure!(out.status.success(), "ffmpeg не прочитал конец");
    let times: Vec<f64> = String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(|l| l.split("pts_time:").nth(1)?.split_whitespace().next()?.parse().ok())
        .collect();
    let (chunks, _) = out.stdout.as_chunks::<{ TW * TH }>();
    Ok(times.into_iter().zip(chunks).map(|(t, c)| (t, credits_like(c))).collect())
}

/// Файл в очереди: id, путь, ffprobe, фильм, вид, сезон.
type Next = (i64, String, Option<String>, i64, String, Option<i32>);

/// Следующий начатый файл без найденных отметок: отпечатки начала (сериал) и титры. true - что-то сделано.
pub async fn auto_one(app: &Arc<App>, ffmpeg: &Path) -> bool {
    let next: Option<Next> = app
        .db
        .with(|c| {
            c.query_row(
                "SELECT f.id, f.path, f.probe, f.title_id, t.kind, f.season FROM files f JOIN titles t ON t.id = f.title_id
                 WHERE f.auto_markers IS NULL AND f.missing = 0 AND f.probe IS NOT NULL
                   AND EXISTS (SELECT 1 FROM progress p JOIN files pf ON pf.id = p.file_id WHERE pf.title_id = f.title_id)
                 ORDER BY (SELECT MAX(p.updated_at) FROM progress p JOIN files pf ON pf.id = p.file_id WHERE pf.title_id = f.title_id) DESC,
                          f.season, f.episode LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .optional()
        })
        .ok()
        .flatten();
    let Some((id, path, probe, title, kind, season)) = next else { return false };
    let p: probe::Probe = probe.and_then(|p| serde_json::from_str(&p).ok()).unwrap_or_default();
    let file = Path::new(&path);
    let mut found = Auto::default();
    let mut head = None;
    if let (Some(_), Some(d)) = (&p.video, p.duration)
        && d >= 300.0
    {
        let from = d - TAIL.min(d / 3.0);
        if let Ok(frames) = tail_frames(ffmpeg, file, from).await {
            found.credits = credits_start(&frames, d).filter(|&c| c > d * 0.6);
        }
        if kind == "series"
            && let Ok(h) = head_hashes(ffmpeg, file, HEAD.min(d / 3.0)).await
        {
            head = Some(pack(&h));
        }
    }
    let json = serde_json::to_string(&found).unwrap_or_else(|_| "{}".into());
    let _ = app
        .db
        .with(|c| c.execute("UPDATE files SET auto_markers = ?, head_hash = ? WHERE id = ?", params![json, head, id]));
    if kind == "series" {
        match_season(app, title, season);
    }
    true
}

/// Заставки сезона: каждую серию с отпечатками сравниваем с соседними (по номеру) и берём самый длинный общий ряд.
pub fn match_season(app: &App, title: i64, season: Option<i32>) {
    let rows: Vec<(i64, Vec<u8>, Option<String>)> = app
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT id, head_hash, auto_markers FROM files WHERE title_id = ? AND COALESCE(season, 0) = ?
                   AND head_hash IS NOT NULL AND missing = 0 ORDER BY episode, id",
            )?;
            let r = st.query_map(params![title, season.unwrap_or(0)], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            r.collect()
        })
        .unwrap_or_default();
    let prints: Vec<Vec<Option<u64>>> = rows.iter().map(|r| unpack(&r.1)).collect();
    for (i, (id, _, auto)) in rows.iter().enumerate() {
        let mut best: Option<(f64, f64)> = None;
        for j in [i.wrapping_sub(1), i + 1, i.wrapping_sub(2), i + 2] {
            let Some(other) = prints.get(j) else { continue };
            if let Some((mine, _)) = common_intro(&prints[i], other)
                && best.is_none_or(|b| mine.1 - mine.0 > b.1 - b.0)
            {
                best = Some(mine);
            }
        }
        let mut a = parse(auto.as_deref());
        if a.intro != best {
            a.intro = best;
            let json = serde_json::to_string(&a).unwrap_or_default();
            let _ = app.db.with(|c| c.execute("UPDATE files SET auto_markers = ? WHERE id = ?", params![json, id]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credits_need_a_long_dark_run_to_the_end() {
        // Фильм 6000 с: тёмная сцена на 5300-5320, титры с 5500 до конца, сцена после титров 5900-5910.
        let frames: Vec<(f64, bool)> = (5280..6000)
            .step_by(5)
            .map(|t| (t as f64, (5300..5320).contains(&t) || (t >= 5500 && !(5900..5910).contains(&t))))
            .collect();
        assert_eq!(credits_start(&frames, 6000.0), Some(5500.0));
        // Нет титров - нет и отметки.
        let none: Vec<(f64, bool)> = (5000..6000).step_by(5).map(|t| (t as f64, false)).collect();
        assert_eq!(credits_start(&none, 6000.0), None);
    }

    #[test]
    fn dark_frame_with_letters_is_credits() {
        let mut f = vec![5u8; TW * TH];
        f[..100].fill(230);
        assert!(credits_like(&f));
        assert!(!credits_like(&vec![120u8; TW * TH]));
    }

    #[test]
    fn same_intro_found_at_different_offsets() {
        // Псевдослучайные кадры; заставка (40 с) - одни и те же кадры в первой серии с 50 с, во второй с 90 с.
        let mut seed = 7u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            Some(seed | 1)
        };
        let intro: Vec<Option<u64>> = (0..40).map(|_| rnd()).collect();
        let mut a: Vec<Option<u64>> = (0..50).map(|_| rnd()).collect();
        a.extend(&intro);
        a.extend((0..200).map(|_| rnd()));
        let mut b: Vec<Option<u64>> = (0..90).map(|_| rnd()).collect();
        b.extend(&intro);
        b.extend((0..150).map(|_| rnd()));
        // Один кадр заставки во второй серии чуть другой (сжатие) - ряд не рвётся.
        b[100] = b[100].map(|h| h ^ 0b111);
        assert_eq!(common_intro(&a, &b), Some(((50.0, 90.0), (90.0, 130.0))));
        // Пустые кадры (чёрный экран) не совпадают.
        let black = vec![None; 120];
        assert_eq!(common_intro(&black, &black), None);
        assert_eq!(unpack(&pack(&a)), a);
    }

    #[test]
    fn flat_frame_has_no_hash() {
        assert_eq!(dhash(&[40u8; 72]), None);
        let px: Vec<u8> = (0..72).map(|i| (i * 3) as u8).collect();
        assert!(dhash(&px).is_some());
    }
}
