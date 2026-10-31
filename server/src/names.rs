//! Русские имена людей и персонажей: IMDb отдаёт латиницу. Перевод накладывается при отдаче
//! страницы (в базе остаются оригинальные имена): кэш `person_names` -> Викиданные -> транслитерация.

use crate::db::Db;
use crate::util::now_ms;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

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

/// Записать (id, русское имя, источник).
pub fn store(db: &Db, rows: &[(String, String, &str)]) -> anyhow::Result<()> {
    let at = now_ms();
    db.tx(|tx| {
        for (id, ru, source) in rows {
            tx.execute(
                "INSERT INTO person_names (id, ru, source, at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET ru = ?2, source = ?3, at = ?4",
                params![id, ru, source, at],
            )?;
        }
        Ok(())
    })?;
    Ok(())
}

pub fn has_latin(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_alphabetic())
}

/// Английские имена, которые правила пишут неверно (ключ - в нижнем регистре).
const FIRST: &[(&str, &str)] = &[
    ("kyle", "Кайл"), ("claire", "Клэр"), ("jason", "Джейсон"), ("corey", "Кори"), ("neil", "Нил"),
    ("ed", "Эд"), ("peter", "Питер"), ("john", "Джон"), ("james", "Джеймс"), ("michael", "Майкл"),
    ("david", "Дэвид"), ("robert", "Роберт"), ("william", "Уильям"), ("richard", "Ричард"),
    ("thomas", "Томас"), ("charles", "Чарльз"), ("daniel", "Дэниел"), ("matthew", "Мэттью"),
    ("anthony", "Энтони"), ("mark", "Марк"), ("paul", "Пол"), ("steven", "Стивен"), ("stephen", "Стивен"),
    ("andrew", "Эндрю"), ("joshua", "Джошуа"), ("kevin", "Кевин"), ("brian", "Брайан"),
    ("george", "Джордж"), ("edward", "Эдвард"), ("ryan", "Райан"), ("jacob", "Джейкоб"),
    ("gary", "Гэри"), ("nicholas", "Николас"), ("eric", "Эрик"), ("jonathan", "Джонатан"),
    ("larry", "Ларри"), ("justin", "Джастин"), ("scott", "Скотт"), ("brandon", "Брэндон"),
    ("benjamin", "Бенджамин"), ("samuel", "Сэмюэл"), ("patrick", "Патрик"), ("jack", "Джек"),
    ("henry", "Генри"), ("tom", "Том"), ("tim", "Тим"), ("sam", "Сэм"), ("ben", "Бен"), ("joe", "Джо"),
    ("jim", "Джим"), ("bill", "Билл"), ("bob", "Боб"), ("chris", "Крис"), ("christopher", "Кристофер"),
    ("alex", "Алекс"), ("adam", "Адам"), ("jeff", "Джефф"), ("jeremy", "Джереми"), ("jesse", "Джесси"),
    ("mary", "Мэри"), ("jennifer", "Дженнифер"), ("emma", "Эмма"), ("emily", "Эмили"),
    ("olivia", "Оливия"), ("sophia", "София"), ("anna", "Анна"), ("amy", "Эми"), ("kate", "Кейт"),
    ("katie", "Кэти"), ("julia", "Джулия"), ("julie", "Джули"), ("jessica", "Джессика"),
    ("sarah", "Сара"), ("laura", "Лора"), ("rachel", "Рэйчел"), ("rebecca", "Ребекка"),
    ("elizabeth", "Элизабет"), ("margaret", "Маргарет"), ("natalie", "Натали"), ("scarlett", "Скарлетт"),
    ("jean", "Жан"), ("claude", "Клод"), ("zoe", "Зои"), ("mia", "Миа"), ("diane", "Дайан"),
    ("cate", "Кейт"), ("keira", "Кира"), ("joaquin", "Хоакин"), ("quentin", "Квентин"),
    ("xavier", "Ксавье"), ("wes", "Уэс"), ("woody", "Вуди"), ("tony", "Тони"), ("buzz", "Базз"),
    ("deke", "Дик"), ("frank", "Фрэнк"), ("harry", "Гарри"), ("leo", "Лео"), ("liam", "Лиам"),
    ("max", "Макс"), ("nick", "Ник"), ("oliver", "Оливер"), ("ray", "Рэй"), ("roy", "Рой"),
];

fn first_name(w: &str) -> Option<&'static str> {
    let lower = w.to_ascii_lowercase();
    FIRST.iter().find(|(k, _)| *k == lower).map(|(_, v)| *v)
}

fn single(c: char) -> &'static str {
    match c {
        'a' => "а", 'b' => "б", 'c' => "к", 'd' => "д", 'e' => "е", 'f' => "ф", 'g' => "г",
        'h' => "х", 'i' => "и", 'j' => "дж", 'k' => "к", 'l' => "л", 'm' => "м", 'n' => "н",
        'o' => "о", 'p' => "п", 'q' => "к", 'r' => "р", 's' => "с", 't' => "т", 'u' => "у",
        'v' => "в", 'w' => "в", 'x' => "кс", 'y' => "и", 'z' => "з",
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
            ('a', 'y', _) => ("эй", 2),
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
        store(&db, &[("nm1".into(), "Райан Гослинг".into(), "wikidata")]).unwrap();
        store(&db, &[("nm1".into(), "Райан Гослинг".into(), "translit")]).unwrap();
        let got = cached(&db, &["nm1".into(), "nm2".into()]);
        assert_eq!(got.len(), 1);
        assert_eq!(got["nm1"].source, "translit");
        assert_eq!(got["nm1"].ru, "Райан Гослинг");
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
