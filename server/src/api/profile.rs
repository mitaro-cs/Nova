//! Личная страница: статистика (часы, фильмы, серии, любимые жанры, по месяцам) и оформление -
//! обложка, свой цвет, «о себе», любимые фильмы (с сервиса и из ленты IMDb).

use crate::App;
use crate::auth::Viewer;
use crate::error::{ApiError, ApiResult};
use crate::util::now_ms;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

const MONTH: i64 = 30 * 86_400_000;

const DAY: i64 = 86_400_000;

#[derive(Deserialize)]
pub struct PersonSuggestQuery {
    q: String,
}

pub async fn person_suggest(
    State(app): State<Arc<App>>,
    _viewer: Viewer,
    Query(query): Query<PersonSuggestQuery>,
) -> ApiResult<Json<Value>> {
    let q = query.q.trim().to_lowercase();
    if q.is_empty() || q.chars().count() > 80 {
        return Ok(Json(json!([])));
    }
    let entries: Vec<(String, String)> = app.db.with(|c| {
        let mut statement = c.prepare("SELECT ru, COALESCE(orig, '') FROM person_names ORDER BY at DESC LIMIT 5000")?;
        let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    })?;
    let mut found = Vec::new();
    for (ru, orig) in entries {
        if (ru.to_lowercase().contains(&q) || orig.to_lowercase().contains(&q)) && !found.contains(&ru) {
            found.push(ru);
            if found.len() == 12 {
                break;
            }
        }
    }
    Ok(Json(json!(found)))
}

/// Кто видит раздел профиля: все, друзья или только сам человек.
pub const LEVELS: [&str; 3] = ["all", "friends", "me"];

/// Приватность разделов «Статистика», «Просмотрено» и «Отзывы».
pub struct Visibility {
    pub stats: String,
    pub watched: String,
    pub reviews: String,
}

pub fn visibility(app: &App, id: i64) -> rusqlite::Result<Visibility> {
    app.db.with(|c| {
        c.query_row("SELECT vis_stats, vis_watched, vis_reviews FROM profiles WHERE id = ?", [id], |r| {
            Ok(Visibility { stats: r.get(0)?, watched: r.get(1)?, reviews: r.get(2)? })
        })
    })
}

/// Может ли `viewer` видеть раздел `owner` при таком уровне. Хост и администраторы - как все: приватность одна для всех.
pub fn allowed(app: &App, viewer: i64, owner: i64, level: &str) -> bool {
    viewer == owner
        || level == "all"
        || (level == "friends" && super::social::relation(app, viewer, owner).is_ok_and(|r| r == "friend"))
}

/// Статистика человека. `now` - для подсчёта по месяцам (последние 12); `tz_min` - смещение часового пояса зрителя
/// в минутах (восток - плюс), по нему считаются «когда смотрит», дни недели и серия дней подряд.
pub fn stats(app: &App, id: i64, now: i64, tz_min: i64) -> ApiResult<Value> {
    let (minutes, movies, episodes, series, reviews, friends, outside): (f64, i64, i64, i64, i64, i64, i64) =
        app.db.with(|c| {
            c.query_row(
                "SELECT
                   COALESCE((SELECT SUM(MIN(p.position, p.duration)) FROM progress p WHERE p.profile_id = ?1), 0) / 60.0,
                   (SELECT COUNT(*) FROM watched w JOIN titles t ON t.id = w.title_id WHERE w.profile_id = ?1 AND t.kind = 'movie'),
                   (SELECT COUNT(*) FROM progress p JOIN files f ON f.id = p.file_id JOIN titles t ON t.id = f.title_id
                      WHERE p.profile_id = ?1 AND p.done = 1 AND t.kind = 'series'),
                   (SELECT COUNT(DISTINCT f.title_id) FROM progress p JOIN files f ON f.id = p.file_id JOIN titles t ON t.id = f.title_id
                      WHERE p.profile_id = ?1 AND t.kind = 'series'),
                   (SELECT COUNT(*) FROM reviews WHERE profile_id = ?1),
                   (SELECT COUNT(*) FROM friends WHERE status = 'accepted' AND (a = ?1 OR b = ?1)),
                   (SELECT COUNT(*) FROM ext_marks WHERE profile_id = ?1 AND status = 'watched')",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            )
        })?;
    // Источники по ссылке раньше вовсе не попадали в статистику. Несколько источников одной
    // серии считаем один раз; фильм, уже учтённый через локальный файл, не дублируем.
    let external_minutes: f64 = app.db.with(|c| {
        c.query_row(
            "SELECT COALESCE(SUM(pos),0)/60.0 FROM (
          SELECT MAX(MIN(p.position,p.duration)) pos FROM link_progress p JOIN links l ON l.id=p.link_id
          WHERE p.profile_id=?1 AND NOT EXISTS(SELECT 1 FROM progress x JOIN files f ON f.id=x.file_id
            WHERE x.profile_id=?1 AND f.title_id=l.title_id)
          GROUP BY COALESCE(l.ext_id,'t:'||l.title_id,'l:'||l.id),l.season,l.episode)",
            [id],
            |r| r.get(0),
        )
    })?;
    let (external_movies, external_episodes, external_series): (i64,i64,i64) = app.db.with(|c| c.query_row(
        "SELECT
          (SELECT COUNT(*) FROM ext_marks m JOIN ext_titles e ON e.id=m.ext_id WHERE m.profile_id=?1 AND m.status='watched' AND e.kind='movie'
            AND NOT EXISTS(SELECT 1 FROM watched w JOIN titles t ON t.id=w.title_id WHERE w.profile_id=?1 AND t.imdb_id=e.id)),
          (SELECT COUNT(*) FROM ext_seen s WHERE s.profile_id=?1 AND NOT EXISTS(SELECT 1 FROM progress p JOIN files f ON f.id=p.file_id JOIN titles t ON t.id=f.title_id
            WHERE p.profile_id=?1 AND p.done=1 AND t.imdb_id=s.ext_id AND f.season=s.season AND f.episode=s.episode)),
          (SELECT COUNT(DISTINCT s.ext_id) FROM ext_seen s WHERE s.profile_id=?1 AND NOT EXISTS(SELECT 1 FROM progress p JOIN files f ON f.id=p.file_id JOIN titles t ON t.id=f.title_id
            WHERE p.profile_id=?1 AND t.imdb_id=s.ext_id))", [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))))?;
    // Жанры - по всему, что смотрел (с сервиса) и отметил просмотренным в ленте.
    let genre_lists: Vec<String> = app.db.with(|c| {
        let mut st = c.prepare(
            "SELECT t.genres FROM titles t WHERE t.id IN (
               SELECT f.title_id FROM progress p JOIN files f ON f.id = p.file_id WHERE p.profile_id = ?1
               UNION SELECT title_id FROM watched WHERE profile_id = ?1)
             UNION ALL
             SELECT e.genres FROM ext_titles e JOIN ext_marks m ON m.ext_id = e.id AND m.profile_id = ?1 AND m.status = 'watched'",
        )?;
        let r = st.query_map([id], |r| r.get::<_, Option<String>>(0))?;
        r.map(|g| g.map(|g| g.unwrap_or_default())).collect()
    })?;
    // Режиссёры того, что смотрел, - чаще всего первыми (подсказки для «любимых режиссёров»).
    let peoples: Vec<String> = app.db.with(|c| {
        let mut st = c.prepare(
            "SELECT t.people FROM titles t WHERE t.id IN (
               SELECT f.title_id FROM progress p JOIN files f ON f.id = p.file_id WHERE p.profile_id = ?1
               UNION SELECT title_id FROM watched WHERE profile_id = ?1)",
        )?;
        let r = st.query_map([id], |r| r.get(0))?;
        r.collect()
    })?;
    let mut dirs: HashMap<String, i64> = HashMap::new();
    let mut cast: HashMap<String, i64> = HashMap::new();
    for p in &peoples {
        if let Ok(Value::Array(list)) = serde_json::from_str::<Value>(p) {
            for x in &list {
                let Some(n) = x["name"].as_str() else { continue };
                match x["job"].as_str() {
                    Some("director") => *dirs.entry(n.to_string()).or_default() += 1,
                    Some("actor") => *cast.entry(n.to_string()).or_default() += 1,
                    _ => {}
                }
            }
        }
    }
    let mut dirs: Vec<(String, i64)> = dirs.into_iter().collect();
    dirs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    dirs.truncate(10);
    let mut cast: Vec<(String, i64)> = cast.into_iter().collect();
    cast.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    cast.truncate(12);
    let mut genres: HashMap<String, i64> = HashMap::new();
    for g in genre_lists.iter().flat_map(|s| super::json_list(Some(s.clone()))) {
        *genres.entry(g).or_default() += 1;
    }
    let mut genres: Vec<(String, i64)> = genres.into_iter().collect();
    genres.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    genres.truncate(6);
    // По месяцам: досмотренное (фильмы и серии) за последние 12 месяцев, старое - слева.
    let done: Vec<i64> = app.db.with(|c| {
        let mut st = c.prepare("SELECT MAX(at) FROM (
          SELECT COALESCE(t.imdb_id,'t:'||t.id)||':'||COALESCE(f.season,0)||':'||COALESCE(f.episode,0) item,p.updated_at at
            FROM progress p JOIN files f ON f.id=p.file_id JOIN titles t ON t.id=f.title_id WHERE p.profile_id=?1 AND p.done=1
          UNION ALL SELECT COALESCE(t.imdb_id,'t:'||t.id)||':0:0',w.at FROM watched w JOIN titles t ON t.id=w.title_id WHERE w.profile_id=?1 AND t.kind='movie'
          UNION ALL SELECT s.ext_id||':'||s.season||':'||s.episode,s.at FROM ext_seen s WHERE s.profile_id=?1
          UNION ALL SELECT m.ext_id||':0:0',m.at FROM ext_marks m JOIN ext_titles e ON e.id=m.ext_id WHERE m.profile_id=?1 AND m.status='watched' AND e.kind='movie'
        ) GROUP BY item HAVING MAX(at)>?2")?;
        let r = st.query_map(params![id, now - 13 * MONTH], |r| r.get(0))?;
        r.collect()
    })?;
    let mut months = [0i64; 12];
    let month_index = |at: i64| -> rusqlite::Result<i32> {
        app.db.with(|c| c.query_row(
        "SELECT CAST(strftime('%Y',?1/1000,'unixepoch') AS INTEGER)*12 + CAST(strftime('%m',?1/1000,'unixepoch') AS INTEGER)-1", [at], |r| r.get(0)))
    };
    let current = month_index(now)?;
    for &at in &done {
        let ago = current - month_index(at)?;
        if (0..12).contains(&ago) {
            months[11 - ago as usize] += 1;
        }
    }
    let labels = (0..12)
        .map(|i| {
            let m = current - 11 + i;
            format!("{:04}-{:02}", m / 12, m % 12 + 1)
        })
        .collect::<Vec<_>>();
    // «Смотрит больше, чем N% зрителей»: сравнение по времени с сервиса среди тех, у кого оно есть.
    // Меньше трёх других зрителей - сравнивать не с кем, доли нет.
    let (others, fewer): (i64, i64) = app.db.with(|c| {
        c.query_row(
            "SELECT COUNT(*), COALESCE(SUM(m < ?1), 0) FROM (
               SELECT SUM(MIN(position, duration)) / 60.0 AS m FROM progress WHERE profile_id != ?2
               GROUP BY profile_id HAVING m > 0)",
            params![minutes, id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
    })?;
    let better = (minutes > 0.0 && others >= 3).then(|| (fewer as f64 / others as f64 * 100.0).round() as i64);
    // Когда смотрит: по времени последнего просмотра каждой досмотренной вещи за год, в часовом поясе зрителя.
    let mut dayparts = [0i64; 4];
    let mut weekdays = [0i64; 7];
    let mut days: BTreeSet<i64> = BTreeSet::new();
    for &at in &done {
        let local = at + tz_min * 60_000;
        let hour = local.rem_euclid(DAY) / 3_600_000;
        // Утро 5-11, день 11-17, вечер 17-23, ночь - остальное.
        dayparts[match hour {
            5..=10 => 0,
            11..=16 => 1,
            17..=22 => 2,
            _ => 3,
        }] += 1;
        let day = local.div_euclid(DAY);
        // 1 января 1970 - четверг: при понедельнике = 0 день 0 - это 3.
        weekdays[(day + 3).rem_euclid(7) as usize] += 1;
        days.insert(day);
    }
    let (mut best, mut run, mut prev) = (0i64, 0i64, i64::MIN);
    for &d in &days {
        run = if prev != i64::MIN && d == prev + 1 { run + 1 } else { 1 };
        best = best.max(run);
        prev = d;
    }
    let today = (now + tz_min * 60_000).div_euclid(DAY);
    let (mut streak, mut d) = (0i64, if days.contains(&today) { today } else { today - 1 });
    while days.contains(&d) {
        streak += 1;
        d -= 1;
    }
    // Оценки: звёзды в отзывах, 1..10 (половинки).
    let stars: Vec<i64> = app.db.with(|c| {
        let mut st = c.prepare("SELECT stars FROM reviews WHERE profile_id = ? AND stars IS NOT NULL")?;
        let r = st.query_map([id], |r| r.get(0))?;
        r.collect()
    })?;
    let mut hist = [0i64; 10];
    for s in &stars {
        if (1..=10).contains(s) {
            hist[(*s - 1) as usize] += 1;
        }
    }
    let avg =
        (!stars.is_empty()).then(|| (stars.iter().sum::<i64>() as f64 / stars.len() as f64 * 10.0).round() / 10.0);
    // Эпохи: десятилетия просмотренных фильмов и сериалов.
    let years: Vec<i64> = app.db.with(|c| {
        let mut st = c.prepare(
            "SELECT year FROM (
               SELECT t.year FROM titles t WHERE t.year IS NOT NULL AND t.id IN (
                 SELECT title_id FROM watched WHERE profile_id = ?1
                 UNION SELECT f.title_id FROM progress p JOIN files f ON f.id = p.file_id WHERE p.profile_id = ?1 AND p.done = 1)
               UNION ALL
               SELECT e.year FROM ext_titles e JOIN ext_marks m ON m.ext_id = e.id AND m.profile_id = ?1 AND m.status = 'watched'
                 WHERE e.year IS NOT NULL AND NOT EXISTS (SELECT 1 FROM titles t WHERE t.imdb_id = e.id))",
        )?;
        let r = st.query_map([id], |r| r.get(0))?;
        r.collect()
    })?;
    let mut decades: BTreeMap<i64, i64> = BTreeMap::new();
    for y in years {
        *decades.entry(y.div_euclid(10) * 10).or_default() += 1;
    }
    Ok(json!({
        "better": better,
        "dayparts": dayparts, "weekdays": weekdays, "streak": {"best": best, "current": streak},
        "ratings": {"avg": avg, "hist": hist, "count": stars.len()},
        "decades": decades.into_iter().map(|(d, n)| json!({"decade": d, "count": n})).collect::<Vec<_>>(),
        "actors": cast.into_iter().map(|(n, c)| json!({"name": crate::names::translit(&n), "count": c})).collect::<Vec<_>>(),
        "hours": ((minutes + external_minutes) / 60.0 * 10.0).round() / 10.0,
        "movies": movies + external_movies, "episodes": episodes + external_episodes, "series": series + external_series, "reviews": reviews, "friends": friends,
        "outside": outside,
        "genres": genres.into_iter().map(|(g, n)| json!({"genre": g, "count": n})).collect::<Vec<_>>(),
        "months": months,
        "monthLabels": labels,
        "directors": dirs.into_iter().map(|(n, c)| json!({"name": n, "count": c})).collect::<Vec<_>>(),
    }))
}

/// Оформление профиля с любимыми фильмами (карточки).
pub fn decor(app: &App, v: &Viewer, id: i64) -> ApiResult<Value> {
    type Row = (Option<String>, Option<String>, Option<String>, String, String, String, bool);
    let (cover, accent, bio, favs, directors, kind, show): Row = app.db.with(|c| {
        c.query_row(
            "SELECT cover, accent, bio, favorites, directors, cover_kind, show_activity FROM profiles WHERE id = ?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
    })?;
    let vis = visibility(app, id)?;
    let actors_raw: String =
        app.db.with(|c| c.query_row("SELECT actors FROM profiles WHERE id = ?", [id], |r| r.get(0)))?;
    let see_watched = allowed(app, v.profile.id, id, &vis.watched);
    // Лента обложки - постеры просмотренного (с сервиса и из Обзора), свежие первыми.
    let ribbon: Vec<String> = if !see_watched {
        Vec::new()
    } else {
        app.db.with(|c| {
            let mut st = c.prepare(&format!(
                "SELECT p FROM (
               SELECT COALESCE(t.poster_sm, t.poster) p, w.at FROM watched w JOIN titles t ON t.id = w.title_id
                 WHERE w.profile_id = ?1 AND {}
               UNION ALL
               SELECT e.poster, m.at FROM ext_marks m JOIN ext_titles e ON e.id = m.ext_id
                 WHERE m.profile_id = ?1 AND m.status = 'watched')
             WHERE p IS NOT NULL AND p != '' ORDER BY at DESC LIMIT 72",
                super::visible(v.profile.kids)
            ))?;
            let r = st.query_map([id], |r| r.get::<_, String>(0))?;
            r.collect()
        })?
    };
    let mut favorites = Vec::new();
    for f in super::json_list(Some(favs)) {
        if let Some(t) = f.strip_prefix("t:").and_then(|t| t.parse::<i64>().ok()) {
            if let Some(c) = super::cards_by_ids(app, v.profile.kids, &[t])?.pop() {
                favorites.push(json!({"key": f, "card": c}));
            }
        } else if let Some(e) = f.strip_prefix("x:")
            && let Ok(c) = super::discover::ext_card(app, v, e)
        {
            favorites.push(json!({"key": f, "ext": c}));
        }
    }
    Ok(json!({
        "cover": super::img(cover), "coverKind": kind, "accent": accent, "bio": bio, "favorites": favorites,
        "directors": super::json_list(Some(directors.clone())), "showActivity": show,
        "likedPeople": liked_people(app, id)?,
        // Фото режиссёров - что уже известно (без запросов к IMDb; недостающее ищет `directors::warm`); сколько его
        // фильмов человек смотрел - только если «Просмотрено» открыто зрителю.
        "directorsInfo": super::json_list(Some(directors)).into_iter().map(|n| {
            let photo = crate::directors::cached(app, &n).and_then(|c| c.1);
            let seen = if see_watched { crate::directors::seen_stats(app, &n, id) } else { None };
            json!({"name": n, "photo": super::img(photo), "seen": seen})
        }).collect::<Vec<_>>(),
        // Любимые актёры и актрисы: имя и фото, что уже известно.
        "actors": super::json_list(Some(actors_raw)).into_iter().map(|n| {
            let photo = crate::directors::cached(app, &n).and_then(|c| c.1);
            json!({"name": n, "photo": super::img(photo)})
        }).collect::<Vec<_>>(),
        // Настройки приватности видит только сам человек.
        "visibility": (v.profile.id == id).then(|| json!({"stats": vis.stats, "watched": vis.watched, "reviews": vis.reviews})),
        "ribbon": ribbon.into_iter().filter_map(|n| super::img(Some(n))).collect::<Vec<_>>(),
    }))
}

#[derive(Deserialize)]
pub struct DecorBody {
    bio: Option<String>,
    directors: Option<Vec<String>>,
    #[serde(rename = "coverKind")]
    cover_kind: Option<String>,
    accent: Option<String>,
    favorites: Option<Vec<String>>,
    /// Показывать друзьям, что смотрю.
    #[serde(rename = "showActivity")]
    show_activity: Option<bool>,
    /// Любимые актёры и актрисы (до `MAX_ACTORS`).
    actors: Option<Vec<String>>,
    /// Кто видит разделы: all | friends | me.
    #[serde(rename = "visStats")]
    vis_stats: Option<String>,
    #[serde(rename = "visWatched")]
    vis_watched: Option<String>,
    #[serde(rename = "visReviews")]
    vis_reviews: Option<String>,
}

/// Любимых актёров и актрис в личном профиле - до пяти.
pub const MAX_ACTORS: usize = 5;

/// Любимых в личном профиле: любых пять фильмов и любых пять сериалов (отдельно друг от друга).
pub const MAX_FAVORITES: usize = 5;
/// Любимый режиссёр в личном профиле - один; «нравятся» можно ставить любым актёрам и режиссёрам
/// (`liked_people`), а любимый - один.
pub const MAX_DIRECTORS: usize = 1;

/// Фильм или сериал у ключа любимого («t:12» - с сервиса, «x:tt…» - из ленты).
fn fav_kind(app: &App, key: &str) -> Option<String> {
    let (sql, id) = if let Some(t) = key.strip_prefix("t:") {
        ("SELECT kind FROM titles WHERE id = ?", t)
    } else {
        ("SELECT kind FROM ext_titles WHERE id = ?", key.strip_prefix("x:")?)
    };
    app.db.with(|c| c.query_row(sql, [id], |r| r.get(0))).ok()
}

/// Слишком много любимых одного вида - ошибка с понятным текстом.
fn check_favorites(app: &App, list: &[String]) -> ApiResult<()> {
    // Неизвестное (фильма уже нет) - считаем фильмом: иначе любимых можно было бы набрать сколько угодно.
    let kinds: Vec<Option<String>> = list.iter().map(|k| fav_kind(app, k).or(Some("movie".into()))).collect();
    for (kind, what) in [("movie", "фильмов"), ("series", "сериалов")] {
        if kinds.iter().filter(|k| k.as_deref() == Some(kind)).count() > MAX_FAVORITES {
            return Err(ApiError::bad("too_many", format!("В избранном можно сохранить до {MAX_FAVORITES} {what}")));
        }
    }
    Ok(())
}

fn valid_key(k: &str) -> bool {
    k.strip_prefix("t:").is_some_and(|t| t.parse::<i64>().is_ok())
        || k.strip_prefix("x:").is_some_and(crate::discover::valid_id)
}

#[derive(Deserialize)]
pub struct FavBody {
    key: String,
    value: bool,
}

/// Один фильм в любимые или из них (с его страницы). Шестой фильм (или сериал) - нельзя: сначала убрать один.
pub async fn set_favorite(State(app): State<Arc<App>>, v: Viewer, Json(b): Json<FavBody>) -> ApiResult<Json<Value>> {
    if !valid_key(&b.key) {
        return Err(ApiError::bad("bad_key", "Неизвестный фильм"));
    }
    let me = v.profile.id;
    let raw: String =
        app.db.with(|c| c.query_row("SELECT favorites FROM profiles WHERE id = ?", [me], |r| r.get(0)))?;
    let mut list = super::json_list(Some(raw));
    list.retain(|k| k != &b.key);
    if b.value {
        list.push(b.key.clone());
        check_favorites(&app, &list)?;
    }
    let s = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
    app.db.with(|c| c.execute("UPDATE profiles SET favorites = ? WHERE id = ?", params![s, me]))?;
    Ok(Json(json!({"favorites": list})))
}

/// Актёры и режиссёры, которым человек поставил «нравится» (новые первыми), с фото из того, что уже известно.
pub fn liked_people(app: &App, id: i64) -> ApiResult<Vec<Value>> {
    let rows: Vec<(String, String)> = app.db.with(|c| {
        let mut st =
            c.prepare("SELECT name, role FROM liked_people WHERE profile_id = ? ORDER BY at DESC LIMIT 200")?;
        let r = st.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        r.collect()
    })?;
    Ok(rows
        .into_iter()
        .map(|(name, role)| {
            let photo = crate::directors::cached(app, &name).and_then(|c| c.1);
            // `name` - как сохранено (по нему ищется страница человека), `nameRu` - для подписи.
            json!({"name": name, "nameRu": crate::names::ru_director(&app.db, &name), "role": role, "photo": super::img(photo)})
        })
        .collect())
}

pub async fn my_people(State(app): State<Arc<App>>, v: Viewer) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"liked": liked_people(&app, v.profile.id)?, "directors": director_names(&app, v.profile.id)?})))
}

fn director_names(app: &App, id: i64) -> ApiResult<Vec<String>> {
    let raw: String =
        app.db.with(|c| c.query_row("SELECT directors FROM profiles WHERE id = ?", [id], |r| r.get(0)))?;
    Ok(super::json_list(Some(raw)))
}

#[derive(Deserialize)]
pub struct PersonLike {
    name: String,
    role: String,
    value: bool,
}

/// «Нравится» актёру или режиссёру (со страницы человека) и обратно.
pub async fn like_person(State(app): State<Arc<App>>, v: Viewer, Json(b): Json<PersonLike>) -> ApiResult<Json<Value>> {
    let name = b.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(ApiError::bad("bad_name", "Имя - от 1 до 80 символов"));
    }
    if b.role != "actor" && b.role != "director" {
        return Err(ApiError::bad("bad_role", "Актёр или режиссёр"));
    }
    let me = v.profile.id;
    app.db.with(|c| {
        if b.value {
            c.execute(
                "INSERT OR IGNORE INTO liked_people (profile_id, name, role, at) VALUES (?, ?, ?, ?)",
                params![me, name, b.role, now_ms()],
            )?;
        } else {
            c.execute(
                "DELETE FROM liked_people WHERE profile_id = ? AND name = ? AND role = ?",
                params![me, name, b.role],
            )?;
        }
        Ok::<(), rusqlite::Error>(())
    })?;
    Ok(Json(json!({"liked": liked_people(&app, me)?})))
}

#[derive(Deserialize)]
pub struct WatchedQuery {
    kind: Option<String>,
}

/// Что человек смотрел: фильмы или сериалы (с сервиса и из Обзора), свежее первым. Чужое - только если
/// человек не скрыл, что смотрит.
pub async fn watched(
    State(app): State<Arc<App>>,
    v: Viewer,
    Path(id): Path<i64>,
    Query(q): Query<WatchedQuery>,
) -> ApiResult<Json<Value>> {
    let kind = if q.kind.as_deref() == Some("series") { "series" } else { "movie" };
    let show: bool = app
        .db
        .with(|c| c.query_row("SELECT show_activity FROM profiles WHERE id = ?", [id], |r| r.get(0)).optional())?
        .ok_or_else(ApiError::not_found)?;
    if id != v.profile.id && !show {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "hidden", "Человек скрыл, что смотрит"));
    }
    if !allowed(&app, v.profile.id, id, &visibility(&app, id)?.watched) {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "hidden", "Скрыто владельцем"));
    }
    let local: Vec<i64> = app.db.with(|c| {
        let mut st = c.prepare(
            "SELECT t.id FROM titles t WHERE t.kind = ?2 AND (
               EXISTS (SELECT 1 FROM watched w WHERE w.title_id = t.id AND w.profile_id = ?1)
               OR EXISTS (SELECT 1 FROM progress p JOIN files f ON f.id = p.file_id WHERE f.title_id = t.id AND p.profile_id = ?1))
             ORDER BY COALESCE((SELECT MAX(w.at) FROM watched w WHERE w.title_id = t.id AND w.profile_id = ?1), 0) DESC, t.id DESC
             LIMIT 300",
        )?;
        let r = st.query_map(params![id, kind], |r| r.get(0))?;
        r.collect()
    })?;
    let ext: Vec<String> = app.db.with(|c| {
        let mut st = c.prepare(
            "SELECT x.id FROM (
               SELECT m.ext_id id, m.at at FROM ext_marks m WHERE m.profile_id = ?1 AND m.status IN ('watched', 'watching')
               UNION ALL SELECT s.ext_id, MAX(s.at) FROM ext_seen s WHERE s.profile_id = ?1 GROUP BY s.ext_id) x
             JOIN ext_titles e ON e.id = x.id
             WHERE e.kind = ?2 AND NOT EXISTS (SELECT 1 FROM titles t WHERE t.imdb_id = e.id)
             GROUP BY x.id ORDER BY MAX(x.at) DESC LIMIT 300",
        )?;
        let r = st.query_map(params![id, kind], |r| r.get(0))?;
        r.collect()
    })?;
    let cards = super::cards_by_ids(&app, v.profile.kids, &local)?;
    let ext_cards: Vec<_> = ext.iter().filter_map(|e| super::discover::ext_card(&app, &v, e).ok()).collect();
    Ok(Json(json!({"kind": kind, "local": cards, "ext": ext_cards})))
}

fn valid_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

pub async fn set_decor(State(app): State<Arc<App>>, v: Viewer, Json(b): Json<DecorBody>) -> ApiResult<Json<Value>> {
    let me = v.profile.id;
    if let Some(bio) = &b.bio {
        let bio = bio.trim();
        if bio.chars().count() > 300 {
            return Err(ApiError::bad("too_long", "«О себе» - не длиннее 300 символов"));
        }
        app.db.with(|c| {
            c.execute("UPDATE profiles SET bio = ? WHERE id = ?", params![(!bio.is_empty()).then_some(bio), me])
        })?;
    }
    if let Some(a) = &b.accent {
        let a = a.trim().to_lowercase();
        if !a.is_empty() && !valid_color(&a) {
            return Err(ApiError::bad("bad_color", "Цвет - в виде #ff8800"));
        }
        app.db.with(|c| {
            c.execute("UPDATE profiles SET accent = ? WHERE id = ?", params![(!a.is_empty()).then_some(a), me])
        })?;
    }
    if let Some(d) = &b.directors {
        let mut keep: Vec<String> = Vec::new();
        for n in d.iter().map(|n| n.trim()).filter(|n| !n.is_empty()) {
            if n.chars().count() > 80 {
                return Err(ApiError::bad("too_long", "Имя режиссёра - не длиннее 80 символов"));
            }
            if !keep.iter().any(|k| k.to_lowercase() == n.to_lowercase()) {
                keep.push(n.to_string());
            }
        }
        if keep.len() > MAX_DIRECTORS {
            return Err(ApiError::bad("too_many", "Любимый режиссёр - один: сначала уберите прежнего"));
        }
        let s = serde_json::to_string(&keep).unwrap_or_else(|_| "[]".into());
        app.db.with(|c| c.execute("UPDATE profiles SET directors = ? WHERE id = ?", params![s, me]))?;
    }
    if let Some(a) = &b.actors {
        let mut keep: Vec<String> = Vec::new();
        for n in a.iter().map(|n| n.trim()).filter(|n| !n.is_empty()) {
            if n.chars().count() > 80 {
                return Err(ApiError::bad("too_long", "Имя - не длиннее 80 символов"));
            }
            if !keep.iter().any(|k| k.to_lowercase() == n.to_lowercase()) {
                keep.push(n.to_string());
            }
        }
        if keep.len() > MAX_ACTORS {
            return Err(ApiError::bad(
                "too_many",
                format!("Любимых актёров и актрис - не больше {MAX_ACTORS}: сначала уберите кого-то"),
            ));
        }
        let s = serde_json::to_string(&keep).unwrap_or_else(|_| "[]".into());
        app.db.with(|c| c.execute("UPDATE profiles SET actors = ? WHERE id = ?", params![s, me]))?;
        crate::directors::warm_role(&app, keep, "actor");
    }
    for (col, value) in [("vis_stats", &b.vis_stats), ("vis_watched", &b.vis_watched), ("vis_reviews", &b.vis_reviews)]
    {
        if let Some(level) = value {
            if !LEVELS.contains(&level.as_str()) {
                return Err(ApiError::bad("bad_value", "Видят: все, друзья или только я"));
            }
            app.db.with(|c| c.execute(&format!("UPDATE profiles SET {col} = ? WHERE id = ?"), params![level, me]))?;
        }
    }
    if let Some(k) = &b.cover_kind {
        if k != "ribbon" && k != "image" {
            return Err(ApiError::bad("bad_value", "Обложка - лента или своя картинка"));
        }
        app.db.with(|c| c.execute("UPDATE profiles SET cover_kind = ? WHERE id = ?", params![k, me]))?;
    }
    if let Some(s) = b.show_activity {
        app.db.with(|c| c.execute("UPDATE profiles SET show_activity = ? WHERE id = ?", params![s, me]))?;
    }
    if let Some(f) = &b.favorites {
        let mut keep: Vec<String> = Vec::new();
        for k in f {
            if valid_key(k) && !keep.contains(k) {
                keep.push(k.clone());
            }
        }
        check_favorites(&app, &keep)?;
        let s = serde_json::to_string(&keep).unwrap_or_else(|_| "[]".into());
        app.db.with(|c| c.execute("UPDATE profiles SET favorites = ? WHERE id = ?", params![s, me]))?;
    }
    Ok(Json(json!({"ok": true})))
}

/// Обложка профиля: картинка, уменьшенная до 1600 px по ширине.
pub async fn set_cover(State(app): State<Arc<App>>, v: Viewer, body: axum::body::Bytes) -> ApiResult<Json<Value>> {
    if body.len() > crate::images::MAX_IMAGE {
        return Err(ApiError::bad("too_big", "Картинка больше 15 МБ"));
    }
    if crate::images::sniff(&body).is_none() {
        return Err(ApiError::bad("bad_image", "Нужна картинка: JPEG, PNG или WebP"));
    }
    let a = app.clone();
    let raw = crate::images::strip_metadata(&body);
    let name = tokio::task::spawn_blocking(move || a.images.save(&raw))
        .await
        .map_err(ApiError::internal)?
        .map_err(|e| ApiError::bad("bad_image", e.to_string()))?;
    let ffmpeg = app.tools.read().ffmpeg.clone();
    let name = match (ffmpeg, app.images.path(&name)) {
        (Some(ff), Some(src)) => match crate::library::probe::scale_image(&ff, &src, 1600).await {
            Ok(b) => app.images.save(&b).unwrap_or(name),
            Err(_) => name,
        },
        _ => name,
    };
    app.db.with(|c| {
        c.execute("UPDATE profiles SET cover = ?, cover_kind = 'image' WHERE id = ?", params![name, v.profile.id])
    })?;
    Ok(Json(json!({"cover": format!("/img/{name}")})))
}

pub async fn delete_cover(State(app): State<Arc<App>>, v: Viewer) -> ApiResult<Json<Value>> {
    app.db
        .with(|c| c.execute("UPDATE profiles SET cover = NULL, cover_kind = 'ribbon' WHERE id = ?", [v.profile.id]))?;
    Ok(Json(json!({"ok": true})))
}
