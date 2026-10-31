//! Журнал приложения: консоль, файл `<данные>/logs/nova.log` (ротация 5 × 2 МБ) и кольцо последних
//! предупреждений и ошибок для «Управление → Обзор». Ни адреса, ни токены в журнал не попадают.

use serde::Serialize;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

const FILE_NAME: &str = "nova.log";
const MAX_BYTES: u64 = 2 * 1024 * 1024;
const KEEP: usize = 5;
const RING: usize = 200;
const MAX_MESSAGE: usize = 500;

#[derive(Debug, Clone, Serialize)]
pub struct Line {
    pub at: i64,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Кольцо последних предупреждений и ошибок.
pub struct Ring {
    cap: usize,
    lines: Mutex<VecDeque<Line>>,
}

impl Ring {
    pub fn new(cap: usize) -> Self {
        Self { cap, lines: Mutex::new(VecDeque::new()) }
    }

    fn push(&self, line: Line) {
        let mut l = self.lines.lock().unwrap_or_else(|e| e.into_inner());
        if l.len() >= self.cap {
            l.pop_front();
        }
        l.push_back(line);
    }

    /// Последние `n` записей, новые первыми.
    pub fn recent(&self, n: usize) -> Vec<Line> {
        let l = self.lines.lock().unwrap_or_else(|e| e.into_inner());
        l.iter().rev().take(n).cloned().collect()
    }
}

fn ring() -> &'static Arc<Ring> {
    static RING_CELL: OnceLock<Arc<Ring>> = OnceLock::new();
    RING_CELL.get_or_init(|| Arc::new(Ring::new(RING)))
}

/// Последние предупреждения и ошибки приложения, новые первыми.
pub fn recent(n: usize) -> Vec<Line> {
    ring().recent(n)
}

/// Убрать из текста адреса и значения секретов, обрезать до 500 знаков.
pub fn sanitize(text: &str) -> String {
    static URL: OnceLock<regex::Regex> = OnceLock::new();
    static SECRET: OnceLock<regex::Regex> = OnceLock::new();
    let url = URL.get_or_init(|| regex::Regex::new(r"(?i)https?://\S+").expect("url pattern"));
    let secret = SECRET
        .get_or_init(|| regex::Regex::new(r"(?i)\b(token|t|password|pass|key|secret)=\S+").expect("secret pattern"));
    let out = url.replace_all(text, "[адрес]");
    let out = secret.replace_all(&out, "$1=[скрыто]");
    out.chars().take(MAX_MESSAGE).collect()
}

/// Сдвинуть файлы журнала, если текущий достиг `max_bytes`: `nova.log` -> `nova.log.1` -> ... Всего не
/// больше `keep` файлов.
pub fn rotate(dir: &Path, max_bytes: u64, keep: usize) -> std::io::Result<()> {
    let current = dir.join(FILE_NAME);
    match fs::metadata(&current) {
        Ok(m) if m.len() >= max_bytes => {}
        _ => return Ok(()),
    }
    let numbered = |i: usize| dir.join(format!("{FILE_NAME}.{i}"));
    let _ = fs::remove_file(numbered(keep.saturating_sub(1).max(1)));
    for i in (1..keep.saturating_sub(1)).rev() {
        if numbered(i).exists() {
            fs::rename(numbered(i), numbered(i + 1))?;
        }
    }
    fs::rename(current, numbered(1))
}

fn stamp(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, sod) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC", sod / 3600, sod % 3600 / 60, sod % 60)
}

struct FileSink {
    dir: PathBuf,
    file: File,
    size: u64,
}

fn sink() -> &'static Mutex<Option<FileSink>> {
    static SINK: OnceLock<Mutex<Option<FileSink>>> = OnceLock::new();
    SINK.get_or_init(|| Mutex::new(None))
}

fn open(dir: &Path) -> std::io::Result<FileSink> {
    rotate(dir, MAX_BYTES, KEEP)?;
    let file = OpenOptions::new().create(true).append(true).open(dir.join(FILE_NAME))?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    Ok(FileSink { dir: dir.to_path_buf(), file, size })
}

/// Начать писать журнал в `<data>/logs/nova.log`. Не вышло (нет прав, диск полон) - молча: журнал
/// остаётся в консоли и кольце.
pub fn attach(data: &Path) {
    let dir = data.join("logs");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    if let Ok(s) = open(&dir) {
        *sink().lock().unwrap_or_else(|e| e.into_inner()) = Some(s);
    }
}

fn write_line(text: &str) {
    let mut guard = sink().lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = guard.as_mut() else { return };
    if s.size + text.len() as u64 >= MAX_BYTES {
        let dir = s.dir.clone();
        match open(&dir) {
            Ok(n) => *s = n,
            Err(_) => {
                *guard = None;
                return;
            }
        }
    }
    let Some(s) = guard.as_mut() else { return };
    if s.file.write_all(text.as_bytes()).is_ok() {
        s.size += text.len() as u64;
    }
}

#[derive(Default)]
struct Collect {
    message: String,
    fields: String,
}

impl Visit for Collect {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }
}

fn describe(event: &Event<'_>) -> (String, String, String) {
    let mut c = Collect::default();
    event.record(&mut c);
    let meta = event.metadata();
    (meta.level().to_string(), meta.target().to_string(), sanitize(&format!("{}{}", c.message, c.fields)))
}

/// Слой: предупреждения и ошибки - в кольцо.
pub struct RingLayer(pub Arc<Ring>);

impl<S: Subscriber> Layer<S> for RingLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if *event.metadata().level() > Level::WARN {
            return;
        }
        let (level, target, message) = describe(event);
        self.0.push(Line { at: crate::util::now_ms(), level, target, message });
    }
}

/// Слой: все события - в файл.
struct FileLayer;

impl<S: Subscriber> Layer<S> for FileLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let (level, target, message) = describe(event);
        write_line(&format!("{} {level:5} {target}: {message}\n", stamp(crate::util::now_ms())));
    }
}

/// Включить журнал: консоль, файл (после `attach`) и кольцо. `RUST_LOG` главнее `default_filter`.
pub fn init(default_filter: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(RingLayer(ring().clone()))
        .with(FileLayer)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn rotates_after_limit() {
        let dir = tempfile::tempdir().unwrap();
        let big = vec![b'x'; 3 * 1024 * 1024];
        for round in 0..8 {
            std::fs::write(dir.path().join("nova.log"), &big).unwrap();
            rotate(dir.path(), 2 * 1024 * 1024, 5).unwrap();
            assert!(!dir.path().join("nova.log").exists(), "round {round}");
            assert!(dir.path().join("nova.log.1").exists());
        }
        let count = std::fs::read_dir(dir.path()).unwrap().count();
        assert!(count <= 5, "files: {count}");
    }

    #[test]
    fn small_file_is_not_rotated() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("nova.log"), b"small").unwrap();
        rotate(dir.path(), 2 * 1024 * 1024, 5).unwrap();
        assert!(dir.path().join("nova.log").exists());
    }

    #[test]
    fn ring_keeps_only_warnings_and_errors() {
        let ring = Arc::new(Ring::new(200));
        let subscriber = tracing_subscriber::registry().with(RingLayer(ring.clone()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("обычное сообщение");
            for i in 0..300 {
                if i % 2 == 0 {
                    tracing::warn!("предупреждение {i}");
                } else {
                    tracing::error!("ошибка {i}");
                }
            }
        });
        let lines = ring.recent(500);
        assert_eq!(lines.len(), 200);
        assert!(lines.iter().all(|l| l.level == "WARN" || l.level == "ERROR"));
        assert!(lines[0].message.contains("299"), "новые первыми: {}", lines[0].message);
        assert!(!lines.iter().any(|l| l.message.contains("обычное")));
    }

    #[test]
    fn sanitize_cuts_urls_and_tokens() {
        let s = sanitize("fail http://127.0.0.1:1/x?t=abc token=zzz password=qq https://example.com/a");
        assert!(!s.contains("http"), "{s}");
        assert!(!s.contains("abc") && !s.contains("zzz") && !s.contains("qq"), "{s}");
        let long = sanitize(&"я".repeat(2000));
        assert_eq!(long.chars().count(), 500);
    }
}
