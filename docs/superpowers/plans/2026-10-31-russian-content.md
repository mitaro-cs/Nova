# Русские имена людей и персонажей - план работ

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** на страницах фильма (`/title/[id]`, `/ext/[id]`) имена людей и персонажей показываются по-русски, оригинал сохраняется отдельным полем.

**Architecture:** новый модуль `server/src/names.rs` переводит имена в момент отдачи страницы (а не при записи): русское имя берётся из кэша `person_names`, при промахе - из Викиданных (один SPARQL-запрос на всех людей фильма, таймаут 3 с), иначе транслитерация. В базе (`titles.people`, `ext_titles.extra`) остаются оригинальные имена, поэтому ключи по имени (`liked_people`, `ext_people_names`, поиск режиссёров) не ломаются, а уже сохранённые фильмы получают русские имена без перезагрузки описаний.

**Tech Stack:** Rust (axum, rusqlite, reqwest, serde_json), SvelteKit 5 + TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-10-31-russian-content-design.md`. Отличия от спецификации (решены при составлении плана): перевод накладывается при чтении, а не при записи - поэтому фоновая задача не нужна, просроченную транслитерацию (старше 24 ч) перепроверяет следующее чтение; должности уже переведены на фронте (`team` в `ext/[id]/+page.svelte`, `title/[id]/+page.svelte`); перевод персонажей через TMDB в эту часть не входит (имя актёра в TMDB не сопоставить с id IMDb без лишних запросов) - персонажи транслитерируются.

## Global Constraints

- Все тексты интерфейса и сообщения об ошибках - по-русски, в тексте обычный дефис « - », без длинных тире.
- Внутренние имена (`fsfy`, `app.fsfy`, `X-Fsfy`, cookie `fsfy_*`) не менять.
- Миграции - только новый элемент `db::MIGRATIONS`, старые не править. Следующий номер - 25. `OLDEST` не менять (миграция только добавляет таблицу).
- Время - миллисекунды Unix (`crate::util::now_ms`).
- Ответ Викиданных не ждём дольше 3 секунд; сеть недоступна - страница всё равно отвечает.
- Данные в `titles.people` и `ext_titles.extra` не переписываем: имена в базе остаются оригинальными.
- Коммиты - Conventional Commits; коммитить только когда владелец разрешил (правило проекта: GitHub и установка - по слову владельца). Шаги «Commit» ниже - локальный коммит в ветку `alpha-0.1`, push не делать.

## Review Focus

- Человек без `id` (старые записи): Викиданные не трогаем, транслитерация, `orig` заполнен.
- Имя уже кириллицей («Тим Роббинс») и не-латиница («봉준호»): не меняется, `orig` пуст.
- Дефис, апостроф, инициалы («Jean-Claude Van Damme», «Peter O'Toole», «J.K. Simmons»): без паники, в результате нет латинских букв.
- Викиданные недоступны или отвечают дольше 3 с: ответ страницы приходит, имя - транслитерация, в кэше `source = 'translit'`, через 24 ч попытка повторяется.
- Идентификатор не вида `nm` + цифры никогда не попадает в SPARQL-запрос (защита от подстановки).

---

### Task 1: Транслитерация

**Files:**
- Create: `server/src/names.rs`
- Modify: `server/src/lib.rs` (добавить `pub mod names;` после `pub mod monitor;`)
- Test: юнит-тесты в `server/src/names.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces: `pub fn has_latin(s: &str) -> bool` (есть хотя бы одна буква `A-Za-z`), `pub fn translit(name: &str) -> String` (латиницу заменяет русской записью, остальные символы - кириллицу, не-латиницу, пробелы, цифры, знаки - оставляет как есть).

- [ ] **Step 1: Write the failing tests** в `names.rs`:

```rust
#[test] fn translit_known_names() {
    for (from, to) in [
        ("Kyle Chandler", "Кайл Чандлер"), ("Claire Foy", "Клэр Фой"),
        ("Jason Clarke", "Джейсон Кларк"), ("Corey Stoll", "Кори Столл"),
        ("Neil Armstrong", "Нил Армстронг"), ("Ed Harris", "Эд Харрис"),
        ("Peter O'Toole", "Питер О'Тул"),
    ] { assert_eq!(translit(from), to, "{from}"); }
}
#[test] fn translit_leaves_non_latin_alone() {
    for s in ["Тим Роббинс", "봉준호", "三船敏郎"] { assert_eq!(translit(s), s); }
}
#[test] fn translit_leaves_no_latin_letters() {
    for s in ["Jean-Claude Van Damme", "J.K. Simmons", "Joaquin Phoenix", "Zoe Saldana",
              "Wes Anderson", "Quentin Tarantino", "Xavier Dolan", "Lupita Nyong'o"] {
        assert!(!has_latin(&translit(s)), "{s} -> {}", translit(s));
    }
}
```

- [ ] **Step 2: Run** `cargo test -p fsfy-server names::` - Expected: FAIL (модуля нет / функции не определены).

- [ ] **Step 3: Implement `translit` and `has_latin` in `server/src/names.rs`.** Две ступени: (1) словарь первых слов - английские имена, у которых правило ошибается (Kyle -> Кайл, Claire -> Клэр, Jason -> Джейсон, Corey -> Кори, Neil -> Нил, Ed -> Эд, Peter -> Питер, плюс ещё порядка 80 самых частых: John, James, Michael, David, Robert, William, Mary, Jennifer и т.п.), поиск без учёта регистра; (2) побуквенные правила с диграфами (ch, sh, th, ph, ck, kh, zh, oo, ee, ll, qu, dg, ew) и учётом конца слова (`ke` -> `к`, `er` -> `ер`), заглавная буква исходного слова сохраняется. Любая латинская буква, для которой нет правила, обязана иметь запасное соответствие - в результате латиницы не остаётся.

- [ ] **Step 4: Run** `cargo test -p fsfy-server names::` - Expected: PASS (3 теста).

- [ ] **Step 5: Commit**

```bash
git add server/src/names.rs server/src/lib.rs
git commit -m "feat: транслитерация имён людей (names.rs)"
```

### Task 2: Таблица `person_names` и кэш

**Files:**
- Modify: `server/src/db.rs` (новый последний элемент `MIGRATIONS`, номер 25, комментарий в стиле соседних)
- Modify: `server/src/names.rs`
- Test: юнит-тест в `names.rs` на временной базе (как в других модулях `db::Db::open_in_memory` или аналог - найти в `db.rs`)

**Interfaces:**
- Consumes: `crate::util::now_ms`, `crate::db` (`app.db.with`, `app.db.tx`).
- Produces: `pub const RETRY_MS: i64 = 86_400_000;` `pub struct Cached { pub ru: String, pub source: String, pub at: i64 }` `pub fn cached(app: &crate::App, ids: &[String]) -> HashMap<String, Cached>` `pub fn store(app: &crate::App, rows: &[(String, String, &str)]) -> anyhow::Result<()>` (id, ru, source: `"wikidata"` | `"translit"`).

- [ ] **Step 1: Write the failing test** `person_names_roundtrip`: `store` пишет `("nm1","Райан Гослинг","wikidata")`, повторный `store` с тем же id и `"translit"` обновляет строку, `cached(&["nm1","nm2"])` возвращает только `nm1` с `source == "translit"`.

- [ ] **Step 2: Run** `cargo test -p fsfy-server person_names_roundtrip` - Expected: FAIL.

- [ ] **Step 3: Add migration 25** в `db.rs`: `CREATE TABLE person_names (id TEXT PRIMARY KEY, ru TEXT NOT NULL, source TEXT NOT NULL, at INTEGER NOT NULL);` и реализовать `cached`/`store` (upsert `ON CONFLICT(id) DO UPDATE`).

- [ ] **Step 4: Run** `cargo test -p fsfy-server person_names_roundtrip` и `cargo test -p fsfy-server db::` - Expected: PASS (тесты миграций не сломаны).

- [ ] **Step 5: Commit**

```bash
git add server/src/db.rs server/src/names.rs
git commit -m "feat: кэш русских имён (миграция 25, person_names)"
```

### Task 3: Клиент Викиданных

**Files:**
- Modify: `server/src/names.rs`
- Test: юнит-тесты в `names.rs`

**Interfaces:**
- Consumes: образец HTTP-вызова - `server/src/franchise.rs:85-98` (`user-agent`, `accept: application/sparql-results+json`).
- Produces: `pub fn valid_id(id: &str) -> bool` (`nm` + только цифры, 3-12 символов), `pub fn sparql(ids: &[String]) -> String` (`VALUES ?imdb { "nm…" … }`, `?item wdt:P345 ?imdb`, `rdfs:label ?ru` с `FILTER(LANG(?ru)="ru")`; в запрос попадают только id, прошедшие `valid_id`), `pub fn parse_wikidata(v: &serde_json::Value) -> HashMap<String, String>` (id -> русская метка), `pub async fn wikidata_ru(http: &reqwest::Client, endpoint: &str, ids: &[String]) -> anyhow::Result<HashMap<String, String>>` (таймаут запроса 3 с; пустой `endpoint` - сразу `Ok` с пустой картой).

- [ ] **Step 1: Write the failing tests:**

```rust
#[test] fn only_nm_ids_reach_sparql() {
    let q = sparql(&["nm0331516".into(), "nm1\" } DROP".into(), "tt0111161".into()]);
    assert!(q.contains("\"nm0331516\"")); assert!(!q.contains("DROP")); assert!(!q.contains("tt0111161"));
}
#[test] fn wikidata_answer_is_parsed() {
    let v = serde_json::json!({"results":{"bindings":[
        {"imdb":{"value":"nm0331516"},"ru":{"value":"Райан Гослинг"}},
        {"imdb":{"value":"nm0001"},"ru":{"value":"Ryan X"}}]}});
    let m = parse_wikidata(&v);
    assert_eq!(m["nm0331516"], "Райан Гослинг");
    assert!(!m.contains_key("nm0001")); // метка без кириллицы - не перевод
}
```

- [ ] **Step 2: Run** `cargo test -p fsfy-server names::` - Expected: FAIL.

- [ ] **Step 3: Implement** функции из Interfaces; `parse_wikidata` отбрасывает метки без кириллицы (`crate::ruwiki::has_cyrillic`).

- [ ] **Step 4: Run** `cargo test -p fsfy-server names::` - Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add server/src/names.rs
git commit -m "feat: русские имена из Викиданных одним запросом"
```

### Task 4: Перевод людей фильма (`localize`) и поле `orig`

**Files:**
- Modify: `server/src/meta/mod.rs` (`Person` - поле `orig`; во всех местах, где `Person { … }` создаётся перечислением полей - `meta/imdb.rs` (`parse_details`, тесты), `meta/tmdb.rs`, `api/catalog.rs`, `api/admin.rs` - добавить `orig: None`)
- Modify: `server/src/names.rs`
- Test: юнит-тесты в `names.rs`

**Interfaces:**
- Consumes: `names::{translit, has_latin, cached, store, RETRY_MS, wikidata_ru, valid_id}`, `app.db.setting_or(key, default)`.
- Produces: `meta::Person.orig: Option<String>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`) и `pub async fn localize(app: &crate::App, people: &mut [crate::meta::Person])`. Настройка `names.wikidata.sparql` (по умолчанию `https://query.wikidata.org/sparql`, пустая строка - Викиданные выключены).

- [ ] **Step 1: Write the failing tests** (`#[tokio::test]`, приложение из тестовых помощников соседних модулей или `Db` + `reqwest::Client` по образцу):
  - `localize_uses_cache_then_translit`: в `person_names` лежит `nm0331516 -> Райан Гослинг (wikidata)`, настройка `names.wikidata.sparql = ""`; людей: Gosling с этим id и ролью «Neil Armstrong», `Kyle Chandler` с id `nm0151419` (нет в кэше), `Tim X` без id. Ожидается: `Райан Гослинг` (`orig = Some("Ryan Gosling")`), роль `Нил Армстронг`; `Кайл Чандлер` и в `person_names` строка `nm0151419` с `source = 'translit'`; безымянный по id - транслитерация, в кэш не пишется.
  - `localize_skips_cyrillic_and_other_scripts`: `Тим Роббинс` и `봉준호` остаются, `orig == None`.
  - `translit_is_retried_after_a_day`: строка `translit` с `at = now - RETRY_MS - 1` считается просроченной и уходит в Викиданные (проверка через `names::due(&Cached, now) -> bool`: `translit` старше `RETRY_MS` - `true`, `wikidata` - `false`).

- [ ] **Step 2: Run** `cargo test -p fsfy-server localize` - Expected: FAIL.

- [ ] **Step 3: Implement `localize`:** набрать id людей с латинским именем; из `cached` взять годные (`wikidata` или свежий `translit`); остальным - `tokio::time::timeout(3 с, wikidata_ru(...))`; найденное - `store(..., "wikidata")`; не найденное и при сбое - `translit(name)` и `store(..., "translit")`; людям без id - только `translit`, без кэша. Для каждого изменённого: `orig = Some(старое имя)`, `name = русское`. Роль (`role`) с латиницей - `translit(role)` без кэша.

- [ ] **Step 4: Run** `cargo test -p fsfy-server localize` и `cargo test -p fsfy-server --lib` - Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add server/src/names.rs server/src/meta server/src/api
git commit -m "feat: names::localize - русские имена людей и персонажей"
```

### Task 5: Подключение к страницам фильма

**Files:**
- Modify: `server/src/api/catalog.rs:447` (после `serde_json::from_str(&people)` вызвать `crate::names::localize(&app, &mut people).await;` до цикла с `p.photo`)
- Modify: `server/src/api/discover.rs:326-331` (`x["people"]` разобрать в `Vec<crate::meta::Person>`, вызвать `localize`, собрать JSON с полями `name`, `orig`, `job`, `role`, `photo` (через `img`), `id`)
- Test: `server/tests/discover.rs` (новый тест рядом с `feed_marks_wishes_groups_links`, по образцу строк 419-426)

**Interfaces:**
- Consumes: `names::localize`; ответ `GET /api/discover/title/{id}` - поле `people[]` теперь содержит `orig`.
- Produces: `GET /api/titles/{id}` (`people[]`) и `GET /api/discover/title/{id}` (`people[]`) отдают `name` по-русски и `orig` (или без поля, если перевода не было).

- [ ] **Step 1: Write the failing test** `ext_page_people_are_russian` в `server/tests/discover.rs`: установить настройку `names.wikidata.sparql` = `""`; записать в `ext_titles.extra` (как в строке 419) людей `[{"name":"Ryan Gosling","job":"actor","role":"Neil Armstrong","id":"nm0331516"},{"name":"Kyle Chandler","job":"actor","role":"Deke Slayton","id":"nm0151419"}]` и `detailed = fsfy::discover::DETAILED`; заранее положить в `person_names` строку `nm0331516 / Райан Гослинг / wikidata`; запросить `/api/discover/title/<id>`; ожидать у первого `name = "Райан Гослинг"`, `orig = "Ryan Gosling"`, `role = "Нил Армстронг"`; у второго `name = "Кайл Чандлер"`. Аналогично для `/api/titles/{id}`: тест `own_title_people_are_russian` с записью в `titles.people`.

- [ ] **Step 2: Run** `cargo test -p fsfy-server --test discover ext_page_people_are_russian own_title_people_are_russian` - Expected: FAIL (имена латиницей).

- [ ] **Step 3: Wire `localize` in both handlers** как описано в Files; остальные поля ответа не менять.

- [ ] **Step 4: Run** `cargo test -p fsfy-server --test discover` и `cargo clippy -p fsfy-server --all-targets -- -D warnings` - Expected: PASS, без предупреждений.

- [ ] **Step 5: Commit**

```bash
git add server/src/api server/tests/discover.rs
git commit -m "feat: имена людей на страницах фильма - по-русски"
```

### Task 6: Фронт - оригинал в ключах, русское в тексте, проверки

**Files:**
- Modify: `web/src/lib/people.ts` (`personHref`)
- Modify: `web/src/lib/types.ts` (тип `people` у карточки ленты на строке ~371 и `Person` фильма - добавить `orig?: string | null; id?: string | null`)
- Modify: `web/src/routes/title/[id]/+page.svelte:511`, `web/src/routes/ext/[id]/+page.svelte:716` (подсказка)
- Modify: `web/src/routes/person/+page.svelte:131` (заголовок)
- Create: `web/src/lib/people.test.ts`
- Modify: `CHANGELOG.md` (новый раздел `0.7.2-alpha (не выпущено)` над `0.7.1-alpha`, строка «Имена людей и персонажей на страницах фильма - по-русски»); версии в файлах не поднимать.

**Interfaces:**
- Consumes: поле `orig` из ответов Task 5.
- Produces: `personHref(p: { name: string; orig?: string | null; job?: string; id?: unknown }): string` - параметр `name` в ссылке = `p.orig ?? p.name` (ключ остаётся оригинальным, как у `liked_people` и `ext_people_names`), дополнительный параметр `ru` = `p.name`, когда `orig` задан.

- [ ] **Step 1: Write the failing test** `web/src/lib/people.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { personHref } from './people';
describe('personHref', () => {
  it('ключ - оригинальное имя, русское - отдельным параметром', () => {
    const u = new URL(personHref({ name: 'Райан Гослинг', orig: 'Ryan Gosling', job: 'actor', id: 'nm0331516' }), 'http://x');
    expect(u.searchParams.get('name')).toBe('Ryan Gosling');
    expect(u.searchParams.get('ru')).toBe('Райан Гослинг');
    expect(u.searchParams.get('id')).toBe('nm0331516');
  });
  it('без orig ссылка как раньше', () => {
    const u = new URL(personHref({ name: 'Тим Роббинс', job: 'actor' }), 'http://x');
    expect(u.searchParams.get('name')).toBe('Тим Роббинс');
    expect(u.searchParams.has('ru')).toBe(false);
  });
});
```

- [ ] **Step 2: Run** `npm test -- people` в `web/` - Expected: FAIL.

- [ ] **Step 3: Implement** `personHref` по Interfaces; заголовок на странице человека - `d?.info?.nameRu ?? page.url.searchParams.get('ru') ?? name`; подсказка на карточке - `{p.orig ? `${p.name} (${p.orig})` : p.name} - фильмография`.

- [ ] **Step 4: Run** в `web/`: `npm run check`, `npm test`, `npm run build`; затем `cargo build --release -p fsfy-server` (интерфейс вшит в бинарник) и `npx playwright test e2e/layout.spec.ts --project=desktop --project=phone` с `FSFY_BIN=C:\Maison\Nova\target\release\fsfy.exe` - Expected: всё PASS.

- [ ] **Step 5: Ручная проверка на живых данных:** запустить Nova, открыть «Первый человек» (`/ext/tt1213641`) - в блоке «Актёры и съёмочная группа» нет латиницы; клик по актёру открывает страницу с русским заголовком; «нравится» работает.

- [ ] **Step 6: Commit**

```bash
git add web/src CHANGELOG.md
git commit -m "feat: ссылки на людей по оригиналу, русское имя в тексте"
```

---

## Проверка перед установкой (общая)

`cargo fmt --all --check`, `cargo clippy -p fsfy-server --all-targets -- -D warnings`, `cargo test -p fsfy-server`, в `web/` - `npm run check`, `npm test`, `npm run build`, затем `cargo build --release -p fsfy-server` и `npx playwright test --project=desktop --project=phone --project=tablet`. Установка на этот компьютер и GitHub - только по слову владельца.
