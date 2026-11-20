//! Русские имена людей и персонажей: IMDb отдаёт латиницу. Перевод накладывается при отдаче
//! страницы (в базе остаются оригинальные имена): кэш `person_names` -> Викиданные -> транслитерация.

use crate::db::Db;
use crate::util::now_ms;
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Транслитерация - временная: через сутки её заменяет перевод из Викиданных, если он появился.
pub const RETRY_MS: i64 = 86_400_000;

pub struct Cached {
    pub ru: String,
    /// `wikidata` или `translit`.
    pub source: String,
    pub at: i64,
}

/// Что уже известно по id людей («nm…»).
pub fn cached(db: &Db, ids: &[String]) -> HashMap<String, Cached> {
    let mut out = HashMap::new();
    for id in ids {
        let row = db.with(|c| {
            c.prepare_cached("SELECT ru, source, at FROM person_names WHERE id = ?")?
                .query_row([id], |r| Ok(Cached { ru: r.get(0)?, source: r.get(1)?, at: r.get(2)? }))
                .optional()
        });
        if let Ok(Some(c)) = row {
            out.insert(id.clone(), c);
        }
    }
    out
}

/// Записать (id, русское имя, источник, оригинальное написание).
pub fn store(db: &Db, rows: &[(String, String, &str, String)]) -> anyhow::Result<()> {
    let at = now_ms();
    db.tx(|tx| {
        for (id, ru, source, orig) in rows {
            tx.execute(
                "INSERT INTO person_names (id, ru, source, at, orig) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET ru = ?2, source = ?3, at = ?4, orig = ?5",
                params![id, ru, source, at, orig],
            )?;
        }
        Ok(())
    })?;
    Ok(())
}

/// Режиссёры своих фильмов с латинским именем и id IMDb, которых ещё нет в кэше имён: (id, имя).
pub fn pending_directors(db: &Db, limit: i64) -> rusqlite::Result<Vec<(String, String)>> {
    db.with(|c| {
        let mut st = c.prepare(
            "SELECT DISTINCT json_extract(p.value, '$.id'), json_extract(p.value, '$.name')
             FROM titles t, json_each(t.people) p
             WHERE json_extract(p.value, '$.job') = 'director'
               AND json_extract(p.value, '$.id') LIKE 'nm%'
               AND json_extract(p.value, '$.name') GLOB '*[A-Za-z]*'
               AND NOT EXISTS (SELECT 1 FROM person_names n WHERE n.id = json_extract(p.value, '$.id'))
             LIMIT ?1",
        )?;
        let r = st.query_map([limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
        r.collect()
    })
}

/// Фоновый прогрев кэша имён режиссёров своих фильмов: к первому показу карточки там уже русское
/// имя из Викиданных, а не транслитерация. По 50 за раз, пауза между проходами.
pub async fn warm_directors(app: std::sync::Arc<crate::App>) {
    tokio::time::sleep(std::time::Duration::from_secs(90)).await;
    loop {
        let endpoint = app.db.setting("names.wikidata.sparql").unwrap_or_else(|| WIKIDATA.to_string());
        let todo = if endpoint.is_empty() { vec![] } else { pending_directors(&app.db, 50).unwrap_or_default() };
        if todo.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(900)).await;
            continue;
        }
        let mut people: Vec<crate::meta::Person> = todo
            .into_iter()
            .map(|(id, name)| crate::meta::Person { name, job: "director".into(), id: Some(id), ..Default::default() })
            .collect();
        localize_with(&app.db, &app.http, &endpoint, &mut people).await;
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}

/// Имя режиссёра на карточке по-русски без сетевых запросов: из кэша по оригинальному написанию,
/// иначе транслитерация. Кириллица возвращается как есть.
pub fn ru_director(db: &Db, name: &str) -> String {
    if !has_latin(name) {
        return name.to_string();
    }
    let hit: Option<String> = db
        .with(|c| c.query_row("SELECT ru FROM person_names WHERE orig = ?1 LIMIT 1", [name], |r| r.get(0)).optional())
        .ok()
        .flatten();
    hit.unwrap_or_else(|| translit(name))
}

/// Адрес SPARQL Викиданных; настройка `names.wikidata.sparql`: пустая - Викиданные выключены.
pub const WIKIDATA: &str = "https://query.wikidata.org/sparql";

/// «nm» и цифры - только такие id попадают в запрос.
pub fn valid_id(id: &str) -> bool {
    id.len() >= 3 && id.len() <= 12 && id.starts_with("nm") && id[2..].bytes().all(|b| b.is_ascii_digit())
}

pub fn sparql(ids: &[String]) -> String {
    let values: Vec<String> = ids.iter().filter(|i| valid_id(i)).map(|i| format!("\"{i}\"")).collect();
    format!(
        "SELECT ?imdb ?ru WHERE {{ VALUES ?imdb {{ {} }} ?item wdt:P345 ?imdb. \
         ?item rdfs:label ?ru FILTER(LANG(?ru) = \"ru\") }}",
        values.join(" ")
    )
}

/// id IMDb -> русская метка (метка без кириллицы - не перевод).
pub fn parse_wikidata(v: &serde_json::Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for b in v["results"]["bindings"].as_array().into_iter().flatten() {
        if let (Some(id), Some(ru)) = (b["imdb"]["value"].as_str(), b["ru"]["value"].as_str())
            && valid_id(id)
            && crate::ruwiki::has_cyrillic(ru)
        {
            out.insert(id.to_string(), ru.to_string());
        }
    }
    out
}

/// Русские имена людей по id IMDb одним запросом (до 3 секунд). Пустой адрес - сразу пустой ответ.
pub async fn wikidata_ru(
    http: &reqwest::Client,
    endpoint: &str,
    ids: &[String],
) -> anyhow::Result<HashMap<String, String>> {
    if endpoint.is_empty() || !ids.iter().any(|i| valid_id(i)) {
        return Ok(HashMap::new());
    }
    let url = reqwest::Url::parse_with_params(endpoint, &[("query", sparql(ids))])?;
    let v: serde_json::Value = http
        .get(url)
        .header("user-agent", concat!("Nova/", env!("CARGO_PKG_VERSION"), " (home cinema)"))
        .header("accept", "application/sparql-results+json")
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(parse_wikidata(&v))
}

/// Транслитерацию пора перепроверить в Викиданных.
pub fn due(c: &Cached, now: i64) -> bool {
    c.source == "translit" && now - c.at > RETRY_MS
}

/// Русские имена людей и персонажей на странице фильма (настройка `names.wikidata.sparql`).
pub async fn localize(app: &crate::App, people: &mut [crate::meta::Person]) {
    let endpoint = app.db.setting("names.wikidata.sparql").unwrap_or_else(|| WIKIDATA.to_string());
    localize_with(&app.db, &app.http, &endpoint, people).await;
}

/// Подготовить имена для немедленного ответа из кэша/транслитерации и обновить кэш в фоне.
/// Возвращает true, пока для кого-либо ещё ожидается перевод из Викиданных.
pub fn localize_fast(app: &std::sync::Arc<crate::App>, people: &mut [crate::meta::Person]) -> bool {
    let originals = people.to_vec();
    let ids: Vec<String> = people
        .iter()
        .filter_map(|p| p.id.as_ref().filter(|id| valid_id(id)).cloned())
        .collect();
    let known = cached(&app.db, &ids);
    let now = now_ms();
    let mut pending = false;
    for p in people.iter_mut() {
        if has_latin(&p.name) {
            if let Some(id) = p.id.as_ref().filter(|id| valid_id(id)) {
                match known.get(id) {
                    Some(c) => {
                        pending |= due(c, now);
                        if p.name != c.ru { p.orig = Some(std::mem::replace(&mut p.name, c.ru.clone())); }
                    }
                    None => { pending = true; p.orig = Some(p.name.clone()); p.name = translit(&p.name); }
                }
            } else {
                p.name = translit(&p.name);
            }
        }
        if let Some(role) = p.role.as_mut().filter(|r| has_latin(r)) { *role = translit(role); }
    }
    if pending {
        static JOBS: OnceLock<parking_lot::Mutex<std::collections::HashSet<String>>> = OnceLock::new();
        let key = ids.join(",");
        let jobs = JOBS.get_or_init(Default::default);
        if jobs.lock().insert(key.clone()) {
            let app = app.clone();
            let mut people = originals;
            tokio::spawn(async move {
                localize(&app, &mut people).await;
                jobs.lock().remove(&key);
            });
        }
    }
    pending
}

/// Имя человека - из кэша или Викиданных (один запрос на всех, до 3 с), иначе транслитерация; роль -
/// транслитерация. Оригинал - в `orig`. Сбой сети не мешает: останется транслитерация.
pub async fn localize_with(db: &Db, http: &reqwest::Client, endpoint: &str, people: &mut [crate::meta::Person]) {
    let mut names: HashMap<String, String> = HashMap::new();
    for p in people.iter().filter(|p| has_latin(&p.name)) {
        if let Some(id) = p.id.as_ref().filter(|i| valid_id(i)) {
            names.entry(id.clone()).or_insert_with(|| p.name.clone());
        }
    }
    let ids: Vec<String> = names.keys().cloned().collect();
    let mut known = cached(db, &ids);
    let now = now_ms();
    let stale: Vec<String> = ids.iter().filter(|i| known.get(*i).is_none_or(|c| due(c, now))).cloned().collect();
    if !stale.is_empty() {
        let found = wikidata_ru(http, endpoint, &stale).await.unwrap_or_default();
        let mut rows: Vec<(String, String, &str, String)> = Vec::new();
        for id in &stale {
            let orig = names[id].clone();
            if let Some(ru) = found.get(id) {
                rows.push((id.clone(), ru.clone(), "wikidata", orig));
            } else if let Some(old) = known.get(id) {
                rows.push((id.clone(), old.ru.clone(), "translit", orig));
            } else {
                rows.push((id.clone(), translit(&orig), "translit", orig));
            }
        }
        if store(db, &rows).is_ok() {
            for (id, ru, source, _) in rows {
                known.insert(id, Cached { ru, source: source.into(), at: now });
            }
        }
    }
    for p in people.iter_mut() {
        if has_latin(&p.name) {
            let ru =
                p.id.as_ref().and_then(|i| known.get(i)).map(|c| c.ru.clone()).unwrap_or_else(|| translit(&p.name));
            if ru != p.name {
                p.orig = Some(std::mem::replace(&mut p.name, ru));
            }
        }
        if let Some(role) = p.role.as_mut().filter(|r| has_latin(r)) {
            *role = translit(role);
        }
    }
}

pub fn has_latin(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_alphabetic())
}

/// Английские имена, которые правила пишут неверно (ключ - в нижнем регистре).
const FIRST: &[(&str, &str)] = &[
    ("kyle", "Кайл"),
    ("claire", "Клэр"),
    ("jason", "Джейсон"),
    ("corey", "Кори"),
    ("neil", "Нил"),
    ("ed", "Эд"),
    ("peter", "Питер"),
    ("john", "Джон"),
    ("james", "Джеймс"),
    ("michael", "Майкл"),
    ("david", "Дэвид"),
    ("robert", "Роберт"),
    ("william", "Уильям"),
    ("richard", "Ричард"),
    ("thomas", "Томас"),
    ("charles", "Чарльз"),
    ("daniel", "Дэниел"),
    ("matthew", "Мэттью"),
    ("anthony", "Энтони"),
    ("mark", "Марк"),
    ("paul", "Пол"),
    ("steven", "Стивен"),
    ("stephen", "Стивен"),
    ("andrew", "Эндрю"),
    ("joshua", "Джошуа"),
    ("kevin", "Кевин"),
    ("brian", "Брайан"),
    ("george", "Джордж"),
    ("edward", "Эдвард"),
    ("ryan", "Райан"),
    ("jacob", "Джейкоб"),
    ("gary", "Гэри"),
    ("nicholas", "Николас"),
    ("eric", "Эрик"),
    ("jonathan", "Джонатан"),
    ("larry", "Ларри"),
    ("justin", "Джастин"),
    ("scott", "Скотт"),
    ("brandon", "Брэндон"),
    ("benjamin", "Бенджамин"),
    ("samuel", "Сэмюэл"),
    ("patrick", "Патрик"),
    ("jack", "Джек"),
    ("henry", "Генри"),
    ("tom", "Том"),
    ("tim", "Тим"),
    ("sam", "Сэм"),
    ("ben", "Бен"),
    ("joe", "Джо"),
    ("jim", "Джим"),
    ("bill", "Билл"),
    ("bob", "Боб"),
    ("chris", "Крис"),
    ("christopher", "Кристофер"),
    ("alex", "Алекс"),
    ("adam", "Адам"),
    ("jeff", "Джефф"),
    ("jeremy", "Джереми"),
    ("jesse", "Джесси"),
    ("mary", "Мэри"),
    ("jennifer", "Дженнифер"),
    ("emma", "Эмма"),
    ("emily", "Эмили"),
    ("olivia", "Оливия"),
    ("sophia", "София"),
    ("anna", "Анна"),
    ("amy", "Эми"),
    ("kate", "Кейт"),
    ("katie", "Кэти"),
    ("julia", "Джулия"),
    ("julie", "Джули"),
    ("jessica", "Джессика"),
    ("sarah", "Сара"),
    ("laura", "Лора"),
    ("rachel", "Рэйчел"),
    ("rebecca", "Ребекка"),
    ("elizabeth", "Элизабет"),
    ("margaret", "Маргарет"),
    ("natalie", "Натали"),
    ("scarlett", "Скарлетт"),
    ("jean", "Жан"),
    ("claude", "Клод"),
    ("zoe", "Зои"),
    ("mia", "Миа"),
    ("diane", "Дайан"),
    ("cate", "Кейт"),
    ("keira", "Кира"),
    ("joaquin", "Хоакин"),
    ("quentin", "Квентин"),
    ("xavier", "Ксавье"),
    ("wes", "Уэс"),
    ("woody", "Вуди"),
    ("tony", "Тони"),
    ("buzz", "Базз"),
    ("deke", "Дик"),
    ("frank", "Фрэнк"),
    ("harry", "Гарри"),
    ("leo", "Лео"),
    ("liam", "Лиам"),
    ("max", "Макс"),
    ("nick", "Ник"),
    ("oliver", "Оливер"),
    ("ray", "Рэй"),
    ("roy", "Рой"),
];

fn first_name(w: &str) -> Option<&'static str> {
    let lower = w.to_ascii_lowercase();
    FIRST.iter().find(|(k, _)| *k == lower).map(|(_, v)| *v)
}

fn single(c: char) -> &'static str {
    match c {
        'a' => "а",
        'b' => "б",
        'c' => "к",
        'd' => "д",
        'e' => "е",
        'f' => "ф",
        'g' => "г",
        'h' => "х",
        'i' => "и",
        'j' => "дж",
        'k' => "к",
        'l' => "л",
        'm' => "м",
        'n' => "н",
        'o' => "о",
        'p' => "п",
        'q' => "к",
        'r' => "р",
        's' => "с",
        't' => "т",
        'u' => "у",
        'v' => "в",
        'w' => "в",
        'x' => "кс",
        'y' => "и",
        'z' => "з",
        _ => "",
    }
}

/// Одно слово из латинских букв (строчные) - по правилам: диграфы, немая «e» на конце, «c» перед e/i/y.
fn map_word(chars: &[char]) -> String {
    let n = chars.len();
    let at = |k: usize| chars.get(k).copied().unwrap_or('\0');
    let vowel = |c: char| "aeiouy".contains(c);
    let mut out = String::new();
    let mut i = 0;
    while i < n {
        let (c, nx, nn) = (chars[i], at(i + 1), at(i + 2));
        let (s, len): (&str, usize) = match (c, nx, nn) {
            ('t', 'c', 'h') => ("ч", 3),
            ('c', 'h', _) => ("ч", 2),
            ('s', 'h', _) => ("ш", 2),
            ('t', 'h', _) => ("т", 2),
            ('p', 'h', _) => ("ф", 2),
            ('c', 'k', _) => ("к", 2),
            ('k', 'h', _) => ("х", 2),
            ('z', 'h', _) => ("ж", 2),
            ('o', 'o', _) => ("у", 2),
            ('e', 'e', _) => ("и", 2),
            ('q', 'u', _) => ("кв", 2),
            ('d', 'g', _) => ("дж", 2),
            ('a', 'y', _) => ("ей", 2),
            ('e', 'y', _) => ("и", 2),
            ('o', 'y', _) => ("ой", 2),
            ('w', 'h', _) => ("в", 2),
            ('i', 'e', _) if i + 2 == n => ("и", 2),
            ('e', _, _) if i == n - 1 && n > 2 && !vowel(chars[i - 1]) => ("", 1),
            ('e', _, _) if i == 0 => ("э", 1),
            ('c', 'e' | 'i' | 'y', _) => ("с", 1),
            _ => (single(c), 1),
        };
        out.push_str(s);
        i += len;
    }
    out
}

fn capitalize(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        Some(f) => f.to_uppercase().chain(it).collect(),
        None => String::new(),
    }
}

/// Латиницу - русской записью; кириллица, иероглифы, пробелы, цифры и знаки остаются как есть.
pub fn translit(name: &str) -> String {
    // Слова - подряд идущие латинские буквы; разделяют их любые другие символы (в том числе « ' » и « - »).
    let chars: Vec<char> = name.chars().collect();
    let mut words: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            words.push((start, i));
        } else {
            i += 1;
        }
    }
    let mut out = String::new();
    let mut at = 0;
    let count = words.len();
    for (k, (a, b)) in words.into_iter().enumerate() {
        out.extend(&chars[at..a]);
        let word: String = chars[a..b].iter().collect();
        // Имена из словаря - только не последнее слово (последнее - фамилия), одно слово - тоже имя.
        let ru = match first_name(&word).filter(|_| k + 1 < count || count == 1) {
            Some(known) => known.to_string(),
            None => {
                let lower: Vec<char> = word.to_ascii_lowercase().chars().collect();
                let mapped = map_word(&lower);
                if word.len() > 1 && word.chars().all(|c| c.is_ascii_uppercase()) {
                    mapped.to_uppercase()
                } else if chars[a].is_ascii_uppercase() {
                    capitalize(&mapped)
                } else {
                    mapped
                }
            }
        };
        out.push_str(&ru);
        at = b;
    }
    out.extend(&chars[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_names_roundtrip() {
        let db = crate::db::Db::memory().unwrap();
        store(&db, &[("nm1".into(), "Райан Гослинг".into(), "wikidata", "Ryan Gosling".into())]).unwrap();
        store(&db, &[("nm1".into(), "Райан Гослинг".into(), "translit", "Ryan Gosling".into())]).unwrap();
        let got = cached(&db, &["nm1".into(), "nm2".into()]);
        assert_eq!(got.len(), 1);
        assert_eq!(got["nm1"].source, "translit");
        assert_eq!(got["nm1"].ru, "Райан Гослинг");
    }

    #[test]
    fn only_nm_ids_reach_sparql() {
        let q = sparql(&["nm0331516".into(), "nm1\" } DROP".into(), "tt0111161".into()]);
        assert!(q.contains("\"nm0331516\""));
        assert!(!q.contains("DROP"));
        assert!(!q.contains("tt0111161"));
    }

    #[test]
    fn wikidata_answer_is_parsed() {
        let v = serde_json::json!({"results": {"bindings": [
            {"imdb": {"value": "nm0331516"}, "ru": {"value": "Райан Гослинг"}},
            {"imdb": {"value": "nm0001"}, "ru": {"value": "Ryan X"}}]}});
        let m = parse_wikidata(&v);
        assert_eq!(m["nm0331516"], "Райан Гослинг");
        assert!(!m.contains_key("nm0001"));
    }

    fn person(name: &str, role: Option<&str>, id: Option<&str>) -> crate::meta::Person {
        crate::meta::Person {
            name: name.into(),
            role: role.map(String::from),
            job: "actor".into(),
            id: id.map(String::from),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn localize_uses_cache_then_translit() {
        let db = crate::db::Db::memory().unwrap();
        store(&db, &[("nm0331516".into(), "Райан Гослинг".into(), "wikidata", "Ryan Gosling".into())]).unwrap();
        let mut people = vec![
            person("Ryan Gosling", Some("Neil Armstrong"), Some("nm0331516")),
            person("Kyle Chandler", Some("Deke Slayton"), Some("nm0151419")),
            person("Tom Hardy", None, None),
        ];
        localize_with(&db, &reqwest::Client::new(), "", &mut people).await;
        assert_eq!(people[0].name, "Райан Гослинг");
        assert_eq!(people[0].orig.as_deref(), Some("Ryan Gosling"));
        assert_eq!(people[0].role.as_deref(), Some("Нил Армстронг"));
        assert_eq!(people[1].name, "Кайл Чандлер");
        assert_eq!(people[2].name, "Том Харди");
        assert_eq!(people[2].orig.as_deref(), Some("Tom Hardy"));
        let got = cached(&db, &["nm0151419".into()]);
        assert_eq!(got["nm0151419"].source, "translit");
    }

    #[tokio::test]
    async fn localize_skips_cyrillic_and_other_scripts() {
        let db = crate::db::Db::memory().unwrap();
        let mut people = vec![person("Тим Роббинс", None, Some("nm0000209")), person("봉준호", None, None)];
        localize_with(&db, &reqwest::Client::new(), "", &mut people).await;
        assert_eq!(people[0].name, "Тим Роббинс");
        assert!(people[0].orig.is_none());
        assert_eq!(people[1].name, "봉준호");
        assert!(people[1].orig.is_none());
    }

    #[test]
    fn pending_directors_lists_only_uncached_latin_ones() {
        let db = crate::db::Db::memory().unwrap();
        let people =
            |name: &str, id: &str| serde_json::json!([{"name": name, "job": "director", "id": id}]).to_string();
        db.with(|c| {
            for (key, p) in [
                ("a", people("Damien Chazelle", "nm2467416")),
                ("b", people("Kyle Chandler", "nm0151419")),
                ("c", people("Тим Роббинс", "nm0000209")),
                ("d", serde_json::json!([{"name": "Tom Hardy", "job": "director"}]).to_string()),
            ] {
                c.execute(
                    "INSERT INTO titles(kind,key,title,people,added_at,updated_at) VALUES('movie',?,?,?,0,0)",
                    params![key, key, p],
                )?;
            }
            Ok(())
        })
        .unwrap();
        store(&db, &[("nm0151419".into(), "Кайл Чендлер".into(), "wikidata", "Kyle Chandler".into())]).unwrap();
        let got = pending_directors(&db, 10).unwrap();
        assert_eq!(got, [("nm2467416".to_string(), "Damien Chazelle".to_string())]);
    }

    #[test]
    fn ru_director_uses_cache_then_translit() {
        let db = crate::db::Db::memory().unwrap();
        store(&db, &[("nm2467416".into(), "Дэмьен Шазелл".into(), "wikidata", "Damien Chazelle".into())]).unwrap();
        assert_eq!(ru_director(&db, "Damien Chazelle"), "Дэмьен Шазелл");
        assert_eq!(ru_director(&db, "Kyle Chandler"), "Кайл Чандлер");
        assert_eq!(ru_director(&db, "Тим Роббинс"), "Тим Роббинс");
    }

    #[test]
    fn translit_is_retried_after_a_day() {
        let now = 10 * RETRY_MS;
        let old = Cached { ru: "x".into(), source: "translit".into(), at: now - RETRY_MS - 1 };
        let fresh = Cached { ru: "x".into(), source: "translit".into(), at: now - 1000 };
        let wiki = Cached { ru: "x".into(), source: "wikidata".into(), at: 0 };
        assert!(due(&old, now));
        assert!(!due(&fresh, now));
        assert!(!due(&wiki, now));
    }

    #[test]
    fn translit_known_names() {
        for (from, to) in [
            ("Kyle Chandler", "Кайл Чандлер"),
            ("Claire Foy", "Клэр Фой"),
            ("Jason Clarke", "Джейсон Кларк"),
            ("Corey Stoll", "Кори Столл"),
            ("Neil Armstrong", "Нил Армстронг"),
            ("Ed Harris", "Эд Харрис"),
            ("Peter O'Toole", "Питер О'Тул"),
        ] {
            assert_eq!(translit(from), to, "{from}");
        }
    }

    #[test]
    fn translit_leaves_non_latin_alone() {
        for s in ["Тим Роббинс", "봉준호", "三船敏郎"] {
            assert_eq!(translit(s), s);
        }
    }

    #[test]
    fn translit_leaves_no_latin_letters() {
        for s in [
            "Jean-Claude Van Damme",
            "J.K. Simmons",
            "Joaquin Phoenix",
            "Zoe Saldana",
            "Wes Anderson",
            "Quentin Tarantino",
            "Xavier Dolan",
            "Lupita Nyong'o",
        ] {
            assert!(!has_latin(&translit(s)), "{s} -> {}", translit(s));
        }
    }
}
