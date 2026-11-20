//! Фоновый импорт запрошенных сайтов с устойчивой очередью и без удаления старых записей.
use crate::{
    App,
    auth::Manager,
    error::{ApiError, ApiResult},
    own_sites,
    site_sources::{self, Entry, Source, Variant},
    util::now_ms,
};
use axum::{
    Json,
    extract::{Path, State},
};
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

fn host(m: &Manager) -> ApiResult<()> {
    if m.0.role == "host" {
        Ok(())
    } else {
        Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "host_only", "Источниками управляет только хост"))
    }
}
fn source(id: &str) -> ApiResult<&'static Source> {
    site_sources::by_id(id).ok_or_else(ApiError::not_found)
}
fn domain(s: &Source) -> String {
    reqwest::Url::parse(s.url).unwrap().host_str().unwrap().trim_start_matches("www.").into()
}

pub async fn list(State(app): State<Arc<App>>, m: Manager) -> ApiResult<Json<Value>> {
    let catalogs = own_sites::load(&app);
    let mut out = Vec::new();
    for s in site_sources::SOURCES {
        let d = domain(s);
        let configured = catalogs.iter().find(|c| c.domain == d);
        let job: Option<(String, Option<String>, i64, String, i64)> = app.db.with(|c| {
            c.query_row(
                "SELECT state,next_url,pages,message,updated_at FROM site_import_jobs WHERE domain=?",
                [&d],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .optional()
        })?;
        let running = app.site_jobs.lock().get(&d).is_some_and(|h| !h.is_finished());
        let counts:Value=app.db.with(|c|{
            c.query_row("SELECT COUNT(*),SUM(state IN ('imported','partial','unmatched')),SUM(state IN ('error','unmatched','partial')),SUM(state='pending') FROM site_import_entries WHERE domain=?",[&d],|r|Ok(json!({"found":r.get::<_,i64>(0)?,"imported":r.get::<_,Option<i64>>(1)?.unwrap_or(0),"failed":r.get::<_,Option<i64>>(2)?.unwrap_or(0),"pending":r.get::<_,Option<i64>>(3)?.unwrap_or(0)})))
        })?;
        let problems: Vec<Value> = if m.0.role == "host" {
            app.db.with(|c|{
            let mut st=c.prepare("SELECT url,metadata,state,message FROM site_import_entries WHERE domain=? AND state IN ('error','unmatched','partial') ORDER BY updated_at DESC LIMIT 20")?;
            let rows=st.query_map([&d],|r|{let meta:String=r.get(1)?;let meta:Value=serde_json::from_str(&meta).unwrap_or(Value::Null);Ok(json!({"url":r.get::<_,String>(0)?,"title":meta["title"],"state":r.get::<_,String>(2)?,"message":r.get::<_,String>(3)?}))})?;rows.collect::<rusqlite::Result<Vec<_>>>()
        })?
        } else {
            Vec::new()
        };
        out.push(json!({"source":s,"domain":d,"configured":configured.is_some(),"enabled":configured.is_some_and(|c|c.enabled),"running":running,
            "state":job.as_ref().map(|j|if j.0=="running"&&!running{"paused"}else{j.0.as_str()}).unwrap_or("new"),
            "pages":job.as_ref().map(|j|j.2).unwrap_or(0),"message":job.as_ref().map(|j|j.3.as_str()).unwrap_or(""),"counts":counts,"problems":problems,
            "verificationAvailable":m.0.local&&m.0.role=="host"&&app.host.can_verify_sources(),"sessionAllowed":app.source_sessions.lock().contains_key(s.id)}));
    }
    Ok(Json(json!({"items":out})))
}

/// Быстрая проверка доступности домена без импорта каталога или извлечения видеопотока.
pub async fn health(State(app): State<Arc<App>>, m: Manager, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    host(&m)?;
    let s = source(&id)?;
    let started = std::time::Instant::now();
    let response = tokio::time::timeout(
        Duration::from_secs(6),
        app.http.get(s.url).header(reqwest::header::RANGE, "bytes=0-0").send(),
    )
    .await;
    let elapsed = started.elapsed().as_millis();
    let result = match response {
        Ok(Ok(r)) => {
            let status = r.status();
            let content_type =
                r.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("");
            let (state, message, next_steps) = if status.is_success() || status.as_u16() == 206 {
                (
                    "available",
                    "Сайт отвечает. Эта проверка не гарантирует доступность конкретного видео.",
                    Vec::<&str>::new(),
                )
            } else if status.as_u16() == 401 || status.as_u16() == 403 || status.as_u16() == 429 {
                (
                    "restricted",
                    "Сайт отказал серверному запросу. Nova не обходит ограничение.",
                    vec![
                        "Запросить у владельца документированный API или разрешённый адрес встраивания.",
                        "Если доступ должен быть серверным — попросить владельца разрешить исходящий IP хоста Nova и указать требуемый способ авторизации.",
                        "Проверить условия использования и повторить эту проверку после официального разрешения.",
                    ],
                )
            } else {
                (
                    "error",
                    "Сайт ответил ошибкой. Подробности доступны по коду ответа.",
                    vec![
                        "Проверьте адрес источника и повторите проверку позже.",
                        "Если ошибка сохраняется, запросите у владельца сайта документацию интеграции.",
                    ],
                )
            };
            json!({"state":state,"status":status.as_u16(),"latencyMs":elapsed,"contentType":content_type,"message":message,"nextSteps":next_steps})
        }
        Ok(Err(e)) => {
            json!({"state":"error","latencyMs":elapsed,"message":format!("Не удалось связаться с сайтом: {}", e),"nextSteps":["Проверьте DNS, TLS и сетевой доступ хоста Nova.","Если проблема сохраняется, запросите у владельца документацию интеграции."]})
        }
        Err(_) => {
            json!({"state":"timeout","latencyMs":elapsed,"message":"Сайт не ответил за 6 секунд. Проверьте соединение и повторите попытку.","nextSteps":["Проверьте сетевой маршрут и доступность сайта с хоста Nova.","Уточните у владельца сайта ограничения по времени ответа и официальную точку интеграции."]})
        }
    };
    Ok(Json(result))
}

pub async fn connect(State(app): State<Arc<App>>, m: Manager, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    host(&m)?;
    let s = source(&id)?;
    let d = domain(s);
    let mut catalogs = own_sites::load(&app);
    if let Some(c) = catalogs.iter_mut().find(|c| c.domain == d) {
        c.enabled = true;
    } else {
        if catalogs.len() >= own_sites::MAX {
            return Err(ApiError::bad("too_many", "Не больше 20 каталогов"));
        }
        catalogs.push(own_sites::Catalog {
            domain: d.clone(),
            name: s.name.into(),
            kind: s.kind.into(),
            manifest: None,
            search: None,
            enabled: true,
        });
    }
    let mut sites = super::links::extra_sites(&app);
    if !sites.contains(&d) {
        sites.push(d.clone());
    }
    if sites.len() > 30 {
        return Err(ApiError::bad("too_many", "Не больше 30 сайтов"));
    }
    app.db.tx(|c| {
        for (k, v) in [
            ("video.catalogs", serde_json::to_string(&catalogs).unwrap()),
            ("video.sites", serde_json::to_string(&sites).unwrap()),
        ] {
            c.execute(
                "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![k, v],
            )?;
        }
        Ok(())
    })?;
    Ok(Json(json!({"ok":true,"domain":d})))
}

pub async fn start(State(app): State<Arc<App>>, m: Manager, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    host(&m)?;
    let s = source(&id)?;
    let d = domain(s);
    if !own_sites::enabled_domains(&app).contains(&d) {
        return Err(ApiError::bad("source_disabled", "Сначала подключите и включите источник"));
    }
    let mut tasks = app.site_jobs.lock();
    if tasks.get(&d).is_some_and(|h| !h.is_finished()) {
        return Ok(Json(json!({"running":true})));
    }
    app.db.tx(|c|{
        c.execute("UPDATE site_import_pages SET state='pending' WHERE domain=?1 AND EXISTS(SELECT 1 FROM site_import_jobs WHERE domain=?1 AND state IN ('complete','partial'))",[&d])?;
        c.execute("UPDATE site_import_entries SET state='pending' WHERE domain=?1 AND (state IN ('error','unmatched') OR EXISTS(SELECT 1 FROM site_import_jobs WHERE domain=?1 AND state IN ('complete','partial')))",[&d])?;
        c.execute("INSERT INTO site_import_jobs(domain,state,next_url,updated_at) VALUES(?1,'running',?2,?3) ON CONFLICT(domain) DO UPDATE SET state='running',next_url=CASE WHEN state IN ('complete','partial') THEN excluded.next_url ELSE next_url END,message='',updated_at=excluded.updated_at",params![d,initial_url(s),now_ms()])?;
        c.execute("INSERT INTO site_import_pages(domain,url) VALUES(?1,COALESCE((SELECT next_url FROM site_import_jobs WHERE domain=?1),?2)) ON CONFLICT DO NOTHING",params![d,initial_url(s)])?;Ok(())
    })?;
    let task_app = app.clone();
    let profile = m.0.profile.id;
    let key = d.clone();
    tasks.insert(
        d,
        tokio::spawn(async move {
            if let Err(e) = run(&task_app, s, profile).await {
                let _ = task_app.db.with(|c| {
                    c.execute(
                        "UPDATE site_import_jobs SET state='blocked',message=?1,updated_at=?2 WHERE domain=?3",
                        params![e.to_string(), now_ms(), key],
                    )
                });
            }
        }),
    );
    Ok(Json(json!({"running":true})))
}

fn initial_url(s: &Source) -> &str {
    if s.id == "yummy" { "https://old.yummyani.me/api/anime?limit=50&offset=0" } else { s.url }
}

#[derive(Deserialize)]
pub struct VerifyBody {
    action: String,
}
pub async fn verify(
    State(app): State<Arc<App>>,
    m: Manager,
    Path(id): Path<String>,
    Json(b): Json<VerifyBody>,
) -> ApiResult<Json<Value>> {
    host(&m)?;
    let s = source(&id)?;
    if !m.0.local {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "local_only",
            "Проверку источника открывает хост в приложении Nova",
        ));
    }
    let h = app.host.clone();
    let id = s.id;
    match b.action.as_str() {
        "open" => tokio::task::spawn_blocking(move || h.source_open(id))
            .await
            .map_err(ApiError::internal)?
            .map_err(|e| ApiError::bad("source_access", e))?,
        "use" => {
            let cookies = tokio::task::spawn_blocking(move || h.source_cookies(id))
                .await
                .map_err(ApiError::internal)?
                .map_err(|e| ApiError::bad("source_access", e))?;
            if cookies.is_empty() {
                return Err(ApiError::bad("source_access", "Сайт ещё не выдал сессию - пройдите проверку вручную"));
            }
            app.source_sessions.lock().insert(id.into(), cookies);
        }
        "forget" => {
            app.source_sessions.lock().remove(id);
        }
        _ => return Err(ApiError::bad("action", "Неизвестное действие проверки источника")),
    }
    Ok(Json(json!({"ok":true,"sessionAllowed":app.source_sessions.lock().contains_key(id)})))
}

pub async fn stop(State(app): State<Arc<App>>, m: Manager, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    host(&m)?;
    let d = domain(source(&id)?);
    if let Some(task) = app.site_jobs.lock().remove(&d) {
        task.abort();
    }
    app.db.with(|c| {
        c.execute("UPDATE site_import_jobs SET state='paused',updated_at=?1 WHERE domain=?2", params![now_ms(), d])
    })?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchBody {
    url: String,
    imdb_id: String,
}
pub async fn assign(
    State(app): State<Arc<App>>,
    _m: Manager,
    Path(id): Path<String>,
    Json(b): Json<MatchBody>,
) -> ApiResult<Json<Value>> {
    if !crate::discover::imdb_id(&b.imdb_id) {
        return Err(ApiError::bad("imdb", "Нужен идентификатор IMDb вида tt1234567"));
    }
    let d = domain(source(&id)?);
    let raw: Option<String> = app.db.with(|c| {
        c.query_row("SELECT metadata FROM site_import_entries WHERE domain=?1 AND url=?2", params![d, b.url], |r| {
            r.get(0)
        })
        .optional()
    })?;
    let mut entry: Entry = serde_json::from_str(&raw.ok_or_else(ApiError::not_found)?).map_err(ApiError::internal)?;
    entry.imdb = Some(b.imdb_id);
    app.db.with(|c|c.execute("UPDATE site_import_entries SET metadata=?1,state='pending',message='',updated_at=?2 WHERE domain=?3 AND url=?4",params![serde_json::to_string(&entry).unwrap(),now_ms(),d,b.url]))?;
    Ok(Json(json!({"ok":true})))
}

async fn run(app: &Arc<App>, s: &'static Source, profile: i64) -> anyhow::Result<()> {
    let d = domain(s);
    loop {
        if !own_sites::enabled_domains(app).contains(&d) {
            app.db.with(|c|c.execute("UPDATE site_import_jobs SET state='paused',message='Источник выключен',updated_at=?1 WHERE domain=?2",params![now_ms(),d]))?;
            return Ok(());
        }
        let pending:Option<(String,String)>=app.db.with(|c|c.query_row("SELECT url,metadata FROM site_import_entries WHERE domain=? AND state='pending' ORDER BY updated_at,url LIMIT 1",[&d],|r|Ok((r.get(0)?,r.get(1)?))).optional())?;
        if let Some((url, raw)) = pending {
            let entry: Entry = serde_json::from_str(&raw)?;
            let matched = site_sources::match_imdb(app, &entry).await;
            // В базе - только фильмы с IMDb: что не нашлось на IMDb, не добавляется (источник нужен лишь для просмотра).
            let target = match matched {
                Ok(id) => id,
                Err(e) => {
                    app.db.with(|c| {
                        c.execute(
                            "UPDATE site_import_entries SET state='unmatched',message=?1,updated_at=?2 WHERE domain=?3 AND url=?4",
                            params![format!("Не найден на IMDb, в базу не добавлен: {e}"), now_ms(), d, url],
                        )
                    })?;
                    continue;
                }
            };
            let (state, message) =
                match tokio::time::timeout(Duration::from_secs(180), site_sources::variants(&entry, Some(app))).await {
                    Ok(Ok((variants, notes))) => {
                        save_variants(app, &d, s, &entry, &target, &variants, profile)?;
                        (if notes.is_empty() { "imported" } else { "partial" }, notes.join("; "))
                    }
                    Ok(Err(e)) => ("error", e.to_string()),
                    Err(_) => ("error", "Истёк срок получения списка вариантов; можно повторить импорт".into()),
                };
            app.db.with(|c| {
                c.execute(
                    "UPDATE site_import_entries SET state=?1,message=?2,updated_at=?3 WHERE domain=?4 AND url=?5",
                    params![state, message, now_ms(), d, url],
                )
            })?;
            continue;
        }
        let next: Option<String> = app.db.with(|c| {
            c.query_row(
                "SELECT url FROM site_import_pages WHERE domain=? AND state='pending' ORDER BY url LIMIT 1",
                [&d],
                |r| r.get(0),
            )
            .optional()
        })?;
        let Some(next) = next else {
            app.db.with(|c|c.execute("UPDATE site_import_jobs SET state=CASE WHEN EXISTS(SELECT 1 FROM site_import_entries WHERE domain=?1 AND state IN ('error','unmatched','partial')) THEN 'partial' ELSE 'complete' END,message='Обход завершён. Импорт записей не подтверждает воспроизведение всех потоков.',updated_at=?2 WHERE domain=?1",params![d,now_ms()]))?;
            return Ok(());
        };
        let url = reqwest::Url::parse(&next)?;
        anyhow::ensure!(
            url.host_str().is_some_and(|h| h.trim_start_matches("www.") == d) && crate::outbound::valid_url(&url),
            "Курсор импорта вышел за пределы источника"
        );
        let (entries, following) = site_sources::list(s, Some(&next), Some(app)).await?;
        let mut branches = site_sources::branches(s, &next, Some(app)).await?;
        if let Some(ref url) = following {
            branches.push(url.clone());
        }
        anyhow::ensure!(
            following.as_deref() != Some(next.as_str()),
            "Источник повторил ту же страницу; импорт остановлен"
        );
        app.db.tx(|c|{
            for e in &entries{
                c.execute("INSERT INTO site_import_entries(domain,url,metadata,state,updated_at) VALUES(?1,?2,?3,'pending',?4) ON CONFLICT(domain,url) DO UPDATE SET metadata=CASE WHEN json_extract(site_import_entries.metadata,'$.imdb') IS NULL THEN excluded.metadata ELSE site_import_entries.metadata END",params![d,e.url,serde_json::to_string(e).unwrap(),now_ms()])?;
            }
            for url in &branches{c.execute("INSERT INTO site_import_pages(domain,url) VALUES(?1,?2) ON CONFLICT DO NOTHING",params![d,url])?;}
            c.execute("UPDATE site_import_pages SET state='done' WHERE domain=?1 AND url=?2",params![d,next])?;
            c.execute("UPDATE site_import_jobs SET next_url=?1,pages=pages+1,updated_at=?2 WHERE domain=?3",params![following,now_ms(),d])?;Ok(())
        })?;
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn save_variants(
    app: &App,
    d: &str,
    source: &Source,
    entry: &Entry,
    imdb: &str,
    variants: &[Variant],
    profile: i64,
) -> anyhow::Result<()> {
    app.db.tx(|c|{
        c.execute("INSERT INTO ext_titles(id,kind,title,year,genres,updated_at,detailed) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO NOTHING",params![imdb,entry.kind,entry.title,Option::<i64>::None,if source.kind=="anime"{"[\"Аниме\"]"}else{"[]"},now_ms(),0])?;
        if entry.title.chars().any(|ch| matches!(ch,'А'..='я'|'Ё'|'ё')) {
            c.execute("INSERT INTO title_localizations(id,title,data,at) VALUES(?1,?2,'{}',?3) ON CONFLICT(id) DO UPDATE SET title=excluded.title,at=excluded.at",params![imdb,entry.title,now_ms()])?;
        }
        let local:Option<i64>=c.query_row("SELECT id FROM titles WHERE imdb_id=? LIMIT 1",[imdb],|r|r.get(0)).optional()?;
        for v in variants{
            if let(Some(season),Some(episode))=(v.season,v.episode){
                c.execute("INSERT INTO ext_episodes(ext_id,season,episode,name) VALUES(?1,?2,?3,?4) ON CONFLICT DO NOTHING",params![imdb,season,episode,format!("Серия {episode}")])?;
                c.execute("INSERT INTO ext_seasons(ext_id,season,at) VALUES(?1,?2,?3) ON CONFLICT DO NOTHING",params![imdb,season,now_ms()])?;
            }
            let stream_key=format!("site:{}:{}",crate::util::sha256_hex(entry.url.as_bytes()),v.id);
            let old:Option<i64>=c.query_row("SELECT link_id FROM catalog_links WHERE domain=?1 AND stream_key=?2",params![d,stream_key],|r|r.get(0)).optional()?;
            let label=format!("{} · {}",source.name,v.label);let ext=local.is_none().then_some(imdb);
            if let Some(id)=old{
                c.execute("UPDATE links SET url=?1,label=?2,season=?3,episode=?4,title_id=?6,ext_id=?7 WHERE id=?5",params![v.url,label,v.season,v.episode,id,local,ext])?;
                c.execute("UPDATE catalog_links SET item_key=?1 WHERE link_id=?2",params![imdb,id])?;
            }else{
                c.execute("INSERT INTO links(title_id,ext_id,kind,url,label,added_by,added_at,season,episode) VALUES(?1,?2,'site',?3,?4,?5,?6,?7,?8)",params![local,ext,v.url,label,profile,now_ms(),v.season,v.episode])?;
                c.execute("INSERT INTO catalog_links(link_id,domain,item_key,stream_key) VALUES(?1,?2,?3,?4)",params![c.last_insert_rowid(),d,imdb,stream_key])?;
            }
        }Ok(())
    })?;
    // Карточка и варианты сразу попадают в очередь постеров и русских названий,
    // иначе новая запись ждёт следующего пробуждения воркера (до часа).
    app.kick_ext();
    Ok(())
}
