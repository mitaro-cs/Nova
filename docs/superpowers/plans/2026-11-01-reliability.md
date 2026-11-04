# Надёжность и целостность - план работ

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** у приложения появляется журнал ошибок (файл + «Недавние ошибки» в «Управление → Обзор»), ошибки интерфейса зрителей попадают в тот же журнал, а «датчик целостности» (обход всех экранов в e2e) ловит ошибки консоли, 4xx/5xx и битые картинки до владельца.

**Architecture:** модуль `server/src/logs.rs` подключается к `tracing`: слой записи в ротируемый файл `<data>/logs/nova.log` и кольцевой буфер последних предупреждений и ошибок. `GET /api/admin/health` отдаёт их вместе с версией, диском и ffmpeg; `POST /api/diag/client` принимает ошибки браузера. Мелкие найденные дефекты (404 отзывов, постер без картинки) чинятся, а обход экранов закрепляется тестом.

**Tech Stack:** Rust (tracing, tracing-subscriber, axum, rusqlite), SvelteKit 5 + TypeScript, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-01-reliability-design.md`. Отличия от спецификации: «Состояние» - не новая вкладка, а расширение существующего «Обзор» (`web/src/lib/admin/Overview.svelte`, «Всё ли работает»); пункт «два `<h1>`» из спецификации снят (видимый заголовок всегда один).

## Global Constraints

- Всё по-русски, обычный дефис « - », без длинных тире; зрителям - «администрация/сервер Nova», не «хост».
- Внутренние имена (`fsfy`, `app.fsfy`, `X-Fsfy`) не менять; изменяющие запросы `/api/*` требуют `X-Fsfy: 1`.
- В журнал не попадают адреса клиентов, пароли, токены, тела запросов; запись от клиента ограничена по размеру и частоте.
- Миграции: новых не нужно (журнал - файл).
- Права: `/api/admin/*` - только `Manager`; `/api/diag/client` - любой вошедший (`Viewer`).
- Проверки: `cargo fmt --all --check`, `cargo clippy -p fsfy-server --all-targets -- -D warnings`, `cargo test -p fsfy-server`, в `web/`: `npm run check`, `npm test`, `npm run build`, затем `cargo build --release -p fsfy-server` и `npx playwright test --project=desktop --project=phone --project=tablet` с `FSFY_BIN=C:\Maison\Nova\target\release\fsfy.exe`.
- Коммиты - Conventional Commits, автор `Claude` через `-c user.name=Claude -c user.email=noreply@anthropic.com`, только локально; push, сборка и установка - по слову владельца.

## Review Focus

- Журнал не растёт бесконечно: после 2 МБ файл ротируется, хранится не больше 5 файлов.
- Нет доступа к каталогу данных (только чтение, диск полон): приложение запускается и работает, журнал просто не пишется.
- Сообщение от клиента с гигантским текстом, управляющими символами и потоком из 1000 запросов не раздувает журнал и память (обрезка 500 знаков, 20 запросов в минуту).
- Зритель (не администратор) не может прочитать `GET /api/admin/health`.
- Строка журнала не содержит пути с именем пользователя Windows, если её можно не печатать (адреса `http://…` и токены вырезаются).

---

### Task 1: Журнал в файл и кольцо ошибок

**Files:**
- Create: `server/src/logs.rs`
- Modify: `server/src/lib.rs` (`pub mod logs;`, вызов `logs::attach(&opts.data)` в начале `start`), `server/src/main.rs:19-23` и `desktop/src-tauri/src/main.rs:146-148` (инициализация через `fsfy::logs::init()`)
- Test: юнит-тесты в `server/src/logs.rs`

**Interfaces:**
- Produces: `pub fn init()` (ставит `tracing_subscriber` с выводом в консоль, слоем файла и слоем кольца; фильтр - как сейчас, `RUST_LOG` главнее), `pub fn attach(data: &Path)` (открывает `<data>/logs/nova.log`; ошибки открытия - молча, журнал остаётся только в консоли и кольце), `pub struct Line { pub at: i64, pub level: String, pub target: String, pub message: String }`, `pub fn recent(n: usize) -> Vec<Line>` (последние предупреждения и ошибки, новые первыми), `pub fn rotate(dir: &Path, max_bytes: u64, keep: usize) -> std::io::Result<()>` (`nova.log` → `nova.log.1` … `nova.log.{keep-1}`).

- [ ] **Step 1: Write the failing tests** в `logs.rs`: `rotates_after_limit` (во временном каталоге файл `nova.log` на 3 МБ при `max_bytes = 2 МБ`, `keep = 5`: после `rotate` есть `nova.log.1`, нет `nova.log`, старые сдвинуты, файлов не больше 5); `ring_keeps_only_warnings_and_errors` (через `tracing::subscriber::with_default` и слой кольца: `info!` не попадает, `warn!`/`error!` попадают, при 300 записях `recent(500).len() == 200`, первая - самая новая); `sanitize_cuts_urls_and_tokens` (`sanitize("fail http://127.0.0.1:1/x?t=abc token=zzz")` не содержит `http://` и `abc`/`zzz`).

- [ ] **Step 2: Run** `cargo test -p fsfy-server --lib logs::` - Expected: FAIL (модуля нет).

- [ ] **Step 3: Implement `logs.rs`.** Файл пишется синхронно под `Mutex` (событий мало), на каждую запись проверка размера; кольцо - `VecDeque<Line>` на 200 под `Mutex` в `OnceLock`; `sanitize` вырезает `https?://\S+` и значения после `token=`/`t=`/`password=`; сообщение режется до 500 знаков.

- [ ] **Step 4: Wire** `init()` в обоих `main.rs` вместо прямого `tracing_subscriber::fmt()`, `attach` - первой строкой `fsfy::start`. Run `cargo test -p fsfy-server --lib` и `cargo build -p fsfy-server` - Expected: PASS, сборка без предупреждений.

- [ ] **Step 5: Commit**

```bash
git add server/src/logs.rs server/src/lib.rs server/src/main.rs desktop/src-tauri/src/main.rs
git commit -m "feat: журнал ошибок в файл и кольцо последних ошибок"
```

### Task 2: Диагностика для администратора

**Files:**
- Modify: `server/src/api/admin.rs` (новый `health`), `server/src/api/mod.rs` (маршрут рядом со строкой 172), `web/src/lib/admin/Overview.svelte`, `web/src/lib/admin/types.ts`
- Test: `server/tests/security.rs` (права) и новый тест `health_has_version_disk_and_recent_errors` в `server/tests/flow.rs` (по образцу соседних, через `common::start`)

**Interfaces:**
- Consumes: `logs::recent`, `crate::monitor::snapshot(&app.data)` (как в `status`), `app.tools` (ffmpeg), `crate::VERSION`.
- Produces: `GET /api/admin/health` -> `{"version": str, "uptimeSec": i64, "dataDir": {"freeBytes": i64|null, "dbBytes": i64}, "ffmpeg": {"found": bool, "path": str|null}, "errors": [{"at","level","target","message"}]}`; тип `Health` в `types.ts`.

- [ ] **Step 1: Write the failing tests:** хост получает 200 и поле `errors` - массив, после `tracing::error!("проверка журнала")` в ответе есть эта строка; зритель получает 403.
- [ ] **Step 2: Run** `cargo test -p fsfy-server --test flow health_` - Expected: FAIL (маршрута нет, 404).
- [ ] **Step 3: Implement** `admin::health(State(app), _m: Manager)` и маршрут `/api/admin/health`; на фронте в `Overview.svelte` - карточка «Недавние ошибки» (пусто - «Ошибок нет»), строка «Диск: свободно N ГБ», «Версия», кнопка «Скопировать для отчёта» (текст ошибок в буфер обмена).
- [ ] **Step 4: Run** `cargo test -p fsfy-server --test flow --test security` и `npm run check` в `web/` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src/api server/tests web/src/lib/admin
git commit -m "feat: «Обзор» показывает недавние ошибки, диск и версию"
```

### Task 3: Ошибки интерфейса попадают в журнал

**Files:**
- Create: `web/src/lib/diag.ts`
- Modify: `server/src/api/mod.rs` (маршрут `POST /api/diag/client`), `server/src/api/admin.rs` или новый `server/src/api/diag.rs`, `web/src/routes/+layout.svelte` (вызов `installDiag()` один раз)
- Test: `server/tests/security.rs`, `web/src/lib/diag.test.ts`

**Interfaces:**
- Produces: `POST /api/diag/client` с телом `{"kind": "error"|"rejection"|"player", "message": str, "where": str}` -> `204`; лимит 20 запросов в минуту на вошедшего (через `auth::Limiter`, как в `signup_limit`); в журнал пишется `warn!` с `target = "client"`, текст через `logs::sanitize`, до 500 знаков. `installDiag(): void` - вешает `window.onerror` и `unhandledrejection`, шлёт не больше 5 сообщений за сессию страницы и не повторяет одинаковые.

- [ ] **Step 1: Write the failing tests:** сервер - `client_log_is_limited_and_sanitized` (вошедший шлёт сообщение с `http://x/?t=secret` - в `logs::recent` его нет; 21-й запрос за минуту - 429; без входа - 401); фронт - `diag.test.ts`: `installDiag` шлёт одно сообщение при двух одинаковых ошибках и не больше 5 разных.
- [ ] **Step 2: Run** `cargo test -p fsfy-server --test security client_log` и `npm test -- diag` - Expected: FAIL.
- [ ] **Step 3: Implement** обработчик и `diag.ts` (отправка через `navigator.sendBeacon` не годится из-за заголовка `X-Fsfy` - используем `fetch` с `keepalive` и этим заголовком; без ожидания ответа).
- [ ] **Step 4: Run** те же команды - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src web/src/lib/diag.ts web/src/lib/diag.test.ts web/src/routes/+layout.svelte server/tests
git commit -m "feat: ошибки интерфейса зрителей попадают в журнал"
```

### Task 4: Мелкие дефекты, найденные обходом

**Files:**
- Modify: `server/src/api/social.rs` (`ext_reviews`, строка ~186), `web/src/lib/discover/Poster.svelte` (`retryImage`, строки 29-33)
- Test: `server/tests/discover.rs` (отзывы), `web/e2e/` (постер, в задаче 5)

**Interfaces:**
- Produces: `GET /api/discover/title/{id}/reviews` для id, которого ещё нет в `ext_titles`, отвечает `200` и пустой список в том же виде, что `reviews_of` без отзывов (а не 404); для невалидного id по-прежнему 404.

- [ ] **Step 1: Write the failing test** `ext_reviews_of_unknown_title_is_empty` в `server/tests/discover.rs`: запрос по `tt0000001`, которого нет в базе, - `200` и `items == []`; по `bad` - `404`.
- [ ] **Step 2: Run** `cargo test -p fsfy-server --test discover ext_reviews_of_unknown` - Expected: FAIL (404).
- [ ] **Step 3: Implement** в `ext_reviews`: при `not_found` от `target_ext` и валидном id (`discover::valid_id`) вернуть пустой ответ в формате `reviews_of`. В `Poster.svelte`: после неудачи и запасного адреса `/api/discover/poster/{id}` не рендерить `<img>` (флаг `failed`), оставлять заглушку с названием.
- [ ] **Step 4: Run** `cargo test -p fsfy-server --test discover` и `npm run check` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src/api/social.rs server/tests/discover.rs web/src/lib/discover/Poster.svelte
git commit -m "fix: пустые отзывы нового фильма без 404, постер без картинки без битого img"
```

### Task 5: «Датчик целостности» - обход экранов в e2e

**Files:**
- Create: `web/e2e/integrity.spec.ts`
- Modify: `web/e2e/util.ts` (общая функция `watchPage(page)`: собирает ошибки консоли, `pageerror`, ответы ≥ 400 на своём адресе)
- Test: сам файл

**Interfaces:**
- Consumes: `env()` и вход из `e2e/util.ts`/`e2e/util-test.ts`, список маршрутов из `layout.spec.ts` (взять тот же перечень и дополнить: `/discover`, `/ext/tt1196946`, `/person?name=…`, `/franchise/…`, `/watch/<id>`).
- Produces: тест `integrity: все экраны` для проектов phone, tablet, desktop; провал, если на странице за 4 с есть: ошибка JS или консоли (кроме списка разрешённых), ответ 4xx/5xx на свой запрос (кроме `/api/auth/*` до входа и заранее известных «пусто»), видимая картинка с `naturalWidth === 0`, ширина документа больше окна, пустой `main`, число **видимых** `h1` не равно 1 (`checkVisibility`); сообщение называет страницу и виновника.

- [ ] **Step 1: Write the test** с перечнем маршрутов и проверками выше.
- [ ] **Step 2: Run** `npx playwright test e2e/integrity.spec.ts --project=desktop` с `FSFY_BIN=...\fsfy.exe` - Expected: PASS после задач 1-4 (если падает на `/person` из-за постера или на `/ext/…` из-за отзывов - вернуться к задаче 4; новое найденное - в «Review notes» итога, а не молча в разрешённые).
- [ ] **Step 3: Run** тот же файл на `--project=phone` и `--project=tablet` - Expected: PASS.
- [ ] **Step 4: Commit**

```bash
git add web/e2e
git commit -m "test: обход всех экранов - ошибки консоли, 4xx/5xx, битые картинки, ширина"
```

### Task 6: Лента «Обзор» легче

**Files:**
- Modify: `server/src/api/discover.rs` (ответ ленты `GET /api/discover`), `web/e2e/speed.spec.ts`
- Test: `web/e2e/speed.spec.ts`, `server/tests/discover.rs`

**Interfaces:**
- Produces: ответ `GET /api/discover` без полей, которые интерфейс обзора не читает, и не больше 20 карточек в строке с полем `more` (сколько ещё); для полной строки - существующий запрос ряда, если он есть (иначе - параметр `?row=<key>`).

- [ ] **Step 1: Write the failing test** в `speed.spec.ts`: размер ответа `/api/discover` без сжатия ≤ 150 КБ (сейчас ~268 КБ). Сначала измерить на демо-данных и записать «до» в комментарий.
- [ ] **Step 2: Run** `npx playwright test e2e/speed.spec.ts --project=desktop` - Expected: FAIL на размере.
- [ ] **Step 3: Implement:** найти поля карточки, которые `Poster.svelte`, `PosterRow.svelte` и `Shelf` не читают (проверить `grep` по `types.ts ExtCard`), убрать их из ответа ленты; ограничить строки 20 карточками. Если строка нужна целиком (кнопка «Все»), добавить параметр запроса.
- [ ] **Step 4: Run** `speed.spec.ts` и `cargo test -p fsfy-server --test discover` - Expected: PASS.
- [ ] **Step 5: Commit**

```bash
git add server/src/api/discover.rs web/e2e/speed.spec.ts server/tests/discover.rs
git commit -m "perf: лента обзора без лишних полей, строки по 20 карточек"
```

---

## Итоговая проверка

Полный набор проверок из Global Constraints. После этого - сборка и установка только по слову владельца (после того, как проработаны все части плана).
