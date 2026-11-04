# Источники видео и библиотека - план работ

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** «Смотреть» на странице фильма из ленты появляется быстро и находится чаще (запасной ход на другие источники, мягкое сопоставление года и названий), сбой источника не тормозит страницу, а скан библиотеки показывает ход.

**Architecture:** серверный поиск `server/src/video_search.rs` остаётся (кэш, блокировка, ограничение числа поисков, фильтр `matches`); добавляются: допуск года ± 1 и прокатные названия, учёт здоровья источников с выключателем, новые сроки кэша, прогрев кэша для «Хочу»/«Смотрю». Фронт (`ext/[id]/+page.svelte`, `lib/links/`) после LordFilm сам идёт по остальным источникам. Библиотека: ход скана и список неопознанных.

**Tech Stack:** Rust (axum, tokio, reqwest, rusqlite), SvelteKit 5 + TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-01-sources-library-design.md`. Зависит от плана надёжности (`2026-11-01-reliability.md`, Task 2: `GET /api/admin/health`); если тот ещё не сделан, Task 3 этого плана добавляет поле `sources` позже, когда эндпоинт появится.

## Global Constraints

- Всё по-русски, обычный дефис « - », без длинных тире; зрителям - «администрация/сервер Nova».
- Внутренние имена (`fsfy`, `app.fsfy`, `X-Fsfy`) не менять; LordFilm остаётся встроенным источником и первым в очереди (решение владельца).
- Новых сетевых источников не добавлять; запросы к чужим сайтам - только через существующие функции `video_search`/`lordfilm` с их таймаутами.
- Нагрузка: прогрев - один поиск за раз, пауза 3 с, только когда никто не смотрит (нет свежего прогресса за 60 с).
- Миграции не нужны.
- Проверки: `cargo fmt --all --check`, `cargo clippy -p fsfy-server --all-targets -- -D warnings`, `cargo test -p fsfy-server`, в `web/`: `npm run check`, `npm test`, `npm run build`, затем `cargo build --release -p fsfy-server` и `npx playwright test --project=desktop --project=phone --project=tablet` с `FSFY_BIN=C:\Maison\Nova\target\release\fsfy.exe`. Сетевые проверки - `#[ignore]` и скрипт `web/scripts/search-time.mjs` вручную.
- Коммиты - Conventional Commits, автор `Claude` через `-c user.name=Claude -c user.email=noreply@anthropic.com`, только локально; push, сборка и установка - по слову владельца.

## Review Focus

- Все источники упали: страница не висит, через ≤ 30 с кнопка поиска доступна, сообщение объясняет причину («источники не ответили»), а не «ничего нет».
- Фильм 2024 года, на сайте подписанный 2025: находится; фильм 2010 года на сайте подписан 2024: не находится (допуск строго ± 1).
- Однословное название («Дюна», «Мышь») без года в подписи: не принимается за любой ролик (правило «одно слово без года» сохраняется).
- Источник, выключенный на 10 минут, возвращается сам и не остаётся выключенным навсегда.
- «Это не тот фильм» (`bad_videos`) продолжает скрывать находку и после запасного хода.

---

### Task 1: Год ± 1 и прокатные названия в сопоставлении

**Files:**
- Modify: `server/src/video_search.rs` (`matches`, строки 175-226; блок `expected`, строки 274-285; фильтр, строки 375-385), `server/src/api/links.rs` (`SearchQuery`, если нужны поля)
- Test: юнит-тесты в `server/src/video_search.rs`

**Interfaces:**
- Produces: `matches(label, title, season, episode, year, trailer)` принимает год подписи в диапазоне `year-1..=year+1` (раньше - только равный); `expected` для `ext_id` включает прокатные названия: `Option<(String, Option<String>, Option<i64>)>` дополняется списком `aka: Vec<String>` (из `ext_titles.extra -> '$.aka'`, до 3), фильтр принимает находку, если совпало любое из названий.

- [ ] **Step 1: Write the failing tests** в `video_search.rs`: `year_off_by_one_matches` (`matches("Дюна (2022)", "Дюна: Часть вторая", None, None, Some(2021), false)` - нет по названию; `matches("Первый человек (2019)", "Первый человек", None, None, Some(2018), false)` - да; `matches("Первый человек (2020)", "Первый человек", None, None, Some(2018), false)` - нет); `single_word_without_year_is_rejected` (`matches("Мышь про животных", "Мышь", None, None, Some(2024), false)` - нет, как и раньше); `aka_name_matches` (проверка функции `any_name_matches(label, &["Человек на Луне", "Первый человек"], year)`).
- [ ] **Step 2: Run** `cargo test -p fsfy-server --lib video_search::` - Expected: FAIL.
- [ ] **Step 3: Implement:** в ветке с годом заменить `found.contains(&y)` на проверку `found.iter().any(|v| (*v - y).abs() <= 1)` (правило «одно слово без совпавшего года» оставить, год считается совпавшим при ± 1); добавить `any_name_matches` и использовать её вместо двух вызовов `matches` в фильтре.
- [ ] **Step 4: Run** `cargo test -p fsfy-server --lib video_search::` и `cargo test -p fsfy-server --test discover` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src/video_search.rs server/src/api/links.rs
git commit -m "feat: поиск видео - год ± 1 и прокатные названия"
```

### Task 2: Сроки кэша поиска

**Files:**
- Modify: `server/src/video_search.rs` (`ttl`, строки 17-23, и места `remember`)
- Test: юнит-тест в том же файле

**Interfaces:**
- Produces: `fn ttl(result: &SearchResult) -> i64`: непустой успех - 3 600 000 мс (1 час), пустой успех - 1 800 000 (30 минут), ошибка - 5 000 (как сейчас).

- [ ] **Step 1: Write the failing test** `cache_ttl_by_result`: `ttl(&Ok(vec![json!({})])) == 3_600_000`, `ttl(&Ok(vec![])) == 1_800_000`, `ttl(&Err("x".into())) == 5_000`.
- [ ] **Step 2: Run** `cargo test -p fsfy-server --lib cache_ttl_by_result` - Expected: FAIL (сейчас 600 000 для любого успеха).
- [ ] **Step 3: Implement** `ttl` по Interfaces.
- [ ] **Step 4: Run** `cargo test -p fsfy-server --lib video_search::` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src/video_search.rs
git commit -m "feat: кэш поиска видео - час для находок, полчаса для пустого ответа"
```

### Task 3: Здоровье источников и выключатель

**Files:**
- Create: `server/src/video_health.rs`
- Modify: `server/src/lib.rs` (`pub mod video_health;`), `server/src/video_search.rs` (обернуть каждый из шести источников в `find`, строки ~286-358), `server/src/api/admin.rs` (`health`, если уже есть - поле `sources`), `server/src/api/mod.rs` (маршрут `GET /api/sources/status` для вошедших)
- Test: юнит-тесты в `server/src/video_health.rs`

**Interfaces:**
- Produces: `pub fn record(site: &str, ok: bool, ms: u64)`, `pub fn open(site: &str, now: i64) -> bool` (выключатель открыт - источник сейчас пропускаем: 3 ошибки подряд → пауза 600 000 мс, после паузы - одна пробная попытка), `pub fn snapshot(now: i64) -> Vec<SourceState>` где `SourceState { site: String, label: String, state: "ok" | "down" | "unknown", ok_ms: Option<u64>, fails: u32 }`; `GET /api/sources/status` отдаёт `snapshot` вошедшим зрителям без внутренних подробностей (без `fails`, `ok_ms`).

- [ ] **Step 1: Write the failing tests:** `breaker_opens_after_three_failures` (3 × `record("rutube", false, 100)` → `open("rutube", now) == true`; успех сбрасывает счётчик); `breaker_closes_after_ten_minutes` (при `now + 600_001` - `open == false`, одна пробная попытка допускается); `snapshot_marks_down_and_unknown` (источник без записей - `unknown`).
- [ ] **Step 2: Run** `cargo test -p fsfy-server --lib video_health::` - Expected: FAIL.
- [ ] **Step 3: Implement** `video_health.rs` (глобальный `OnceLock<Mutex<HashMap<…>>>`, время - параметр для тестов); в `find` перед запросом к источнику `if video_health::open(site, now) { return None }`, после ответа - `record(site, result.is_some(), elapsed_ms)`; `None` от выключенного источника не считается «не ответил» для сообщения «Источники видео сейчас не отвечают».
- [ ] **Step 4: Run** `cargo test -p fsfy-server --lib video_health:: video_search::` и clippy - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src
git commit -m "feat: здоровье источников видео и выключатель на 10 минут"
```

### Task 4: Запасной ход на другие источники

**Files:**
- Create: `web/src/lib/links/best.ts`, `web/src/lib/links/best.test.ts`
- Modify: `web/src/routes/ext/[id]/+page.svelte` (эффект поиска, строки 215-246)

**Interfaces:**
- Consumes: `searchVideos(params, publish, signal, otherSourcesOnly)` и `searchLordfilm(params, signal)` из `$lib/links/search`.
- Produces: `pickBest(found: FoundVideo[], names: string[], runtimeSec: number): FoundVideo | null` - из находок оставляет те, чья подпись содержит одно из названий (после нормализации), оценивает длительность против `runtimeSec` (чем ближе, тем лучше) и качество в подписи (`2160|4k` > `1080` > `720|hd`), возвращает лучшую или `null`; на странице: LordFilm пуст или ответил ошибкой → автоматически `searchVideos(..., true)`, `online` получает отсортированный список, первая находка - кнопка «Смотреть» с подписью источника (`sourceName`).

- [ ] **Step 1: Write the failing tests** `best.test.ts`: из `[{label:'Первый человек (2018, 1080p)', duration:8300}, {label:'Первый человек - трейлер', duration:150}, {label:'Новый Человек-паук', duration:7800}]` при `names=['первый человек']`, `runtimeSec=8460` выбирается первая; пустой список - `null`; при нескольких подходящих выше качество 4K при равной длительности.
- [ ] **Step 2: Run** `npm test -- best` - Expected: FAIL.
- [ ] **Step 3: Implement** `pickBest` (нормализация - как в `norm` страницы; вынести общий `norm` в `$lib/links/norm.ts`, если он уже есть в странице - переиспользовать) и изменить эффект: после `searchLordfilm` с пустым или упавшим результатом - запасной ход; 30-секундную отсечку оставить для всей цепочки; `sourceError = true` только если не ответил ни один источник.
- [ ] **Step 4: Run** `npm test`, `npm run check`, затем `npx playwright test e2e/episodes.spec.ts --project=desktop` (тесты страницы фильма из ленты с подменой сети) - Expected: PASS; при падении из-за нового автоматического запроса поправить подмены в тестах, не ослабляя проверки.
- [ ] **Step 5: Commit**

```bash
git add web/src web/e2e
git commit -m "feat: страница фильма сама ищет на других источниках, если LordFilm пуст"
```

### Task 5: Прогрев кэша поиска

**Files:**
- Modify: `server/src/video_search.rs` (`pub async fn warm(app: Arc<App>)` и `pub fn warm_query(title: &str, ext_id: &str) -> SearchQuery`), `server/src/lib.rs` (`tokio::spawn(video_search::warm(app.clone()))` рядом со строкой 256)
- Test: юнит-тест в `server/src/video_search.rs`

**Interfaces:**
- Consumes: `find(app, &SearchQuery)`, таблицы `ext_marks` (`status IN ('want','watching')`), `ext_titles`.
- Produces: `warm_query("Первый человек", "tt1213641")` равен запросу страницы (`q = title`, `ext_id = Some(id)`, `provider = Some("lordfilm")`, остальное пусто), чтобы ключ кэша совпал; `warm` раз в 10 минут (первый раз через 2 минуты после запуска) проходит не более 20 фильмов из «Хочу» и «Смотрю» без свежего кэша, по одному, с паузой 3 с, пропуская круг, если за последние 60 с был чей-то прогресс просмотра.

- [ ] **Step 1: Write the failing test** `warm_query_matches_page_query`: поля `q`, `ext_id`, `provider`, `season`, `episode`, `mode`, `min` у `warm_query` те же, что у запроса, который строит `ext/[id]/+page.svelte:224` для фильма (`title`, `extId`, `provider=lordfilm`).
- [ ] **Step 2: Run** `cargo test -p fsfy-server --lib warm_query` - Expected: FAIL.
- [ ] **Step 3: Implement** `warm_query` и `warm`; для сериалов (нужны сезон и серия) прогрев не делаем.
- [ ] **Step 4: Run** `cargo test -p fsfy-server --lib` и clippy - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src
git commit -m "feat: прогрев поиска видео для «Хочу» и «Смотрю»"
```

### Task 6: Понятный пустой результат

**Files:**
- Modify: `web/src/routes/ext/[id]/+page.svelte` (блок `sourceMissing`, строка ~798), `web/src/lib/api.ts` (если нужен тип), создать `web/src/lib/links/SourceStatus.svelte`
- Test: `web/e2e/episodes.spec.ts` или новый `web/e2e/sources.spec.ts`

**Interfaces:**
- Consumes: `GET /api/sources/status` (Task 3) -> `[{site, label, state}]`.
- Produces: блок «Источник не найден» показывает: что искали (название, год, оригинал), строку «Источники: LordFilm - не ответил, Rutube - ответил…» (по `state`), кнопки «Повторить» и «Вставить ссылку» (последняя - существующее окно `LinkFinder`).

- [ ] **Step 1: Write the failing e2e** `sources.spec.ts`: подмена `/api/links/search` (пусто) и `/api/sources/status` (LordFilm `down`, Rutube `ok`): на странице фильма из ленты виден текст «LordFilm - не ответил» и кнопка «Повторить».
- [ ] **Step 2: Run** `npx playwright test e2e/sources.spec.ts --project=desktop` - Expected: FAIL.
- [ ] **Step 3: Implement** `SourceStatus.svelte` и подключить в блок `sourceMissing`.
- [ ] **Step 4: Run** `npm run check` и e2e из шага 2 на `desktop` и `phone` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add web/src web/e2e
git commit -m "feat: пустой результат поиска объясняет, что искали и кто не ответил"
```

### Task 7: Ход скана библиотеки

**Files:**
- Modify: `server/src/library/scan.rs` (`scan` получает отчёт о ходе), `server/src/library/jobs.rs` (`ScanState`, строки 11-17, `scan_worker`), `server/src/api/admin.rs` (`status` уже отдаёт состояние скана - добавить поле), `web/src/lib/admin/Folders.svelte` или `Overview.svelte` (показ), `web/src/lib/admin/types.ts`
- Test: юнит-тест в `server/src/library/scan.rs` (по образцу существующих тестов модуля)

**Interfaces:**
- Produces: `ScanState.progress: Option<Progress>` где `Progress { done: usize, total: usize }`; `scan(db, images)` вызывает переданный обратный вызов `&dyn Fn(usize, usize)` после каждой папки или каждых 25 файлов; в `/api/admin/status` - `scan.progress`; на фронте - строка «Проверено N из M» при `running`.

- [ ] **Step 1: Write the failing test** `scan_reports_progress`: во временной папке 30 файлов-пустышек `.mp4` - обратный вызов получает неубывающие `done`, последний вызов `done == total`.
- [ ] **Step 2: Run** `cargo test -p fsfy-server --lib scan_reports_progress` - Expected: FAIL.
- [ ] **Step 3: Implement** обратный вызов в `scan`, запись в `app.scan_state` из `scan_worker`, поле в статусе, строка в интерфейсе.
- [ ] **Step 4: Run** `cargo test -p fsfy-server --lib`, `npm run check` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src web/src
git commit -m "feat: ход скана библиотеки виден в «Управлении»"
```

### Task 8: Неопознанные фильмы в «Каталоге»

**Files:**
- Modify: `server/src/api/admin.rs` (список каталога - фильтр `?status=unmatched`), `web/src/lib/admin/Catalog.svelte` (кнопка-фильтр «Неопознанные»)
- Test: `server/tests/flow.rs` (серверная часть)

**Interfaces:**
- Produces: список каталога принимает `status=unmatched` и возвращает свои фильмы, у которых `meta_status` не `matched`/`manual` (нет описания или сопоставление неуверенное); интерфейс показывает счётчик на кнопке-фильтре и открывает существующее окно выбора описания.

- [ ] **Step 1: Write the failing test** `catalog_filters_unmatched`: два фильма, у одного `meta_status = 'matched'`, у другого `'none'`; запрос `?status=unmatched` возвращает только второй.
- [ ] **Step 2: Run** `cargo test -p fsfy-server --test flow catalog_filters_unmatched` - Expected: FAIL.
- [ ] **Step 3: Implement** фильтр на сервере и кнопку на фронте.
- [ ] **Step 4: Run** `cargo test -p fsfy-server --test flow`, `npm run check` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src web/src server/tests
git commit -m "feat: «Неопознанные» в каталоге администратора"
```

### Task 9: Замер до и после

**Files:**
- Create: `web/scripts/search-time.mjs` (скрипт из рабочей папки: восемь фильмов - популярный, новый, нишевый, сериал; печатает время и число находок)

- [ ] **Step 1:** добавить скрипт и строку про него в `CLAUDE.md` («Проверки перед пушем»).
- [ ] **Step 2: Run** демо-сервер и `node web/scripts/search-time.mjs <baseURL>` - записать результат в итог; цель: популярные - первая находка < 2,5 с, «ничего нигде» - < 6 с.
- [ ] **Step 3: Commit**

```bash
git add web/scripts CLAUDE.md
git commit -m "chore: скрипт замера поиска видео"
```

---

## Итоговая проверка

Полный набор проверок из Global Constraints и замер Task 9. Сборка и установка - только по слову владельца.
