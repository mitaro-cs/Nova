# Русский контент, остаток: язык, слоган, названия, режиссёры на карточках - план работ

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** убрать оставшуюся латиницу, найденную аудитом 2026-11-01: язык фильма, слоган, «Также», названия без русского релиза, режиссёр на карточках, подпись фото человека.

**Architecture:** фронт использует готовые помощники (`languagesRu`, `localized`) и один новый для «Также». Русские названия фильмов ленты - уже существующий механизм `catalog_search::localize` (Викиданные -> таблица `title_localizations`, читают `ext_cards` и поиск видео): он сейчас вызывается только из поиска, поэтому названия подборок остаются английскими. Добавляем фонового воркера, который прогоняет через него названия без кириллицы пачками. Режиссёры на карточках - из кэша `person_names` без сетевых запросов при выдаче.

**Tech Stack:** Rust (axum, rusqlite, reqwest, tokio), SvelteKit 5 + TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-10-31-russian-content-design.md`, раздел «Дополнение: аудит латиницы». Имена людей и персонажей на страницах фильма уже реализованы: `server/src/names.rs`.

## Global Constraints

- Всё по-русски, обычный дефис « - », без длинных тире.
- Внутренние имена (`fsfy`, `app.fsfy`, `X-Fsfy`) не менять.
- Миграции - только новый элемент `db::MIGRATIONS` (следующий номер - 26), старые не править; миграция только добавляет столбцы, `OLDEST` не менять.
- Викиданные: пачка до 65 id (так в `catalog_search::localize`), пауза между пачками 2 с, пустая настройка `names.wikidata.sparql` выключает запросы воркера; сбой сети не ломает страницу.
- Названия в `ext_titles.title` не переписываем: русское живёт в `title_localizations` (так уже читают `ext_cards`, поиск видео, превью).
- Проверки: `cargo fmt --all --check`, `cargo clippy -p fsfy-server --all-targets -- -D warnings`, `cargo test -p fsfy-server`, в `web/`: `npm run check`, `npm test`, `npm run build`, затем `cargo build --release -p fsfy-server` и `npx playwright test --project=desktop --project=phone --project=tablet` с `FSFY_BIN=C:\Maison\Nova\target\release\fsfy.exe`.
- Коммиты - Conventional Commits, автор `Claude` через `-c user.name=Claude -c user.email=noreply@anthropic.com`, только локально; push, сборка и установка - по слову владельца.

## Review Focus

- Фильм без русской метки на Викиданных: название остаётся оригинальным, воркер не опрашивает его снова раньше чем через 30 дней (`ru_at`).
- Название уже на кириллице (в том числе смешанное «Дьявол носит Prada 2») воркер не берёт в работу.
- Слоган на кириллице показывается, английский скрыт; хост с настройкой «язык описаний - оригинал» (`metadataLanguage === 'source'`) видит всё как есть.
- «Также»: оригинальное название и русские остаются, испанские, немецкие, турецкие убраны; если после фильтра пусто - строка не выводится.
- Режиссёр, которого нет в кэше имён: показывается транслитерация без сетевого запроса, карточки не замедляются.

---

### Task 1: Язык, слоган и «Также» на странице фильма

**Files:**
- Create: `web/src/lib/aka.ts`, `web/src/lib/aka.test.ts`, `web/src/lib/countries.test.ts`
- Modify: `web/src/lib/countries.ts` (дополнить `LANGUAGES`), `web/src/routes/title/[id]/+page.svelte:605,608`, `web/src/routes/ext/[id]/+page.svelte:788,791`, `web/src/lib/title/RewindHero.svelte:84`

**Interfaces:**
- Produces: `akaShown(aka: string[], original?: string | null): string[]` - оставляет `original` (если задан) и элементы с кириллицей, без повторов, не больше 4; `languageRu` знает не меньше 55 языков, неизвестный остаётся как есть.

- [ ] **Step 1: Write the failing tests:**
  `aka.test.ts`: `akaShown(['First Man','El primer hombre en la luna','Aufbruch zum Mond','Первый человек','Человек на Луне'], 'First Man')` равно `['First Man','Первый человек','Человек на Луне']`; `akaShown([], null)` равно `[]`; не больше 4 элементов; без повторов.
  `countries.test.ts`: `languagesRu(['English','Mandarin','Russian'])` равно `['английский','китайский','русский']`; `languageRu('Klingon')` равно `'Klingon'`; для каждого из `['Indonesian','Vietnamese','Bengali','Tamil','Romanian','Bulgarian','Serbian','Croatian','Icelandic','Estonian']` результат содержит кириллицу.
- [ ] **Step 2: Run** `npm test -- aka countries` в `web/` - Expected: FAIL.
- [ ] **Step 3: Implement** `akaShown` (кириллица - `/[а-яё]/i`) и дополнить `LANGUAGES` до ≥ 55 записей строчными названиями; на обеих страницах «Язык» - `languagesRu(...).slice(0, 3).join(', ')`, «Также» - `akaShown(aka, original).join(', ')` и строка скрыта при пустом списке; в `RewindHero.svelte` слоган выводить через `localized(tagline)` (как в `SeriesHero.svelte:151`): `{#if localized(tagline)}`.
- [ ] **Step 4: Run** `npm test`, `npm run check` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add web/src
git commit -m "fix: язык, слоган и «Также» на странице фильма - только по-русски"
```

### Task 2: Миграция 26

**Files:**
- Modify: `server/src/db.rs` (новый последний элемент `MIGRATIONS`)
- Test: юнит-тест в `server/src/db.rs` (рядом с `migrates_and_keeps_settings`)

**Interfaces:**
- Produces: `ext_titles.ru_at INTEGER` (NULL - русское название ещё не искали; время попытки в мс); `person_names.orig TEXT` (оригинальное написание имени) и индекс `person_names_orig` по нему.

- [ ] **Step 1: Write the failing test** `migration_26_adds_columns`: на `Db::memory()` запросы `SELECT ru_at FROM ext_titles LIMIT 0` и `SELECT orig FROM person_names LIMIT 0` выполняются без ошибки.
- [ ] **Step 2: Run** `cargo test -p fsfy-server --lib migration_26` - Expected: FAIL (нет столбцов).
- [ ] **Step 3: Implement** миграцию: `ALTER TABLE ext_titles ADD COLUMN ru_at INTEGER; ALTER TABLE person_names ADD COLUMN orig TEXT; CREATE INDEX person_names_orig ON person_names(orig);`
- [ ] **Step 4: Run** `cargo test -p fsfy-server --lib` - Expected: PASS (включая тесты миграций и `missing_schema`).
- [ ] **Step 5: Commit**

```bash
git add server/src/db.rs
git commit -m "feat: миграция 26 - ext_titles.ru_at и person_names.orig"
```

### Task 3: Воркер русских названий ленты

**Files:**
- Modify: `server/src/discover.rs` (воркер `titles_ru` и помощники рядом с `posters`, строки ~430-460), `server/src/lib.rs` (поле `titles_kick: Notify` рядом со строкой 72, инициализация рядом со строкой 223, `tokio::spawn(discover::titles_ru(app.clone()))` рядом со строкой 256), `server/src/discover.rs` (`app.titles_kick.notify_one()` рядом с каждым `posters_kick.notify_one()`)
- Test: юнит-тесты в `server/src/discover.rs`

**Interfaces:**
- Consumes: `catalog_search::localize(app: &App, ids: &[String]) -> anyhow::Result<()>` (пишет `title_localizations` для найденных, до 65 id за вызов); настройка `names.wikidata.sparql` (пустая - воркер ничего не делает).
- Produces: `pub fn pending_titles(db: &Db, limit: i64, now: i64) -> rusqlite::Result<Vec<String>>` (id из `ext_titles`, где `title NOT GLOB '*[а-яА-ЯёЁ]*'`, нет строки в `title_localizations` и (`ru_at IS NULL` или `ru_at` старше 30 суток); порядок - сначала из подборок по позиции, как в `posters_batch`); `pub fn mark_tried(db: &Db, ids: &[String], now: i64) -> rusqlite::Result<()>` (`ru_at = now`); `pub async fn titles_ru(app: Arc<App>)` (цикл: пачка до 65, `localize`, `mark_tried`, пауза 2 с; нет работы - ждать `titles_kick` до часа).

- [ ] **Step 1: Write the failing tests:** `pending_titles_skip_cyrillic_localized_and_recent` (в базе: `Ghost Rider` без метки - попадает; `Дьявол носит Prada 2` - нет; `Street Fighter` с записью в `title_localizations` - нет; `Winter` с `ru_at` = сейчас - нет, с `ru_at` старше 30 суток - да); `mark_tried_sets_ru_at`.
- [ ] **Step 2: Run** `cargo test -p fsfy-server --lib pending_titles mark_tried` - Expected: FAIL.
- [ ] **Step 3: Implement** функции из Interfaces, `titles_ru` и подключение в `lib.rs`.
- [ ] **Step 4: Run** `cargo test -p fsfy-server --lib` и `cargo clippy -p fsfy-server --all-targets -- -D warnings` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src/discover.rs server/src/lib.rs
git commit -m "feat: фоновый поиск русских названий для подборок ленты"
```

### Task 4: Режиссёр на карточках по-русски

**Files:**
- Modify: `server/src/names.rs` (`store` получает оригинал; `ru_director`), `server/src/api/mod.rs` (`cards`, после запроса, строки ~339-360), `server/src/meta/mod.rs` (`apply`: прогрев кэша)
- Test: юнит-тесты в `server/src/names.rs`, серверный тест в `server/tests/discover.rs`

**Interfaces:**
- Produces: `names::store(db, rows: &[(String, String, &str, String)])` - (id, ru, источник, оригинальное написание; пишется в `person_names.orig`); `pub fn ru_director(db: &Db, name: &str) -> String` (по `person_names.orig`; нет в кэше - `translit(name)` без сети; кириллица возвращается как есть); `cards()` заменяет латинский `director` каждой карточки через `ru_director`; в `meta::apply` после записи описания в фоне (`tokio::spawn`) вызывается `names::localize` для копии `d.people`, чтобы кэш был заполнен к первому показу карточки.

- [ ] **Step 1: Write the failing tests:** юнит `ru_director_uses_cache_then_translit` (кэш: `nm1 / Дэмьен Шазелл / wikidata / orig "Damien Chazelle"`; `ru_director(db, "Damien Chazelle") == "Дэмьен Шазелл"`; `ru_director(db, "Kyle Chandler") == "Кайл Чандлер"`; `ru_director(db, "Тим Роббинс") == "Тим Роббинс"`); сервер `own_card_director_is_russian` в `server/tests/discover.rs`: свой фильм с `people` из режиссёра `Damien Chazelle` и записью в `person_names` - в `/api/library` у карточки `director == "Дэмьен Шазелл"`.
- [ ] **Step 2: Run** `cargo test -p fsfy-server ru_director own_card_director` - Expected: FAIL.
- [ ] **Step 3: Implement** по Interfaces; существующие вызовы `names::store` и тест `person_names_roundtrip` обновить под новую сигнатуру (`localize_with` передаёт оригинальное имя).
- [ ] **Step 4: Run** `cargo test -p fsfy-server` и clippy - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src server/tests
git commit -m "feat: режиссёр на карточках по-русски из кэша имён"
```

### Task 5: Подпись фото человека и повторяемый аудит

**Files:**
- Modify: `web/src/routes/person/+page.svelte` (атрибут `alt` фото), `CLAUDE.md`
- Create: `web/scripts/latin-audit.mjs` (рабочий скрипт аудита: обходит экраны, печатает видимую латиницу кроме `Nova`, `IMDb`, `HD`, `4K`, `VK`, `Rutube`, `YouTube`, `TMDB`, `TV`, `HDR`; подсказки `title` у карточек людей пропускает - оригинал там по замыслу)

**Interfaces:**
- Produces: `node web/scripts/latin-audit.mjs <baseURL>` печатает по каждому экрану число и список строк с латиницей; код выхода 0 (отчёт, не тест: нужен интернет).

- [ ] **Step 1:** в `person/+page.svelte` заменить `alt` с оригинального имени на `d?.info?.nameRu ?? nameRuLink ?? name`.
- [ ] **Step 2: Run** `npm run check` - Expected: PASS.
- [ ] **Step 3:** добавить `web/scripts/latin-audit.mjs` и строку про него в `CLAUDE.md` («Проверки перед пушем»: «`node web/scripts/latin-audit.mjs <адрес>` - видимая латиница на экранах, нужен интернет»).
- [ ] **Step 4: Run** демо-сервер (`FSFY_BIN=...\fsfy.exe`, `node --experimental-strip-types` с `web/e2e/setup.ts`) и `node web/scripts/latin-audit.mjs <baseURL>` - Expected: на `/ext/…` нет «English», слогана и испанских названий; на главной английских названий меньше, чем до работы воркера; оставшиеся - фильмы без русской метки на Викиданных (проверить на 3-4 примерах).
- [ ] **Step 5: Commit**

```bash
git add web/src/routes/person web/scripts CLAUDE.md
git commit -m "fix: подпись фото человека по-русски; скрипт аудита латиницы"
```

---

## Итоговая проверка

Полный набор проверок из Global Constraints и повторный аудит латиницы (Task 5, шаг 4). Сборка и установка - только по слову владельца.
