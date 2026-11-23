//! Совместный просмотр: комната на фильм или серию с сервиса. У всех в комнате одно место в видео -
//! пауза, перемотка и следующая серия у всех сразу; чат и реакции поверх видео. Комнаты живут в
//! памяти (перезапуск сервера их закрывает), пустая комната закрывается через 10 минут.
//!
//! Связь - поток событий (SSE) от сервера и обычные запросы от зрителя: проходит через туннели,
//! где WebSocket может не пройти.

use crate::util::now_ms;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Пустая комната (никто не подключён) живёт столько.
const IDLE_MS: i64 = 10 * 60_000;
/// Сообщений чата в памяти комнаты - для тех, кто зашёл позже.
const CHAT_KEEP: usize = 60;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub revision: u64,
    pub file_id: i64,
    /// Видео по ссылке (фильм из Обзора: Rutube, VK, свой сайт) - тогда `file_id` = 0.
    pub link_id: Option<i64>,
    pub playing: bool,
    /// Место в секундах на момент `at` (мс Unix); при `playing` - идёт дальше.
    pub position: f64,
    pub at: i64,
    /// Кто последним управлял (имя).
    pub by: String,
}

pub struct Room {
    pub id: String,
    pub owner: i64,
    pub title_id: i64,
    pub state: State,
    /// Кто подключён: профиль → (карточка человека, сколько вкладок).
    pub members: HashMap<i64, (Value, usize)>,
    pub chat: Vec<Value>,
    /// Кто в голосовом чате (браузеры связаны напрямую - WebRTC; через нас - только сигналы).
    pub voice: std::collections::BTreeSet<i64>,
    pub empty_since: Option<i64>,
    pub tx: broadcast::Sender<String>,
    /// Кого создатель позвал (создатель - всегда): входить, писать и управлять могут только они.
    pub invited: std::collections::BTreeSet<i64>,
    /// Очередь просмотра: порядок задаёт создатель, остальные могут лишь предложить (`approved = false`).
    pub queue: Vec<Item>,
    pub current: Option<u64>,
    pub next_item: u64,
    /// Реакции зала на полосе времени текущего видео: место (с), смайлик, кто. Новое видео - с чистой полосы.
    pub marks: Vec<Value>,
}

/// Сколько отметок реакций держим на одно видео.
pub const MARKS_MAX: usize = 300;

/// Что стоит в очереди: фильм с сервиса (`file_id`) или видео по ссылке (`link_id`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: u64,
    pub file_id: i64,
    pub link_id: Option<i64>,
    pub title_id: i64,
    pub label: String,
    /// «Nova», «Rutube», «VK», «YouTube», «Файл», «Сайт».
    pub source: String,
    pub by: i64,
    pub by_name: String,
    pub approved: bool,
    /// Кто голосует за это видео («что дальше»): порядок всё равно задаёт создатель, голоса ему подсказка.
    pub votes: std::collections::BTreeSet<i64>,
}

/// В очереди не больше стольких видео, и не больше стольких чужих предложений от одного человека.
pub const QUEUE_MAX: usize = 50;
pub const SUGGEST_MAX: usize = 10;

impl Room {
    pub fn allows(&self, profile: i64) -> bool {
        self.owner == profile || self.invited.contains(&profile)
    }

    pub fn queue_event(&self) {
        self.send(json!({"type": "queue", "queue": self.queue, "current": self.current}));
    }

    pub fn members_json(&self) -> Vec<Value> {
        let mut m: Vec<&Value> = self.members.values().map(|(p, _)| p).collect();
        m.sort_by_key(|p| p["id"].as_i64());
        m.into_iter().cloned().collect()
    }

    /// Состояние на сейчас: при «смотрим» место ушло вперёд с момента последней команды (зашедший
    /// позже иначе начинал бы с места той команды - минуты назад).
    pub fn current(&self) -> State {
        let mut s = self.state.clone();
        if s.playing {
            let now = now_ms();
            s.position += (now - s.at).max(0) as f64 / 1000.0;
            s.at = now;
        }
        s
    }

    pub fn snapshot(&self) -> Value {
        json!({
            "type": "snapshot", "id": self.id, "ownerId": self.owner, "titleId": self.title_id,
            "state": self.current(), "members": self.members_json(), "chat": self.chat, "voice": self.voice,
            "queue": self.queue, "current": self.current, "marks": self.marks,
        })
    }

    /// Включить видео из очереди у всех с начала, на паузе (в пустой комнате - первое добавленное).
    fn start(&mut self, i: &Item) {
        self.marks.clear();
        self.title_id = i.title_id;
        self.current = Some(i.id);
        self.state = State {
            revision: self.state.revision.saturating_add(1),
            file_id: i.file_id,
            link_id: i.link_id,
            playing: false,
            position: 0.0,
            at: now_ms(),
            by: self.state.by.clone(),
        };
        self.send(json!({"type": "now", "titleId": self.title_id, "state": self.state, "current": self.current}));
    }

    fn send(&self, v: Value) {
        let _ = self.tx.send(v.to_string());
    }
}

#[derive(Default, Clone)]
pub struct Rooms(Arc<Mutex<HashMap<String, Room>>>);

impl Rooms {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &self,
        owner: i64,
        title_id: i64,
        file_id: i64,
        link_id: Option<i64>,
        position: f64,
        by: String,
        first: Option<Item>,
    ) -> String {
        let mut map = self.0.lock();
        let id = loop {
            let id = crate::util::human_code(8).to_lowercase();
            if !map.contains_key(&id) {
                break id;
            }
        };
        let (tx, _) = broadcast::channel(64);
        map.insert(
            id.clone(),
            Room {
                id: id.clone(),
                owner,
                title_id,
                state: State { revision: 0, file_id, link_id, playing: false, position, at: now_ms(), by },
                members: HashMap::new(),
                voice: Default::default(),
                chat: Vec::new(),
                empty_since: Some(now_ms()),
                tx,
                invited: [owner].into(),
                current: first.as_ref().map(|i| i.id),
                next_item: 2,
                queue: first.into_iter().collect(),
                marks: Vec::new(),
            },
        );
        id
    }

    /// Есть ли комната и можно ли этому человеку в неё: `None` - комнаты нет.
    pub fn allows(&self, id: &str, profile: i64) -> Option<bool> {
        self.with(id, |r| r.allows(profile))
    }

    /// Поставить в очередь. Создатель - сразу в очередь, остальные - предложением, которое создатель принимает.
    pub fn add_item(&self, id: &str, mut item: Item) -> Option<Result<Item, &'static str>> {
        self.with(id, |r| {
            let owner = item.by == r.owner;
            if r.queue.len() >= QUEUE_MAX {
                return Err("В очереди уже 50 видео");
            }
            if !owner && r.queue.iter().filter(|i| i.by == item.by && !i.approved).count() >= SUGGEST_MAX {
                return Err("Слишком много предложений: дождитесь, пока создатель их разберёт");
            }
            item.id = r.next_item;
            r.next_item += 1;
            item.approved = owner;
            r.queue.push(item.clone());
            if r.current.is_none() && owner {
                r.start(&item);
            }
            r.queue_event();
            Ok(item)
        })
    }

    /// Принять предложение (создатель).
    pub fn approve(&self, id: &str, item: u64) -> Option<bool> {
        self.with(id, |r| {
            let Some(i) = r.queue.iter_mut().find(|i| i.id == item) else { return false };
            i.approved = true;
            let first = i.clone();
            // Пустая комната: первое принятое видео сразу становится текущим.
            if r.current.is_none() {
                r.start(&first);
            }
            r.queue_event();
            true
        })
    }

    /// Убрать: создатель - любое, остальные - только своё предложение.
    pub fn remove_item(&self, id: &str, item: u64, by: i64) -> Option<bool> {
        self.with(id, |r| {
            let owner = by == r.owner;
            let Some(at) = r.queue.iter().position(|i| i.id == item && (owner || (i.by == by && !i.approved))) else {
                return false;
            };
            r.queue.remove(at);
            if r.current == Some(item) {
                r.current = None;
            }
            r.queue_event();
            true
        })
    }

    /// Новый порядок (создатель): те, кого нет в списке, остаются в конце как были.
    pub fn reorder(&self, id: &str, order: &[u64]) -> Option<()> {
        self.with(id, |r| {
            let mut rest = std::mem::take(&mut r.queue);
            let mut next = Vec::with_capacity(rest.len());
            for want in order {
                if let Some(at) = rest.iter().position(|i| i.id == *want) {
                    next.push(rest.remove(at));
                }
            }
            next.extend(rest);
            r.queue = next;
            r.queue_event();
        })
    }

    /// Включить видео из очереди у всех сразу (создатель). `None` - комнаты нет, `Some(false)` - такого видео нет.
    pub fn play_item(&self, id: &str, item: u64, by: String) -> Option<bool> {
        self.with(id, |r| {
            let Some(i) = r.queue.iter().find(|i| i.id == item && i.approved).cloned() else { return false };
            r.title_id = i.title_id;
            r.current = Some(i.id);
            r.state = State {
                revision: r.state.revision.saturating_add(1),
                file_id: i.file_id,
                link_id: i.link_id,
                playing: true,
                position: 0.0,
                at: now_ms(),
                by,
            };
            r.send(json!({"type": "now", "titleId": r.title_id, "state": r.state, "current": r.current}));
            r.queue_event();
            true
        })
    }

    /// Следующее принятое видео после текущего. `Some(None)` - очередь кончилась.
    pub fn next_item(&self, id: &str) -> Option<Option<u64>> {
        self.with(id, |r| {
            let at = r.current.and_then(|c| r.queue.iter().position(|i| i.id == c)).map_or(0, |p| p + 1);
            r.queue.iter().skip(at).find(|i| i.approved).map(|i| i.id)
        })
    }

    pub fn with<T>(&self, id: &str, f: impl FnOnce(&mut Room) -> T) -> Option<T> {
        self.0.lock().get_mut(id).map(f)
    }

    /// Вошёл (ещё одна вкладка): подписка на события и снимок комнаты.
    pub fn join(&self, id: &str, profile: i64, person: Value) -> Option<(broadcast::Receiver<String>, Value)> {
        self.with(id, |r| {
            let e = r.members.entry(profile).or_insert((person, 0));
            e.1 += 1;
            r.empty_since = None;
            let rx = r.tx.subscribe();
            let members = r.members_json();
            r.send(json!({"type": "members", "members": members}));
            (rx, r.snapshot())
        })
    }

    pub fn leave(&self, id: &str, profile: i64) {
        self.with(id, |r| {
            if let Some(e) = r.members.get_mut(&profile) {
                e.1 = e.1.saturating_sub(1);
                if e.1 == 0 {
                    r.members.remove(&profile);
                    if r.voice.remove(&profile) {
                        r.send(json!({"type": "voice", "ids": r.voice}));
                    }
                }
            }
            if r.members.is_empty() {
                r.empty_since = Some(now_ms());
                // Разрыв SSE не означает команду «пауза». Сохраняем часы комнаты,
                // чтобы при обрыве туннеля все вернулись к текущему месту. Пустую
                // комнату janitor удалит через IDLE_MS, как и раньше.
            }
            let members = r.members_json();
            r.send(json!({"type": "members", "members": members}));
        });
    }

    pub fn set_state(&self, id: &str, file_id: i64, playing: bool, position: f64, by: String) -> Option<State> {
        self.with(id, |r| {
            let link_id = r.state.link_id;
            r.state = State {
                revision: r.state.revision.saturating_add(1),
                file_id,
                link_id,
                playing,
                position: position.max(0.0),
                at: now_ms(),
                by,
            };
            r.send(json!({"type": "state", "state": r.state}));
            r.state.clone()
        })
    }

    pub fn say(&self, id: &str, from: Value, body: String) -> Option<()> {
        self.with(id, |r| {
            let m = json!({"type": "chat", "from": from, "body": body, "at": now_ms()});
            r.chat.push(m.clone());
            if r.chat.len() > CHAT_KEEP {
                r.chat.remove(0);
            }
            r.send(m);
        })
    }

    /// Вошёл в голосовой чат или вышел.
    pub fn voice(&self, id: &str, profile: i64, on: bool) -> Option<()> {
        self.with(id, |r| {
            let changed = if on { r.voice.insert(profile) } else { r.voice.remove(&profile) };
            if changed {
                r.send(json!({"type": "voice", "ids": r.voice}));
            }
        })
    }

    /// Сигнал голосового чата (предложение, ответ, адрес) - одному человеку в комнате.
    pub fn signal(&self, id: &str, from: i64, to: i64, data: Value) -> Option<()> {
        self.with(id, |r| {
            if r.voice.contains(&from) && r.members.contains_key(&to) {
                r.send(json!({"type": "signal", "from": from, "to": to, "data": data}));
            }
        })
    }

    /// Реакция: всплывает у всех с аватаром автора и остаётся отметкой на полосе времени (`position` - секунда видео).
    pub fn react(&self, id: &str, from: Value, emoji: &str, position: Option<f64>) -> Option<()> {
        self.with(id, |r| {
            let position = position.filter(|p| p.is_finite() && *p >= 0.0);
            if let Some(p) = position {
                if r.marks.len() >= MARKS_MAX {
                    r.marks.remove(0);
                }
                r.marks.push(json!({"position": p, "emoji": emoji, "from": from["id"]}));
            }
            r.send(json!({"type": "react", "from": from, "emoji": emoji, "position": position, "at": now_ms()}))
        })
    }

    /// Голос за видео в очереди: второй раз - снять. `None` - нет комнаты, `Some(false)` - нет такого видео.
    pub fn vote(&self, id: &str, item: u64, profile: i64) -> Option<bool> {
        self.with(id, |r| {
            let Some(i) = r.queue.iter_mut().find(|i| i.id == item) else { return false };
            if !i.votes.remove(&profile) {
                i.votes.insert(profile);
            }
            r.queue_event();
            true
        })
    }

    /// Где сейчас идущие комнаты: (кто в них, файл, ссылка, место) - чтобы записать «смотреть дальше».
    pub fn playing_places(&self) -> Vec<(Vec<i64>, i64, Option<i64>, f64)> {
        let map = self.0.lock();
        map.values()
            .filter(|r| r.state.playing && !r.members.is_empty())
            .map(|r| {
                let s = r.current();
                (r.members.keys().copied().collect(), s.file_id, s.link_id, s.position)
            })
            .collect()
    }

    /// Закрыть пустые дольше 10 минут.
    pub fn sweep(&self) {
        let now = now_ms();
        self.0.lock().retain(|_, r| r.empty_since.is_none_or(|t| now - t < IDLE_MS));
    }
}

/// Раз в минуту - убрать брошенные комнаты и записать место тем, кто смотрит (на случай, когда плеер
/// зрителя молчит: вкладка в фоне, связь с хостом оборвалась).
pub async fn janitor(app: Arc<crate::App>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        app.rooms.sweep();
        for (who, file, link, position) in app.rooms.playing_places() {
            crate::api::rooms::save_place(&app, &who, file, link, position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_state_leave() {
        let rooms = Rooms::default();
        let id = rooms.create(1, 7, 70, None, 12.0, "Хост".into(), None);
        let (mut rx, snap) = rooms.join(&id, 1, json!({"id": 1, "name": "Хост"})).unwrap();
        assert_eq!(snap["state"]["fileId"], 70);
        assert!(rx.try_recv().unwrap().contains("members"));
        rooms.set_state(&id, 70, true, 30.0, "Хост".into()).unwrap();
        assert!(rx.try_recv().unwrap().contains("\"playing\":true"));
        // Снимок для зашедшего позже - место на сейчас, а не на момент команды.
        rooms.with(&id, |r| r.state.at -= 5_000).unwrap();
        let (_, snap) = rooms.join(&id, 2, json!({"id": 2})).unwrap();
        assert!(snap["state"]["position"].as_f64().unwrap() >= 35.0);
        rooms.leave(&id, 2);
        rooms.leave(&id, 1);
        // Все SSE-соединения оборвались: это не команда паузы. Переподключение
        // получает прежнюю ревизию и актуальное место продолжающегося видео.
        let revision = rooms.with(&id, |r| r.state.revision).unwrap();
        rooms.with(&id, |r| r.state.at -= 3_000).unwrap();
        let (_, reconnected) = rooms.join(&id, 1, json!({"id": 1})).unwrap();
        assert_eq!(reconnected["state"]["playing"], true);
        assert_eq!(reconnected["state"]["revision"], revision);
        assert!(reconnected["state"]["position"].as_f64().unwrap() >= 38.0);
        rooms.leave(&id, 1);
        rooms.sweep();
        assert!(rooms.with(&id, |_| ()).is_some());
        rooms.with(&id, |r| r.empty_since = Some(now_ms() - IDLE_MS - 1)).unwrap();
        rooms.sweep();
        assert!(rooms.with(&id, |_| ()).is_none());
        assert!(rooms.join("нет-такой", 1, json!({})).is_none());
    }

    #[test]
    fn voice_and_signals() {
        let rooms = Rooms::default();
        let id = rooms.create(1, 7, 70, None, 0.0, "А".into(), None);
        let (mut rx, _) = rooms.join(&id, 1, json!({"id": 1})).unwrap();
        rooms.join(&id, 2, json!({"id": 2})).unwrap();
        while rx.try_recv().is_ok() {}
        // Сигнал - только из голосового чата.
        rooms.signal(&id, 1, 2, json!({"type": "offer"})).unwrap();
        assert!(rx.try_recv().is_err());
        rooms.voice(&id, 1, true).unwrap();
        assert!(rx.try_recv().unwrap().contains("\"voice\""));
        rooms.signal(&id, 1, 2, json!({"type": "offer"})).unwrap();
        assert!(rx.try_recv().unwrap().contains("\"signal\""));
        // Ушёл из комнаты - и из голосового чата.
        rooms.leave(&id, 1);
        assert!(rooms.with(&id, |r| r.voice.is_empty()).unwrap());
    }
}
