//! Явный обмен каталогами: без исполнения HTML или кода стороннего сайта.
use crate::auth::Manager;
use crate::error::{ApiError, ApiResult};
use crate::{App, outbound, own_sites, util::now_ms};
use axum::{Json, extract::State};
use reqwest::Url;
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Arc, time::Duration};

pub const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    version: u32,
    items: Vec<Item>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Item {
    imdb_id: String,
    title: String,
    kind: String,
    year: Option<i64>,
    streams: Vec<Stream>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stream {
    id: String,
    url: String,
    label: String,
    quality: Option<String>,
    voice: Option<String>,
    season: Option<i64>,
    episode: Option<i64>,
}

#[derive(Deserialize)]
pub struct ImportBody {
    domain: String,
    manifest: Option<Manifest>,
}

fn bad(message: &str) -> ApiError {
    ApiError::bad("bad_catalog", message)
}

async fn fetch_manifest(catalog: &own_sites::Catalog) -> ApiResult<Manifest> {
    let raw = catalog.manifest.as_deref().ok_or_else(|| bad("Укажите адрес JSON-каталога или выберите файл"))?;
    let url = Url::parse(raw).map_err(|_| bad("Некорректный адрес JSON-каталога"))?;
    if !outbound::valid_url(&url) {
        return Err(bad("Нужен публичный адрес каталога"));
    }
    let fetch = async {
        let client = outbound::builder().timeout(Duration::from_secs(20)).build()?;
        let response = client.get(url).send().await?.error_for_status()?;
        outbound::limited(response, MAX_BYTES).await
    };
    let bytes = tokio::time::timeout(Duration::from_secs(25), fetch)
        .await
        .map_err(|_| bad("Источник не ответил за 25 секунд"))?
        .map_err(|_| bad("Не удалось получить JSON-каталог - проверьте адрес и доступность источника"))?;
    serde_json::from_slice(&bytes).map_err(|_| bad("Источник должен отдавать JSON-каталог Nova версии 1"))
}

#[derive(Deserialize)]
pub struct CheckBody {
    domain: String,
}

pub async fn check(State(app): State<Arc<App>>, m: Manager, Json(b): Json<CheckBody>) -> ApiResult<Json<Value>> {
    if m.0.role != "host" {
        return Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "host_only", "Источники проверяет только хост"));
    }
    let catalog = own_sites::load(&app)
        .into_iter()
        .find(|c| c.domain == b.domain)
        .ok_or_else(|| bad("Сначала добавьте источник"))?;
    let manifest = fetch_manifest(&catalog).await?;
    manifest.validate(&catalog.kind)?;
    Ok(Json(
        json!({"titles":manifest.items.len(),"variants":manifest.items.iter().map(|i| i.streams.len()).sum::<usize>()}),
    ))
}

impl Manifest {
    fn validate(&self, kind: &str) -> ApiResult<()> {
        if self.version != 1 || self.items.is_empty() || self.items.len() > 10_000 {
            return Err(bad("Нужен каталог версии 1, от 1 до 10000 тайтлов"));
        }
        let mut items = HashSet::new();
        let mut count = 0;
        for item in &self.items {
            let imdb = item.imdb_id.strip_prefix("tt").unwrap_or("");
            if !(7..=12).contains(&imdb.len())
                || !imdb.bytes().all(|b| b.is_ascii_digit())
                || !items.insert(&item.imdb_id)
                || item.title.trim().is_empty()
                || item.title.chars().count() > 300
                || !matches!(item.kind.as_str(), "movie" | "series" | "anime")
                || (kind != "all" && kind != item.kind)
                || item.year.is_some_and(|y| !(1800..=2200).contains(&y))
                || item.streams.is_empty()
            {
                return Err(bad(
                    "Проверьте IMDb ID, название, тип, год и варианты каждого тайтла; повторы ID запрещены",
                ));
            }
            let mut ids = HashSet::new();
            for stream in &item.streams {
                count += 1;
                let url = Url::parse(&stream.url).map_err(|_| bad("Некорректный адрес видео"))?;
                if count > 50_000
                    || stream.id.trim().is_empty()
                    || stream.id.len() > 100
                    || !ids.insert(&stream.id)
                    || stream.label.trim().is_empty()
                    || stream.label.chars().count() > 200
                    || stream.quality.as_ref().is_some_and(|v| v.chars().count() > 80)
                    || stream.voice.as_ref().is_some_and(|v| v.chars().count() > 100)
                    || !outbound::valid_url(&url)
                    || super::links::parse(&stream.url).is_none_or(|p| p.kind != "direct")
                    || stream.season.is_some() != stream.episode.is_some()
                    || stream.season.is_some_and(|s| s < 0)
                    || stream.episode.is_some_and(|e| e < 1)
                    || (item.kind == "series" && stream.season.is_none())
                    || (item.kind == "anime"
                        && item.streams.iter().any(|s| s.season.is_some())
                        && stream.season.is_none())
                    || (item.kind == "movie" && stream.season.is_some())
                {
                    return Err(bad(
                        "Проверьте варианты: уникальный id, подпись, публичный MP4/WebM/HLS, сезон и серия. Максимум 50000 вариантов",
                    ));
                }
            }
        }
        Ok(())
    }
}

pub async fn import(State(app): State<Arc<App>>, m: Manager, Json(b): Json<ImportBody>) -> ApiResult<Json<Value>> {
    if m.0.role != "host" {
        return Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "host_only", "Каталоги импортирует только хост"));
    }
    let catalog = own_sites::load(&app)
        .into_iter()
        .find(|c| c.domain == b.domain)
        .ok_or_else(|| bad("Сначала добавьте источник в каталог"))?;
    if !catalog.enabled {
        return Err(bad("Включите источник перед импортом"));
    }
    let manifest = if let Some(manifest) = b.manifest { manifest } else { fetch_manifest(&catalog).await? };
    manifest.validate(&catalog.kind)?;
    // Повторная проверка после сети: источник мог быть выключен из другого окна.
    if !own_sites::enabled_domains(&app).contains(&catalog.domain) {
        return Err(bad("Источник уже выключен"));
    }
    let (mut added, mut updated) = (0, 0);
    app.db.tx(|c| {
        for item in &manifest.items {
            let kind = if item.kind == "anime" {
                if item.streams[0].season.is_some() { "series" } else { "movie" }
            } else { &item.kind };
            c.execute("INSERT INTO ext_titles(id,kind,title,year,genres,updated_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO NOTHING",
                params![item.imdb_id, kind, item.title.trim(), item.year, if item.kind == "anime" { "[\"Аниме\"]" } else { "[]" }, now_ms()])?;
            let local: Option<i64> = c.query_row("SELECT id FROM titles WHERE imdb_id = ? LIMIT 1", [&item.imdb_id], |r| r.get(0)).optional()?;
            for stream in &item.streams {
                if let (Some(season), Some(episode)) = (stream.season, stream.episode) {
                    c.execute("INSERT INTO ext_episodes(ext_id,season,episode,name) VALUES(?1,?2,?3,?4) ON CONFLICT DO NOTHING",
                        params![item.imdb_id, season, episode, format!("Серия {episode}")])?;
                    c.execute("INSERT INTO ext_seasons(ext_id,season,at) VALUES(?1,?2,?3) ON CONFLICT DO NOTHING",
                        params![item.imdb_id, season, now_ms()])?;
                }
                let source_name = if catalog.name.is_empty() { &catalog.domain } else { &catalog.name };
                let label = [Some(source_name.as_str()), Some(stream.label.as_str()), stream.quality.as_deref(), stream.voice.as_deref()]
                    .into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
                let old: Option<i64> = c.query_row("SELECT link_id FROM catalog_links WHERE domain=?1 AND item_key=?2 AND stream_key=?3",
                    params![catalog.domain, item.imdb_id, stream.id], |r| r.get(0)).optional()?;
                let ext = local.is_none().then_some(&item.imdb_id);
                if let Some(id) = old {
                    c.execute("UPDATE links SET title_id=?1,ext_id=?2,url=?3,label=?4,season=?5,episode=?6 WHERE id=?7",
                        params![local, ext, stream.url, label, stream.season, stream.episode, id])?;
                    updated += 1;
                } else {
                    c.execute("INSERT INTO links(title_id,ext_id,kind,url,label,season,episode,added_by,added_at) VALUES(?1,?2,'direct',?3,?4,?5,?6,?7,?8)",
                        params![local, ext, stream.url, label, stream.season, stream.episode, m.0.profile.id, now_ms()])?;
                    c.execute("INSERT INTO catalog_links(link_id,domain,item_key,stream_key) VALUES(?1,?2,?3,?4)",
                        params![c.last_insert_rowid(), catalog.domain, item.imdb_id, stream.id])?;
                    added += 1;
                }
            }
        }
        Ok(())
    })?;
    Ok(Json(json!({"titles":manifest.items.len(),"added":added,"updated":updated})))
}
