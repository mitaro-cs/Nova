//! Лента «как на IMDb»: подборки (популярное, топ-250) и поиск по всему IMDb - фильмы и сериалы,
//! которых может не быть на компьютере хоста. Их можно отметить «Буду смотреть»/«Просмотрел»,
//! положить в свой список и попросить хоста добавить. Картинки сервер скачивает к себе: браузер
//! ходит только к нам. Подборки обновляются раз в 12 часов, постеры докачиваются в фоне.

use crate::App;
use crate::meta::imdb::{Imdb, sized};
use crate::util::now_ms;
use futures_util::StreamExt;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// Откуда подборка: готовый список IMDb, поиск IMDb по жанру и стране или «скоро в кино».
pub enum Src {
    Chart(&'static str),
    /// Типы, жанр IMDb, страны, только вышедшее за последние дни, не меньше стольких оценок.
    Search {
        types: &'static [&'static str],
        genre: Option<&'static str>,
        countries: &'static [&'static str],
        days: Option<i64>,
        votes: i64,
    },
    Soon,
}

pub const MOVIE: &[&str] = &["movie"];
pub const SERIES: &[&str] = &["tvSeries", "tvMiniSeries"];
pub const ANY: &[&str] = &["movie", "tvSeries", "tvMiniSeries"];

/// Запрос - это жанр («фантастика», «комедии», «сериалы ужасы»): жанр IMDb, подпись и что искать
/// (фильмы, сериалы или всё). Слова вроде «фильмы», «лучшие», «про» не мешают; любое другое
/// слово - значит, ищут название.
pub fn genre_query(query: &str) -> Option<(&'static str, &'static str, &'static [&'static str])> {
    const STEMS: &[(&str, &str, &str)] = &[
        ("фантаст", "Sci-Fi", "Фантастика"),
        ("научн", "Sci-Fi", "Фантастика"),
        ("комеди", "Comedy", "Комедия"),
        ("комедийн", "Comedy", "Комедия"),
        ("ужас", "Horror", "Ужасы"),
        ("хоррор", "Horror", "Ужасы"),
        ("боевик", "Action", "Боевик"),
        ("экшн", "Action", "Боевик"),
        ("драм", "Drama", "Драма"),
        ("триллер", "Thriller", "Триллер"),
        ("детектив", "Mystery", "Детектив"),
        ("мелодрам", "Romance", "Мелодрама"),
        ("романтич", "Romance", "Мелодрама"),
        ("приключен", "Adventure", "Приключения"),
        ("фэнтези", "Fantasy", "Фэнтези"),
        ("фентези", "Fantasy", "Фэнтези"),
        ("мульт", "Animation", "Мультфильм"),
        ("анимац", "Animation", "Мультфильм"),
        ("документальн", "Documentary", "Документальный"),
        ("криминал", "Crime", "Криминал"),
        ("военн", "War", "Военный"),
        ("войн", "War", "Военный"),
        ("вестерн", "Western", "Вестерн"),
        ("биограф", "Biography", "Биография"),
        ("историческ", "History", "История"),
        ("мюзикл", "Musical", "Мюзикл"),
        ("спортивн", "Sport", "Спорт"),
        ("семейн", "Family", "Семейный"),
        ("нуар", "Film-Noir", "Нуар"),
    ];
    const FILLER: &[&str] =
        &["лучш", "топ", "про", "кино", "новы", "популярн", "интересн", "хорош", "смотреть", "жанр"];
    let q = crate::util::norm(query);
    let mut genre = None;
    let mut types: &'static [&'static str] = ANY;
    for w in q.split_whitespace() {
        if w.starts_with("сериал") {
            types = SERIES;
        } else if w.starts_with("фильм") && !w.starts_with("фильмограф") {
            types = MOVIE;
        } else if let Some(&(_, en, ru)) = STEMS.iter().find(|(s, ..)| w.starts_with(s)) {
            // «мультсериалы» - мультфильм и сериал.
            if w.contains("сериал") {
                types = SERIES;
            }
            if genre.is_some_and(|(g, _)| g != en) {
                return None;
            }
            genre = Some((en, ru));
        } else if !FILLER.iter().any(|f| w.starts_with(f)) {
            return None;
        }
    }
    genre.map(|(en, ru)| (en, ru, types))
}

/// Популярное в жанре с IMDb (до 50, сначала самое известное): ids в ленте, на 12 часов.
pub async fn genre_search(
    app: &Arc<App>,
    genre: &'static str,
    types: &'static [&'static str],
) -> anyhow::Result<Vec<String>> {
    let src = Src::Search { types, genre: Some(genre), countries: &[], days: None, votes: 25_000 };
    search_cached(app, &format!("genre:{genre}:{}:votes", types.len()), &src).await
}

/// Поиск IMDb по источнику (жанр, страны), больше всего оценок - первым; ids в ленте, на час.
pub async fn search_cached(app: &Arc<App>, key: &str, src: &Src) -> anyhow::Result<Vec<String>> {
    let cached: Option<String> = app.db.with(|c| {
        c.query_row(
            "SELECT ids FROM catalog_search_cache WHERE query = ? AND at > ?",
            params![key, now_ms() - 3_600_000],
            |r| r.get(0),
        )
        .optional()
    })?;
    if let Some(c) = cached {
        return Ok(serde_json::from_str(&c)?);
    }
    let imdb = Imdb::new(app.http.clone());
    let items = parse_chart(&fetch_feed(&imdb, src, "USER_RATING_COUNT").await?);
    upsert(app, &items)?;
    app.kick_ext();
    let ids: Vec<String> = items.into_iter().map(|e| e.id).collect();
    if !ids.is_empty() {
        app.db.with(|c| {
            c.execute(
                "INSERT INTO catalog_search_cache VALUES (?, ?, ?)
                 ON CONFLICT(query) DO UPDATE SET ids = excluded.ids, at = excluded.at",
                params![key, json!(ids).to_string(), now_ms()],
            )
        })?;
    }
    Ok(ids)
}

const fn genre(types: &'static [&'static str], g: &'static str, votes: i64) -> Src {
    Src::Search { types, genre: Some(g), countries: &[], days: None, votes }
}

/// Подборки: ключ у нас (вкладки «Фильмы»/«Сериалы» берут `*_movies`/`*_series`), подпись, откуда.
pub const FEEDS: &[(&str, &str, Src)] = &[
    ("popular_movies", "Популярные фильмы", Src::Chart("MOST_POPULAR_MOVIES")),
    ("popular_series", "Популярные сериалы", Src::Chart("MOST_POPULAR_TV_SHOWS")),
    (
        "new_movies",
        "Новинки кино",
        Src::Search { types: MOVIE, genre: None, countries: &[], days: Some(180), votes: 1000 },
    ),
    (
        "new_series",
        "Новые сериалы",
        Src::Search { types: SERIES, genre: None, countries: &[], days: Some(180), votes: 1000 },
    ),
    ("soon_movies", "Скоро в кино", Src::Soon),
    ("top_movies", "Топ-250 фильмов", Src::Chart("TOP_RATED_MOVIES")),
    ("top_series", "Лучшие сериалы", Src::Chart("TOP_RATED_TV_SHOWS")),
    (
        "russian_movies",
        "Российское кино",
        Src::Search { types: MOVIE, genre: None, countries: &["RU", "SUHH"], days: None, votes: 3000 },
    ),
    (
        "russian_series",
        "Российские сериалы",
        Src::Search { types: SERIES, genre: None, countries: &["RU"], days: None, votes: 1000 },
    ),
    ("comedy_movies", "Комедии", genre(MOVIE, "Comedy", 20000)),
    ("action_movies", "Боевики", genre(MOVIE, "Action", 20000)),
    ("thriller_movies", "Триллеры", genre(MOVIE, "Thriller", 20000)),
    ("horror_movies", "Ужасы", genre(MOVIE, "Horror", 15000)),
    ("scifi_movies", "Фантастика", genre(MOVIE, "Sci-Fi", 20000)),
    ("drama_movies", "Драмы", genre(MOVIE, "Drama", 20000)),
    ("romance_movies", "Мелодрамы", genre(MOVIE, "Romance", 15000)),
    ("animation_movies", "Мультфильмы", genre(MOVIE, "Animation", 10000)),
    ("family_movies", "Для всей семьи", genre(MOVIE, "Family", 10000)),
    ("documentary_movies", "Документальное", genre(MOVIE, "Documentary", 3000)),
    ("crime_series", "Криминальные сериалы", genre(SERIES, "Crime", 10000)),
    ("comedy_series", "Комедийные сериалы", genre(SERIES, "Comedy", 10000)),
    ("scifi_series", "Фантастические сериалы", genre(SERIES, "Sci-Fi", 10000)),
    (
        "anime_movies",
        "Аниме-фильмы",
        Src::Search { types: MOVIE, genre: Some("Animation"), countries: &["JP"], days: None, votes: 1000 },
    ),
    (
        "anime_series",
        "Аниме-сериалы",
        Src::Search { types: SERIES, genre: Some("Animation"), countries: &["JP"], days: None, votes: 5000 },
    ),
];

const NODE: &str = "id titleText { text } originalTitleText { text } titleType { id isSeries }
    releaseYear { year } releaseDate { year month day } ratingsSummary { aggregateRating voteCount }
    primaryImage { url } genres { genres { text } }";

/// Поиск IMDb: `POPULARITY` - сначала модное сейчас, `USER_RATING_COUNT` - больше всего оценок (классика жанра).
fn search_query(sort: &str) -> String {
    let order = if sort == "POPULARITY" { "ASC" } else { "DESC" };
    format!(
        "query A($c: AdvancedTitleSearchConstraints!) {{ advancedTitleSearch(first: 50, constraints: $c,
           sort: {{ sortBy: {sort}, sortOrder: {order} }}) {{ edges {{ node {{ title {{ {NODE} }} }} }} }} }}"
    )
}

fn soon_query() -> String {
    format!(
        "query S($from: Date!) {{ comingSoon(first: 50, comingSoonType: MOVIE, releasingOnOrAfter: $from) {{
           edges {{ node {{ {NODE} }} }} }} }}"
    )
}

/// Дата `YYYY-MM-DD` по миллисекундам Unix (UTC).
pub fn ymd(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    // Алгоритм Хиннанта: дни от 1970-01-01 → гражданская дата.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Подборка с IMDb в виде `chartTitles` (`edges[].node` - фильм).
async fn fetch_feed(imdb: &Imdb, src: &Src, sort: &str) -> anyhow::Result<Value> {
    let now = now_ms();
    match src {
        Src::Chart(chart) => imdb.data(CHART, json!({ "chart": chart }), "chartTitles").await,
        Src::Soon => imdb.data(&soon_query(), json!({ "from": ymd(now) }), "comingSoon").await,
        Src::Search { types, genre, countries, days, votes } => {
            let mut c = json!({
                "titleTypeConstraint": { "anyTitleTypeIds": types },
                "userRatingsConstraint": { "ratingsCountRange": { "min": votes } },
            });
            if let Some(g) = genre {
                c["genreConstraint"] = json!({ "allGenreIds": [g] });
            }
            if !countries.is_empty() {
                c["originCountryConstraint"] = json!({ "anyPrimaryCountries": countries });
            }
            if let Some(d) = days {
                c["releaseDateConstraint"] =
                    json!({ "releaseDateRange": { "start": ymd(now - d * 86_400_000), "end": ymd(now) } });
            }
            let v = imdb.data(&search_query(sort), json!({ "c": c }), "advancedTitleSearch").await?;
            let edges: Vec<Value> =
                v["edges"].as_array().into_iter().flatten().map(|e| json!({ "node": e["node"]["title"] })).collect();
            Ok(json!({ "edges": edges }))
        }
    }
}

const CHART: &str = r#"query C($chart: ChartTitleType!) { chartTitles(first: 250, chart: { chartType: $chart }) {
  edges { currentRank node {
    id titleText { text } originalTitleText { text } titleType { id isSeries }
    releaseYear { year } ratingsSummary { aggregateRating voteCount }
    primaryImage { url } genres { genres { text } }
  } }
} }"#;

const SIMILAR: &str = r#"query S($id: ID!) { title(id: $id) { moreLikeThisTitles(first: 12) {
  edges { node {
    id titleText { text } originalTitleText { text } titleType { id isSeries }
    releaseYear { year } ratingsSummary { aggregateRating voteCount }
    primaryImage { url } genres { genres { text } }
  } }
} } }"#;

/// Описание фильма из ленты в нынешнем виде (`ext_titles.detailed`): 1 - только сюжет (0.6),
/// 2 - со своей страницей: кадр, актёры, кадры, похожие; 3 (1.0) - и сезоны сериала (второй
/// запрос IMDb снова работает: актёры, кадры и русские названия - заново).
/// 5 (alpha 0.4) - кадр баннера из фильма (`still_frame` около 16:9), а не рекламный плакат.
pub const DETAILED: i64 = 6;

/// Обновление ленты: раз в 3 часа вместо 12, чтобы новые подборки появлялись быстрее.
/// Чаще не стоит: IMDb неофициальный, лишние запросы легко приводят к блокировке.
const EVERY: Duration = Duration::from_secs(3 * 3600);

#[derive(Debug, Clone, PartialEq)]
pub struct Ext {
    pub id: String,
    pub series: bool,
    pub title: String,
    pub original: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub votes: Option<i64>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    /// Дата выхода (YYYY-MM-DD, у неточной - YYYY или YYYY-MM): по ней видно, что фильм ещё не вышел.
    pub release: Option<String>,
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

pub fn valid_id(id: &str) -> bool {
    imdb_id(id) || (id.len() == 66 && id.starts_with("nv") && id[2..].bytes().all(|b| b.is_ascii_hexdigit()))
}

pub fn imdb_id(id: &str) -> bool {
    id.len() >= 9 && id.len() <= 12 && id.starts_with("tt") && id[2..].bytes().all(|b| b.is_ascii_digit())
}

/// Разбор `chartTitles`: только фильмы и сериалы (без эпизодов и игр).
pub fn parse_chart(v: &Value) -> Vec<Ext> {
    v["edges"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let n = &e["node"];
            let id = text(&n["id"]).filter(|i| valid_id(i))?;
            let t = n["titleType"]["id"].as_str().unwrap_or("movie");
            if matches!(t, "tvEpisode" | "videoGame" | "podcastSeries" | "podcastEpisode") {
                return None;
            }
            let series = n["titleType"]["isSeries"].as_bool().unwrap_or(t.starts_with("tv"));
            let title = text(&n["titleText"]["text"])?;
            let original = text(&n["originalTitleText"]["text"]).filter(|o| *o != title);
            Some(Ext {
                id,
                series,
                title,
                original,
                year: n["releaseYear"]["year"].as_i64().map(|y| y as i32),
                rating: n["ratingsSummary"]["aggregateRating"].as_f64(),
                votes: n["ratingsSummary"]["voteCount"].as_i64(),
                genres: n["genres"]["genres"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|g| text(&g["text"]))
                    .take(3)
                    .collect(),
                poster_url: text(&n["primaryImage"]["url"]).map(|u| sized(&u, 342)),
                release: release_of(&n["releaseDate"]),
            })
        })
        .collect()
}

/// Дата выхода из releaseDate { year month day } IMDb: 2027-03-05, без дня - 2027-03, только год - 2027.
fn release_of(v: &Value) -> Option<String> {
    let y = v["year"].as_i64()?;
    Some(match (v["month"].as_i64(), v["day"].as_i64()) {
        (Some(m), Some(d)) => format!("{y:04}-{m:02}-{d:02}"),
        (Some(m), None) => format!("{y:04}-{m:02}"),
        _ => format!("{y:04}"),
    })
}

pub fn upsert(app: &App, items: &[Ext]) -> rusqlite::Result<()> {
    let now = now_ms();
    app.db.tx(|tx| {
        for e in items {
            tx.execute(
                "INSERT INTO ext_titles (id, kind, title, original, year, rating, votes, genres, poster_url, updated_at, release)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET kind = ?2, title = ?3, original = COALESCE(?4, original),
                   year = COALESCE(?5, year), rating = COALESCE(?6, rating), votes = COALESCE(?7, votes),
                   genres = CASE WHEN ?8 = '[]' THEN genres ELSE ?8 END,
                   poster = CASE WHEN COALESCE(poster_url, '') NOT LIKE '%tmdb.org/%' AND poster_url IS NOT ?9 AND ?9 IS NOT NULL
                     AND json_extract(COALESCE(extra, '{}'), '$.picked.poster') IS NULL
                     THEN NULL ELSE poster END,
                   poster_url = CASE WHEN poster_url LIKE '%tmdb.org/%' THEN poster_url ELSE COALESCE(?9, poster_url) END,
                   updated_at = ?10, release = COALESCE(?11, release)",
                params![
                    e.id,
                    if e.series { "series" } else { "movie" },
                    e.title,
                    e.original,
                    e.year,
                    e.rating,
                    e.votes,
                    serde_json::to_string(&crate::meta::genres_ru(&e.genres)).unwrap_or_else(|_| "[]".into()),
                    e.poster_url,
                    now,
                    e.release
                ],
            )?;
        }
        Ok(())
    })
}

fn set_chart(app: &App, key: &str, items: &[Ext]) -> rusqlite::Result<()> {
    app.db.tx(|tx| {
        tx.execute("DELETE FROM ext_charts WHERE chart = ?", [key])?;
        for (i, e) in items.iter().enumerate() {
            tx.execute("INSERT INTO ext_charts (chart, pos, ext_id) VALUES (?, ?, ?)", params![key, i as i64, e.id])?;
        }
        Ok(())
    })
}

/// Обновить подборки с IMDb. Ошибка одной подборки не мешает остальным.
pub async fn refresh(app: &Arc<App>) -> anyhow::Result<usize> {
    let imdb = Imdb::new(app.http.clone());
    let mut done = 0;
    let mut last = None;
    for (key, _, src) in FEEDS {
        match tokio::time::timeout(Duration::from_secs(8), fetch_feed(&imdb, src, "POPULARITY"))
            .await
            .unwrap_or_else(|e| Err(e.into()))
        {
            Ok(v) => {
                let items = parse_chart(&v);
                if !items.is_empty() {
                    upsert(app, &items)?;
                    set_chart(app, key, &items)?;
                    done += 1;
                }
            }
            Err(e) => last = Some(e),
        }
        // IMDb не любит залпы: подборки - по очереди с паузой.
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    if done > 0 {
        app.db.set_setting("discover.at", &now_ms().to_string())?;
        app.db.set_setting("discover.feeds", &FEEDS.len().to_string())?;
        app.kick_ext();
        Ok(done)
    } else {
        Err(last.unwrap_or_else(|| anyhow::anyhow!("IMDb ничего не вернул")))
    }
}

/// Подборки - раз в 12 часов (первый раз - через минуту после запуска).
pub async fn worker(app: Arc<App>) {
    tokio::time::sleep(Duration::from_secs(2)).await;
    loop {
        let at: i64 = app.db.setting_or("discover.at", "0").parse().unwrap_or(0);
        // Новых подборок ещё нет (обновили программу) - сразу, не дожидаясь 12 часов.
        let fresh_set = app.db.setting_or("discover.feeds", "0") == FEEDS.len().to_string();
        if (now_ms() - at > EVERY.as_millis() as i64 || !fresh_set)
            && let Err(e) = refresh(&app).await
        {
            tracing::warn!(error = %e, "лента IMDb");
        }
        let _ = tokio::time::timeout(Duration::from_secs(300), app.discover_kick.notified()).await;
    }
}

/// Постеры ленты - пока есть нескачанные; затем дешёвая проверка раз в 5 минут.
pub async fn posters(app: Arc<App>) {
    loop {
        let n = posters_batch(&app, 20).await;
        if n == 0 {
            let _ = tokio::time::timeout(Duration::from_secs(300), app.posters_kick.notified()).await;
        } else {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
}

/// Русское название фильма ищем заново не раньше чем через 30 суток.
pub const TITLES_RETRY_MS: i64 = 30 * 86_400_000;

/// Фильмы ленты, которым ещё надо поискать русское название: нет кириллицы в названии, нет русской
/// записи (`title_localizations`), не искали недавно. Сначала - из подборок по порядку.
pub fn pending_titles(db: &crate::db::Db, limit: i64, now: i64) -> rusqlite::Result<Vec<String>> {
    db.with(|c| {
        let mut st = c.prepare(
            "SELECT e.id FROM ext_titles e
             WHERE e.title NOT GLOB '*[а-яА-ЯёЁ]*'
               AND NOT EXISTS (SELECT 1 FROM title_localizations l WHERE l.id = e.id)
               AND e.id LIKE 'tt%' AND (e.ru_at IS NULL OR e.ru_at < ?1 - ?2)
             ORDER BY (SELECT MIN(pos) FROM ext_charts ch WHERE ch.ext_id = e.id) IS NULL,
                      (SELECT MIN(pos) FROM ext_charts ch WHERE ch.ext_id = e.id), e.updated_at DESC
             LIMIT ?3",
        )?;
        let r = st.query_map(params![now, TITLES_RETRY_MS, limit], |r| r.get(0))?;
        r.collect()
    })
}

/// Запомнить, что русское название этих фильмов искали (нашлось или нет).
pub fn mark_tried(db: &crate::db::Db, ids: &[String], now: i64) -> rusqlite::Result<()> {
    let list = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    db.with(|c| {
        c.execute("UPDATE ext_titles SET ru_at = ?1 WHERE id IN (SELECT value FROM json_each(?2))", params![now, list])
            .map(|_| ())
    })
}

/// Русские названия подборок и поиска: пачками по 65, через существующий `catalog_search::localize`
/// (Викиданные -> `title_localizations`). Пустая настройка `names.wikidata.sparql` выключает запросы.
pub async fn titles_ru(app: Arc<App>) {
    tokio::time::sleep(Duration::from_secs(20)).await;
    loop {
        let enabled = app.db.setting("names.wikidata.sparql").is_none_or(|v| !v.is_empty());
        let todo = if enabled { pending_titles(&app.db, 65, now_ms()).unwrap_or_default() } else { vec![] };
        if todo.is_empty() {
            let _ = tokio::time::timeout(Duration::from_secs(300), app.titles_kick.notified()).await;
            continue;
        }
        match crate::catalog_search::localize(&app, &todo).await {
            Ok(()) => {
                let _ = mark_tried(&app.db, &todo, now_ms());
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => {
                // Сеть недоступна: не помечаем (не искали), повторим не скоро.
                tracing::debug!(error = %e, "русские названия ленты");
                tokio::time::sleep(Duration::from_secs(300)).await;
            }
        }
    }
}

/// Скачать до `limit` постеров (сначала - из подборок по порядку). Сколько пробовали.
pub async fn posters_batch(app: &Arc<App>, limit: i64) -> usize {
    let todo: Vec<(String, String)> = app
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT e.id, e.poster_url FROM ext_titles e WHERE e.poster IS NULL AND e.poster_url IS NOT NULL
                 ORDER BY (SELECT MIN(pos) FROM ext_charts ch WHERE ch.ext_id = e.id) IS NULL,
                          (SELECT MIN(pos) FROM ext_charts ch WHERE ch.ext_id = e.id), e.updated_at DESC
                 LIMIT ?",
            )?;
            let r = st.query_map([limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
            r.collect()
        })
        .unwrap_or_default();
    fetch_posters(app, todo).await
}

/// Постеры именно этих фильмов (ещё не скачанные).
pub async fn posters_of(app: &Arc<App>, ids: &[String]) -> usize {
    let list = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    let todo: Vec<(String, String)> = app
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT id, poster_url FROM ext_titles WHERE poster IS NULL AND poster_url IS NOT NULL
                   AND id IN (SELECT value FROM json_each(?))",
            )?;
            let r = st.query_map([list], |r| Ok((r.get(0)?, r.get(1)?)))?;
            r.collect()
        })
        .unwrap_or_default();
    fetch_posters(app, todo).await
}

/// Постер больше этого (байт) - уменьшаем до ширины карточки: запасные источники (TVmaze, Википедия)
/// отдают оригиналы по 2-3 МБ, а показываем мы 150-340 px.
const POSTER_MAX_BYTES: u64 = 200_000;

/// Постер для карточки: большой - уменьшить до 342 px (нет ffmpeg - оставить как есть).
pub async fn small_poster(app: &App, name: String) -> String {
    let Some(path) = app.images.path(&name).filter(|p| p.metadata().is_ok_and(|m| m.len() > POSTER_MAX_BYTES)) else {
        return name;
    };
    let ffmpeg = app.tools.read().ffmpeg.clone();
    let Some(ffmpeg) = ffmpeg else { return name };
    match crate::library::probe::scale_image(&ffmpeg, &path, 342).await.and_then(|b| app.images.save(&b)) {
        Ok(small) => small,
        Err(_) => name,
    }
}

/// Один раз после запуска: уже скачанные большие постеры ленты - уменьшить (понемногу, в фоне).
pub async fn shrink_posters(app: Arc<App>) {
    tokio::time::sleep(Duration::from_secs(60)).await;
    let all: Vec<(String, String)> = app
        .db
        .with(|c| {
            let mut st = c.prepare("SELECT id, poster FROM ext_titles WHERE poster IS NOT NULL AND poster != ''")?;
            let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            r.collect()
        })
        .unwrap_or_default();
    for (id, name) in all {
        let small = small_poster(&app, name.clone()).await;
        if small != name {
            let _ = app.db.with(|c| c.execute("UPDATE ext_titles SET poster = ? WHERE id = ?", params![small, id]));
        }
    }
}

async fn fetch_posters(app: &Arc<App>, todo: Vec<(String, String)>) -> usize {
    let jobs = todo.iter().map(|(id, url)| async move {
        // Не вышло - помечаем пустой строкой, чтобы не пробовать по кругу.
        let name = app.images.download(url).await.unwrap_or_default();
        let name = if name.is_empty() { name } else { small_poster(app, name).await };
        let _ = app.db.with(|c| c.execute("UPDATE ext_titles SET poster = ? WHERE id = ?", params![name, id]));
    });
    futures_util::future::join_all(jobs).await;
    todo.len()
}

/// Адрес картинки у источника: из прокси `/api/discover/art?url=…` или сам адрес (`http` -> `https`).
pub fn art_source(raw: &str) -> Option<String> {
    if let Some(rest) = raw.strip_prefix("http://") {
        return Some(format!("https://{rest}"));
    }
    if raw.starts_with("https://") {
        return Some(raw.into());
    }
    let u = reqwest::Url::parse("http://localhost").ok()?.join(raw).ok()?;
    (u.path() == "/api/discover/art").then(|| u.query_pairs().find(|(k, _)| k == "url").map(|(_, v)| v.into_owned()))?
}

/// Не локальная картинка: прокси или адрес источника (локальные - имя файла или `/img/…`).
fn is_remote_art(s: &str) -> bool {
    s.starts_with("/api/discover/art") || s.starts_with("http://") || s.starts_with("https://")
}

/// Скачать картинку к себе и уменьшить до `width` (нет ffmpeg - как есть). Имя файла в `images/`.
pub async fn keep_image(app: &App, raw: &str, width: u32) -> Option<String> {
    let src = art_source(raw)?;
    let name = tokio::time::timeout(Duration::from_secs(15), app.images.download(&src)).await.ok()?.ok()?;
    let ffmpeg = app.tools.read().ffmpeg.clone();
    if let (Some(ffmpeg), Some(path)) = (ffmpeg, app.images.path(&name))
        && let Ok(bytes) = crate::library::probe::scale_image(&ffmpeg, &path, width).await
        && let Ok(small) = app.images.save(&bytes)
    {
        return Some(small);
    }
    Some(name)
}

/// Все внешние картинки страницы фильма (`extra`) - к себе: (путь JSON, имя файла) для скачанных
/// и «всё ли скачалось».
async fn localize_extra(app: &App, extra: &Value) -> (Vec<(String, String)>, bool) {
    use futures_util::StreamExt;
    let mut jobs: Vec<(String, String, u32)> = vec![];
    let mut want = |path: String, v: &Value, width: u32| {
        if let Some(s) = v.as_str().filter(|s| is_remote_art(s)) {
            jobs.push((path, s.into(), width));
        }
    };
    want("$.poster".into(), &extra["poster"], 640);
    want("$.imdbBackdrop".into(), &extra["imdbBackdrop"], 1280);
    for (i, p) in extra["people"].as_array().into_iter().flatten().enumerate() {
        want(format!("$.people[{i}].photo"), &p["photo"], 342);
    }
    for (i, g) in extra["gallery"].as_array().into_iter().flatten().enumerate() {
        want(format!("$.gallery[{i}]"), g, 1280);
    }
    let total = jobs.len();
    let done: Vec<(String, Option<String>)> = futures_util::stream::iter(jobs)
        .map(|(path, raw, width)| async move { (path, keep_image(app, &raw, width).await) })
        .buffer_unordered(4)
        .collect()
        .await;
    let ok: Vec<(String, String)> = done.into_iter().filter_map(|(p, n)| n.map(|n| (p, n))).collect();
    let all = ok.len() == total;
    (ok, all)
}

/// Картинки одного фильма ленты - к себе (кадр, большой постер, фото актёров, кадры из фильма).
/// Что не скачалось - остаётся прежним (повторит `images_backfill`). `true` - всё уже у нас.
pub async fn localize_row(app: &App, id: &str) -> bool {
    let row: Option<(Option<String>, Option<String>)> = app
        .db
        .with(|c| {
            c.query_row("SELECT backdrop, extra FROM ext_titles WHERE id = ?", [id], |r| Ok((r.get(0)?, r.get(1)?)))
                .optional()
        })
        .unwrap_or(None);
    let Some((backdrop, extra)) = row else { return true };
    let extra: Value = extra.and_then(|e| serde_json::from_str(&e).ok()).unwrap_or(Value::Null);
    let (done, mut all) = localize_extra(app, &extra).await;
    for (path, name) in &done {
        let _ = app.db.with(|c| {
            c.execute(
                "UPDATE ext_titles SET extra = json_set(COALESCE(extra, '{}'), ?1, ?2) WHERE id = ?3",
                params![path, name, id],
            )
        });
    }
    if let Some(old) = backdrop.filter(|b| is_remote_art(b)) {
        match keep_image(app, &old, 1280).await {
            Some(name) => {
                let _ = app.db.with(|c| {
                    c.execute(
                        "UPDATE ext_titles SET backdrop = ?1 WHERE id = ?2 AND backdrop = ?3",
                        params![name, id, old],
                    )
                });
            }
            None => all = false,
        }
    }
    all
}

/// Фоновая докачка: картинки страниц фильмов, которые ещё ходят через прокси или адреса источников,
/// и постеры, не скачавшиеся раньше. Неудача - повтор через час, чтобы не молотить одно и то же.
pub async fn images_backfill(app: Arc<App>) {
    const RETRY_MS: i64 = 3_600_000;
    tokio::time::sleep(Duration::from_secs(45)).await;
    let mut retry: std::collections::HashMap<String, i64> = Default::default();
    loop {
        let now = now_ms();
        retry.retain(|_, at| *at > now);
        let pages: Vec<String> = app
            .db
            .with(|c| {
                let mut st = c.prepare(
                    "SELECT id FROM ext_titles
                     WHERE backdrop LIKE '/api/discover/art%' OR backdrop LIKE 'http%' OR extra LIKE '%/api/discover/art%'
                     ORDER BY updated_at DESC LIMIT 200",
                )?;
                let r = st.query_map([], |r| r.get(0))?;
                r.collect()
            })
            .unwrap_or_default();
        let pages: Vec<String> = pages.into_iter().filter(|i| !retry.contains_key(i)).take(6).collect();
        let posters: Vec<(String, String)> = app
            .db
            .with(|c| {
                let mut st = c.prepare(
                    "SELECT id, poster_url FROM ext_titles WHERE poster = '' AND poster_url IS NOT NULL LIMIT 200",
                )?;
                let r = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
                r.collect()
            })
            .unwrap_or_default();
        let posters: Vec<(String, String)> =
            posters.into_iter().filter(|(i, _)| !retry.contains_key(&format!("p:{i}"))).take(6).collect();
        if pages.is_empty() && posters.is_empty() {
            tokio::time::sleep(Duration::from_secs(600)).await;
            continue;
        }
        for id in pages {
            if !localize_row(&app, &id).await {
                retry.insert(id, now_ms() + RETRY_MS);
            }
        }
        for (id, _) in &posters {
            retry.insert(format!("p:{id}"), now_ms() + RETRY_MS);
        }
        fetch_posters(&app, posters).await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Что сейчас качается: два открытия одного фильма не идут к IMDb дважды.
static LOADING: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
    std::sync::LazyLock::new(Default::default);

/// Всё для страницы фильма из ленты - один раз: описание с IMDb, большой постер, кадр, актёры с
/// фото, кадры и похожие (с постерами). Картинки качаются вместе, браузер получит их у нас.
/// Что-то не вышло - сохраняем что есть (прежнее не теряем), `detailed` = 1 и время попытки:
/// повторим через `RETRY_MS`. Уже качается — `Ok(false)`.
pub async fn detail(app: &Arc<App>, id: &str) -> anyhow::Result<bool> {
    if !LOADING.lock().unwrap().insert(id.to_string()) {
        return Ok(false);
    }
    struct Done<'a>(&'a str);
    impl Drop for Done<'_> {
        fn drop(&mut self) {
            LOADING.lock().unwrap().remove(self.0);
        }
    }
    let _done = Done(id);
    let res = fetch_detail(app, id).await;
    if res.is_err() {
        let _ = app.db.with(|c| {
            c.execute(
                "UPDATE ext_titles SET extra = json_set(COALESCE(extra, '{}'), '$.at', ?) WHERE id = ?",
                params![now_ms(), id],
            )
        });
    }
    res.map(|()| true)
}

async fn fetch_detail(app: &Arc<App>, id: &str) -> anyhow::Result<()> {
    let imdb = Imdb::new(app.http.clone());
    let d = imdb.details(id, &[]).await?;
    let e = Ext {
        id: id.into(),
        series: d.series,
        title: d.title.clone(),
        original: d.original.clone(),
        year: d.year,
        rating: d.rating,
        votes: d.votes,
        genres: d.genres.clone(),
        poster_url: d.poster.as_deref().map(|u| sized(u, 342)),
        release: d.release_date.clone(),
    };
    upsert(app, &[e])?;
    // Есть ключ TMDB - русский постер вместо промо-плаката IMDb (upsert его потом не затирает).
    if let Some(tmdb) = crate::meta::tmdb_posters(app)
        && let Ok(Ok(Some(url))) = tokio::time::timeout(Duration::from_secs(4), tmdb.poster_ru(id)).await
    {
        app.db.with(|c| {
            c.execute(
                "UPDATE ext_titles SET poster_url = ?1, poster = NULL WHERE id = ?2 AND poster_url IS NOT ?1",
                params![url, id],
            )
        })?;
        app.kick_ext();
    }
    let mut extra = json!({"tagline":d.tagline,"age":d.age,"countries":d.countries,"languages":d.languages,"releaseDate":d.release_date,"aka":d.aka,
      "poster":d.poster.as_deref().map(crate::media_meta::art),"imdbBackdrop":d.backdrop.as_deref().map(crate::media_meta::art),"people":d.people.iter().take(16).map(|p|json!({"name":p.name,"job":p.job,"role":p.role,"photo":p.photo.as_deref().map(crate::media_meta::art),"id":p.id})).collect::<Vec<_>>(),
      "gallery":d.gallery.iter().take(6).map(|u|crate::media_meta::art(u)).collect::<Vec<_>>(),"similar":[],"seasons":d.seasons});
    if extra["seasons"].as_array().is_some_and(|s| s.is_empty()) {
        extra.as_object_mut().expect("extra object").remove("seasons");
    }
    app.db.with(|c|c.execute("UPDATE ext_titles SET overview=COALESCE(overview,?1),runtime=?2,backdrop=CASE WHEN json_extract(COALESCE(extra,'{}'),'$.picked.backdrop') IS NOT NULL THEN backdrop ELSE COALESCE(?3,NULLIF(backdrop,'')) END,extra=json_patch(COALESCE(extra,'{}'),?4),detailed=?5 WHERE id=?6",params![d.overview,d.runtime,d.backdrop.as_deref().map(crate::media_meta::art),extra.to_string(),DETAILED,id]))?;
    // Картинки страницы - к себе в фоне: страница откроется сразу, браузер потом возьмёт их у нас.
    {
        let a = app.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            localize_row(&a, &id).await;
        });
    }
    let a = app.clone();
    let id = id.to_string();
    let rich_job = format!("rich:{id}");
    app.detail_jobs.lock().insert(rich_job.clone());
    tokio::spawn(async move {
        let provider = Imdb::new(a.http.clone());
        let (ru, related) = tokio::join!(
            crate::ruwiki::overview(&a.http, &id),
            tokio::time::timeout(Duration::from_secs(8), provider.data(SIMILAR, json!({"id":id}), "title"))
        );
        if let Some(ru) = ru {
            let _ = a.db.with(|c| c.execute("UPDATE ext_titles SET overview=? WHERE id=?", params![ru, id]));
        }
        if let Ok(Ok(v)) = related {
            let items = parse_chart(&v["moreLikeThisTitles"]).into_iter().filter(|e| e.id != id).collect::<Vec<_>>();
            let _ = upsert(&a, &items);
            let _ = a.db.with(|c| {
                c.execute(
                    "UPDATE ext_titles SET extra=json_set(extra,'$.similar',json(?)) WHERE id=?",
                    params![json!(items.iter().map(|e| &e.id).collect::<Vec<_>>()).to_string(), id],
                )
            });
            a.kick_ext();
        }
        a.detail_jobs.lock().remove(&rich_job);
    });
    Ok(())
}

/// Сезон сериала из ленты уже есть и свежий: старые сезоны - навсегда, проверяем раз в неделю.
pub fn season_fresh(app: &App, id: &str, season: i32) -> bool {
    let at: Option<i64> = app
        .db
        .with(|c| {
            c.query_row("SELECT at FROM ext_seasons WHERE ext_id = ? AND season = ?", params![id, season], |r| r.get(0))
                .optional()
        })
        .ok()
        .flatten();
    at.is_some_and(|at| now_ms() - at < 7 * 86_400_000)
}

/// Серии одного сезона сериала из ленты - с IMDb (один раз в неделю), кадры - к себе.
pub async fn season(app: &Arc<App>, id: &str, season: i32) -> anyhow::Result<()> {
    season_data(app, id, season).await?;
    stills(app, id, season).await;
    Ok(())
}

/// Только серии сезона (названия, оценки, даты) - без кадров (сетке оценок кадры не нужны).
/// true - сезон обновился.
pub async fn season_data(app: &Arc<App>, id: &str, season: i32) -> anyhow::Result<bool> {
    if season_fresh(app, id, season) {
        return Ok(false);
    }
    let imdb = Imdb::new(app.http.clone());
    let v = imdb.data(crate::meta::imdb::EPISODES, json!({ "id": id, "season": season.to_string() }), "title").await?;
    let eps = crate::meta::imdb::parse_episodes(&v);
    app.db.tx(|tx| {
        for e in &eps {
            tx.execute(
                "INSERT INTO ext_episodes (ext_id, season, episode, name, overview, still_url, air_date, runtime, rating)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(ext_id, season, episode) DO UPDATE SET name = ?4, overview = ?5,
                   still = CASE WHEN still_url IS NOT ?6 THEN NULL ELSE still END, still_url = ?6,
                   air_date = ?7, runtime = ?8, rating = ?9",
                params![id, e.season, e.episode, e.name, e.overview, e.still, e.air_date, e.runtime, e.rating],
            )?;
        }
        tx.execute(
            "INSERT INTO ext_seasons (ext_id, season, at) VALUES (?, ?, ?)
             ON CONFLICT(ext_id, season) DO UPDATE SET at = excluded.at",
            params![id, season, now_ms()],
        )?;
        Ok(())
    })?;
    Ok(true)
}

/// Кадры серий сезона, которых ещё нет у нас (по 6 разом, не дольше 12 с; остальные - в следующий раз).
pub async fn stills(app: &Arc<App>, id: &str, season: i32) {
    let todo: Vec<(i32, String)> = app
        .db
        .with(|c| {
            let mut st = c.prepare(
                "SELECT episode, still_url FROM ext_episodes
                 WHERE ext_id = ? AND season = ? AND (still IS NULL OR still = '') AND still_url IS NOT NULL ORDER BY episode",
            )?;
            let r = st.query_map(params![id, season], |r| Ok((r.get(0)?, r.get(1)?)))?;
            r.collect()
        })
        .unwrap_or_default();
    let work = futures_util::stream::iter(todo)
        .map(|(ep, url)| async move {
            let Ok(name) = app.images.download(&url).await else { return };
            let _ = app.db.with(|c| {
                c.execute(
                    "UPDATE ext_episodes SET still = ? WHERE ext_id = ? AND season = ? AND episode = ?",
                    params![name, id, season, ep],
                )
            });
        })
        .buffer_unordered(6)
        .collect::<Vec<_>>();
    let _ = tokio::time::timeout(Duration::from_secs(12), work).await;
}

/// Карточка фильма из ленты для интерфейса.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtCard {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub original: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub poster: Option<String>,
    pub genres: Vec<String>,
    /// Есть на сервисе: id фильма у нас.
    pub local: Option<i64>,
    /// «Буду смотреть» / «Просмотрел».
    pub mark: Option<String>,
    /// Просили хоста добавить.
    pub wished: bool,
    /// Дата выхода (см. Ext::release); в будущем - фильм ещё не вышел.
    pub release: Option<String>,
    /// Администратор отметил фильм «Ещё не вышел» вручную.
    pub soon: bool,
}

pub fn card_sql(kids: bool) -> String {
    format!(
        "SELECT e.id, e.kind, COALESCE((SELECT title FROM title_localizations l WHERE l.id=e.id),e.title),
           COALESCE(e.original,CASE WHEN EXISTS(SELECT 1 FROM title_localizations l WHERE l.id=e.id AND l.title!=e.title) THEN e.title END), e.year, e.rating,
           COALESCE(NULLIF(e.poster, ''),'/api/discover/poster/'||e.id), e.genres,
           (SELECT t.id FROM titles t WHERE t.imdb_id = e.id AND {} LIMIT 1),
           CASE WHEN EXISTS (SELECT 1 FROM ext_dropped d WHERE d.ext_id = e.id AND d.profile_id = ?1) THEN 'dropped'
             ELSE (SELECT m.status FROM ext_marks m WHERE m.ext_id = e.id AND m.profile_id = ?1) END,
           EXISTS (SELECT 1 FROM wishes w WHERE w.ext_id = e.id AND w.profile_id = ?1 AND w.dismissed_at IS NULL AND w.done_at IS NULL),
           e.release, e.soon
         FROM ext_titles e",
        crate::api::visible(kids)
    )
}

pub fn card(r: &rusqlite::Row) -> rusqlite::Result<ExtCard> {
    let id: String = r.get(0)?;
    let raw_title: String = r.get(2)?;
    let title = if raw_title.chars().any(|c| matches!(c, 'А'..='я' | 'Ё' | 'ё')) {
        raw_title.clone()
    } else {
        crate::site_sources::SOURCES
            .iter()
            .find(|s| s.imdb == Some(id.as_str()))
            .and_then(|s| s.title)
            .map(String::from)
            .or_else(|| match id.as_str() {
                "tt9458304" => Some("Дороро".into()),
                "tt0468569" => Some("Тёмный рыцарь".into()),
                _ => None,
            })
            .unwrap_or(raw_title)
    };
    Ok(ExtCard {
        id: r.get(0)?,
        kind: r.get(1)?,
        title,
        original: r.get(3)?,
        year: r.get(4)?,
        rating: r.get(5)?,
        poster: r
            .get::<_, Option<String>>(6)?
            .and_then(|s| if s.starts_with('/') { Some(s) } else { crate::api::img(Some(s)) }),
        genres: crate::meta::genres_ru(crate::api::json_list(r.get(7)?)),
        local: r.get(8)?,
        mark: r.get(9)?,
        wished: r.get(10)?,
        release: r.get(11)?,
        soon: r.get(12)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_ext(db: &crate::db::Db, id: &str, title: &str, ru_at: Option<i64>) {
        db.with(|c| {
            c.execute(
                "INSERT INTO ext_titles (id, kind, title, updated_at, ru_at) VALUES (?, 'movie', ?, 0, ?)",
                params![id, title, ru_at],
            )
        })
        .unwrap();
    }

    #[test]
    fn pending_titles_skip_cyrillic_localized_and_recent() {
        let db = crate::db::Db::memory().unwrap();
        let now = 100 * TITLES_RETRY_MS;
        add_ext(&db, "tt9000001", "Ghost Rider", None);
        add_ext(&db, "tt9000002", "Дьявол носит Prada 2", None);
        add_ext(&db, "tt9000003", "Street Fighter", None);
        db.with(|c| c.execute("INSERT INTO title_localizations VALUES ('tt9000003', 'Уличный боец', '{}', 0)", []))
            .unwrap();
        add_ext(&db, "tt9000004", "Winter", Some(now - 1000));
        add_ext(&db, "tt9000005", "Marble Hall Murders", Some(now - TITLES_RETRY_MS - 1));
        let mut got = pending_titles(&db, 100, now).unwrap();
        got.sort();
        assert_eq!(got, ["tt9000001", "tt9000005"]);
    }

    #[test]
    fn mark_tried_sets_ru_at() {
        let db = crate::db::Db::memory().unwrap();
        add_ext(&db, "tt9000001", "Ghost Rider", None);
        mark_tried(&db, &["tt9000001".to_string()], 12345).unwrap();
        let at: Option<i64> =
            db.with(|c| c.query_row("SELECT ru_at FROM ext_titles WHERE id = 'tt9000001'", [], |r| r.get(0))).unwrap();
        assert_eq!(at, Some(12345));
        assert!(pending_titles(&db, 10, 12346).unwrap().is_empty());
    }

    #[test]
    fn genre_queries() {
        let g = |q: &str| genre_query(q).map(|(en, _, t)| (en, t.len()));
        assert_eq!(g("Фантастика"), Some(("Sci-Fi", 3)));
        assert_eq!(g("комедии"), Some(("Comedy", 3)));
        assert_eq!(g("сериалы ужасы"), Some(("Horror", 2)));
        assert_eq!(g("лучшие фильмы про войну"), Some(("War", 1)));
        assert_eq!(g("военные фильмы"), Some(("War", 1)));
        assert_eq!(g("мультсериалы"), Some(("Animation", 2)));
        assert_eq!(g("Тёмный рыцарь"), None);
        assert_eq!(g("фантастика драма"), None);
        assert_eq!(g("фильмы"), None);
    }

    #[test]
    fn parses_charts() {
        let v = json!({"edges": [
            {"currentRank": 1, "node": {"id": "tt0111161", "titleText": {"text": "Побег из Шоушенка"},
              "originalTitleText": {"text": "The Shawshank Redemption"}, "titleType": {"id": "movie", "isSeries": false},
              "releaseYear": {"year": 1994}, "ratingsSummary": {"aggregateRating": 9.3, "voteCount": 3000000},
              "primaryImage": {"url": "https://m.media-amazon.com/images/M/x._V1_.jpg"},
              "genres": {"genres": [{"text": "Drama"}]}}},
            {"currentRank": 2, "node": {"id": "tt0903747", "titleText": {"text": "Breaking Bad"},
              "originalTitleText": {"text": "Breaking Bad"}, "titleType": {"id": "tvSeries", "isSeries": true},
              "releaseYear": {"year": 2008}}},
            {"node": {"id": "tt1", "titleText": {"text": "плохой id"}}},
            {"node": {"id": "tt1234567", "titleText": {"text": "Серия"}, "titleType": {"id": "tvEpisode"}}}
        ]});
        let c = parse_chart(&v);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].original.as_deref(), Some("The Shawshank Redemption"));
        assert!(c[0].poster_url.as_deref().unwrap().contains("UX342"));
        assert_eq!((c[1].series, c[1].original.as_deref()), (true, None));
        assert!(valid_id("tt0111161") && !valid_id("tt12") && !valid_id("../etc"));
    }

    #[test]
    fn dates_and_feeds() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(951_782_400_000), "2000-02-29");
        assert_eq!(ymd(1_792_281_600_000), "2026-10-18");
        // Ключи уникальны и с понятным разделом: вкладки берут `*_movies` и `*_series`.
        let keys: std::collections::HashSet<_> = FEEDS.iter().map(|f| f.0).collect();
        assert_eq!(keys.len(), FEEDS.len());
        assert!(FEEDS.iter().all(|f| f.0.ends_with("_movies") || f.0.ends_with("_series")));
    }

    #[test]
    fn release_dates_keep_the_precision_imdb_gives() {
        assert_eq!(release_of(&json!({"year": 2027, "month": 3, "day": 5})).as_deref(), Some("2027-03-05"));
        assert_eq!(release_of(&json!({"year": 2027, "month": 11, "day": null})).as_deref(), Some("2027-11"));
        assert_eq!(release_of(&json!({"year": 2028})).as_deref(), Some("2028"));
        assert_eq!(release_of(&json!(null)), None);
    }
}

#[cfg(test)]
mod art_source_tests {
    use super::{art_source, is_remote_art};

    #[test]
    fn source_is_taken_from_proxy_or_upgraded_to_https() {
        let proxy = crate::media_meta::art("https://m.media-amazon.com/images/a.jpg?x=1&y=2");
        assert_eq!(art_source(&proxy).as_deref(), Some("https://m.media-amazon.com/images/a.jpg?x=1&y=2"));
        assert_eq!(art_source("http://static.tvmaze.com/a.jpg").as_deref(), Some("https://static.tvmaze.com/a.jpg"));
        assert_eq!(art_source("https://m.media-amazon.com/a.jpg").as_deref(), Some("https://m.media-amazon.com/a.jpg"));
        assert_eq!(art_source("abc123.jpg"), None);
        assert_eq!(art_source("/img/abc123.jpg"), None);
    }

    #[test]
    fn local_names_are_not_remote() {
        assert!(is_remote_art("/api/discover/art?url=x"));
        assert!(is_remote_art("https://a/b.jpg"));
        assert!(!is_remote_art("abc123.jpg"));
        assert!(!is_remote_art("/img/abc123.jpg"));
    }
}
