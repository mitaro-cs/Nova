# Единый стиль Nova: план работ

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Телефонный дизайн 0.7.3 становится общим стилем Nova на всех ширинах, оранжевый заменён цветом фильма, на экранах без фильма - белый.

**Architecture:** Сервер отдаёт цвет постера и для фильмов ленты (`ext_titles.accent`). Маленький модуль `filmColor.ts` превращает его в три CSS-токена на `<html>`, экран с фильмом ставит их через `setFilm`, по умолчанию токены белые. Навигация, герой и ряды сводятся к одному компоненту на все ширины, отличия - только размеры в медиазапросах.

**Tech Stack:** Rust (axum, rusqlite), SvelteKit 3 + Svelte 5, vitest, Playwright.

**Spec:** `docs/superpowers/specs/2026-11-02-unified-style-design.md`

## Global Constraints

- Тексты интерфейса на русском, обычный дефис « - », без длинных тире.
- Файлы правятся только Edit/Write. `Get-Content`/`Set-Content` ломают кириллицу.
- Бюджет JS главной 80 КБ сжатого (`e2e/speed.spec.ts`), поднимать нельзя.
- CLS < 0,02 (`e2e/smooth.spec.ts`). Ключи `{#each}` уникальные. Сетки с лентами - `minmax(0, 1fr)`.
- Старые миграции не править. `OLDEST` не менять.
- Коммиты локально в `alpha-0.1`, Conventional Commits, в конце `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. `docs/superpowers/` в git не добавлять.
- Не пушить, не собирать установщик, не ставить на ПК, версию не поднимать: только по слову владельца.
- Веб вшит в бинарник: перед e2e `npm run build` в `web/`, затем `cargo build --release -p fsfy-server` (около 6,5 мин), `$env:FSFY_BIN='C:\Maison\Nova\target\release\fsfy.exe'`. `$env:Path` = Machine + User.

## Решения сверх спецификации

- `Hero.svelte` удаляется: вкладки Фильмы, Сериалы, Аниме получают `Showcase`. Два героя и есть причина расхождения, а удаление освобождает бюджет JS. Теряются логотип-картинка и медленный наезд кадра.
- `PhoneRow.svelte` и `PhonePoster.svelte` удаляются: `/my` переходит на `PosterRow` + `Poster`. Сейчас главная и «Моё» на телефоне выглядят по-разному.
- `--hero-tint` удаляется вместе с капсулой.

## Review Focus

1. Быстрый переход между экранами: старый экран снимает цвет позже, чем новый поставил свой. Цвет нового экрана должен остаться. Тест - Task 1, `setFilm`.
2. Постер не читается или файл исчез: задача не должна пробовать его по кругу. Тест - Task 2, `pending_accents`.
3. Чёрно-белый постер и цвет ещё не посчитан: кнопка белая с тёмным текстом, не серая. Тест - Task 1 и e2e Task 3.
4. Окно шириной 1001-1240 px: верхняя панель и правые кнопки не налезают друг на друга. Тест - Task 5.
5. Пустая библиотека (`.welcome`, нет героя) и `.flat`: полоса разделов на месте, ничего не уехало под шапку. Тест - Task 4.

---

### Task 1: Модуль цвета и токены

**Files:**
- Create: `web/src/lib/filmColor.ts`, `web/src/lib/filmColor.test.ts`
- Modify: `web/src/app.css:54-55`

**Interfaces:**
- Produces: `filmTones(accent: string | null | undefined): { fill: string; on: string; text: string } | null`; `setFilm(accent: string | null | undefined): () => void`; токены `--film`, `--on-film`, `--film-text`.

- [ ] **Step 1: Написать падающие тесты** в `filmColor.test.ts`, с функцией контраста WCAG прямо в тесте:

```ts
it('заливка читается с белым текстом на всех оттенках', () => {
  for (let h = 0; h < 360; h++) for (const s of [20, 38, 70]) {
    const t = filmTones(hslHex(h, s, 30))!;
    expect(t.on).toBe('#ffffff');
    expect(contrast(t.fill, '#ffffff')).toBeGreaterThanOrEqual(4.5);
  }
});
it('текстовый оттенок читается на панели #141414', () => {
  for (let h = 0; h < 360; h++) expect(contrast(filmTones(hslHex(h, 30, 30))!.text, '#141414')).toBeGreaterThanOrEqual(4.5);
});
it('нет цвета, мусор и серый дают null', () => {
  for (const v of [null, undefined, '', 'red', '#12', hslHex(0, 5, 30)]) expect(filmTones(v)).toBeNull();
});
it('setFilm ставит токены, отмена возвращает белые', () => {
  const off = setFilm('#5a2a2a');
  expect(document.documentElement.style.getPropertyValue('--film')).toMatch(/^#/);
  off();
  expect(document.documentElement.style.getPropertyValue('--film')).toBe('');
});
it('отмена старого экрана не снимает цвет нового', () => {
  const old = setFilm('#5a2a2a');
  setFilm('#2a5a2a');
  const now = document.documentElement.style.getPropertyValue('--film');
  old();
  expect(document.documentElement.style.getPropertyValue('--film')).toBe(now);
});
it('цвет без оттенка снимает прежний', () => {
  setFilm('#5a2a2a');
  setFilm(null);
  expect(document.documentElement.style.getPropertyValue('--film')).toBe('');
});
```

Если окружение vitest без DOM, добавить в начало файла `// @vitest-environment happy-dom` или заглушку `document.documentElement.style` - как в соседних тестах `web/src/lib`.

- [ ] **Step 2: Запустить.** `npx vitest run src/lib/filmColor.test.ts` в `web/`. Ожидание: FAIL, модуля нет.

- [ ] **Step 3: Реализовать `filmColor.ts`.** Разбор только `#rrggbb`. Насыщенность ниже 10 % - `null`. `fill`: тот же оттенок, насыщенность `clamp(s + 0.25, 0.55, 0.75)`, светлота от 0,42 вниз шагом 0,02, пока контраст с белым < 4,5. `text`: насыщенность `clamp(s + 0.3, 0.55, 0.85)`, светлота 0,74. `on` всегда `#ffffff`. `setFilm` хранит счётчик: каждый вызов получает свой номер, отмена с устаревшим номером ничего не делает; вызов с `null`-оттенком снимает три свойства и тоже занимает номер. Без зависимостей, ради бюджета JS.

- [ ] **Step 4: Токены в `app.css`.** Вместо `--m-accent` и его комментария: `--film: #ffffff; --on-film: #0a0a0a; --film-text: #ffffff;` с комментарием «Цвет фильма: по умолчанию белый, экран с фильмом ставит свой через `setFilm`». Пока `--m-accent` используется: оставить строку до Task 3.

- [ ] **Step 5: Запустить.** Та же команда. Ожидание: PASS, 6 тестов.

- [ ] **Step 6: Коммит.** `feat: цвет фильма - оттенки и токены`.

---

### Task 2: Цвет у фильмов ленты и в плеере

**Files:**
- Modify: `server/src/db.rs` (конец `MIGRATIONS`), `server/src/discover.rs:753-804` и рядом с `titles_ru`, `server/src/lib.rs:280`, `server/src/library/probe.rs:271-284`, `server/src/api/play.rs:38,47,155`, `web/src/lib/types.ts:341-354,267-281`

**Interfaces:**
- Produces: `ExtCard.accent: Option<String>` и `accent: string | null` на фронте; `PlayInfo.accent: string | null`; `discover::pending_accents(db: &Db, limit: i64) -> rusqlite::Result<Vec<(String, String)>>`; `discover::accents(app: Arc<App>)`.

- [ ] **Step 1: Написать падающие тесты** в `discover::tests`:

```rust
#[test]
fn pending_accents_takes_only_downloaded_without_color() {
    // ext: a - постер есть, цвета нет; b - постера нет; c - постер не скачался (''); d - цвет есть;
    // e - пробовали, файл не читается (accent = '').
    assert_eq!(pending_accents(&db, 10).unwrap(), vec![("a".to_string(), "a.jpg".to_string())]);
}
#[test]
fn card_gives_color_and_hides_failed_attempt() {
    // Карточка с accent '#112233' отдаёт Some, с accent '' - None.
}
```

- [ ] **Step 2: Запустить.** `cargo test -p fsfy-server --lib discover::tests`. Ожидание: ошибка компиляции, функции нет.

- [ ] **Step 3: Реализовать.**
  - Миграция 27 с комментарием `// 27 (alpha 0.7.4): цвет постера у фильмов ленты.`: `ALTER TABLE ext_titles ADD COLUMN accent TEXT;`.
  - `card_sql`: последним столбцом `NULLIF(e.accent, '')`. `card`: `accent: r.get(11)?`. Столбцы после `card_sql` никто не добавляет (`api/discover.rs:29`, `api/lists.rs:172`), индекс 11 свободен.
  - `pending_accents`: `SELECT id, poster FROM ext_titles WHERE accent IS NULL AND poster IS NOT NULL AND poster != '' LIMIT ?`.
  - `accents`: пауза 30 с, цикл по 20 штук: `app.images.path(&poster)`, `probe::tiny_rgb`, `images::accent`, запись; пути нет или ffmpeg не справился - запись `''`, чтобы не пробовать снова; между фильмами 1 с; пачка пуста - 600 с. Путь к ffmpeg взять как в `library/jobs.rs:126`.
  - `tiny_rgb`: `command` заменить на `probe::background`, оба вызова фоновые.
  - `lib.rs`: `tokio::spawn(discover::accents(app.clone()));` рядом с `titles_ru`.
  - `play.rs`: добавить `t.accent` в SELECT и `"accent"` в JSON ответа.
  - `types.ts`: `accent` в `ExtCard` и `PlayInfo`. Починить места, где карточки создаются в коде (`PlayerShell.svelte:286` и что покажет `npm run check`).

- [ ] **Step 4: Запустить.** `cargo test -p fsfy-server --lib`, затем `cargo clippy -p fsfy-server --all-targets -- -D warnings`, в `web/` - `npm run check`. Ожидание: всё зелёное, включая тесты миграций в `db.rs`.

- [ ] **Step 5: Коммит.** `feat: цвет постера у фильмов ленты и в плеере`.

---

### Task 3: Цвет на экранах вместо оранжевого

**Files:**
- Create: `web/e2e/film-color.spec.ts`, `web/src/lib/noOrange.test.ts`
- Modify: `web/src/app.css`, `web/src/lib/ui/Showcase.svelte`, `web/src/lib/ui/Header.svelte:335`, `web/src/routes/+page.svelte:334`, `web/src/routes/my/+page.svelte:66`, `web/src/lib/ui/FeedCard.svelte:201`, `web/src/lib/ui/PhoneRow.svelte:40`, `web/src/routes/search/+page.svelte:247`, `web/src/routes/feed/+page.svelte`, `web/src/routes/title/[id]/+page.svelte`, `web/src/routes/ext/[id]/+page.svelte`, `web/src/routes/watch/link/[id]/+page.svelte:73`, `web/src/lib/player/PlayerShell.svelte`, `web/src/lib/player/Seekbar.svelte:128-129`, `web/src/lib/title/RewindHero.svelte:276`, `web/src/lib/title/MobileHero.svelte:253`, `web/src/lib/title/SeriesHero.svelte`

**Interfaces:**
- Consumes: `setFilm`, токены из Task 1; `accent` из Task 2.

- [ ] **Step 1: Написать падающего стража** `noOrange.test.ts`: обойти файлы `.svelte`, `.css`, `.ts` в `web/src` (кроме самого теста) и проверить, что нет `--m-accent`, `#ff5500`, `#f7931e`.

- [ ] **Step 2: Написать падающий `e2e/film-color.spec.ts`.** Подменить ответ `**/api/titles/*`, дописав в настоящий JSON `accent: '#7a2a3a'`; подменить трейлеры как в `integrity.spec.ts`.
  - `фильм красит главную кнопку`: на `/title/<Ла-Ла Ленд>` у ссылки или кнопки «Смотреть» `background-color` не `rgb(255, 255, 255)` и совпадает с `--film`.
  - `без фильма снова белое`: со страницы фильма переход по ссылке на `/search` (не `goto`), `--film` на `<html>` пуст, у активной вкладки или пункта панели цвет `rgb(255, 255, 255)`.
  - `нет цвета - белая кнопка`: `accent: null`, кнопка белая с тёмным текстом.
  - `главная: цвет идёт за кадром`: подменить `**/api/library`, раздав разным карточкам разный `accent`; нажать вторую точку карусели, значение `--film` сменилось.

- [ ] **Step 3: Запустить стража.** `npx vitest run src/lib/noOrange.test.ts`. Ожидание: FAIL со списком файлов.

- [ ] **Step 4: Заменить оранжевый.**
  - Текстовые места (`Header.svelte:335`, `+page.svelte:334`, `my:66`, `FeedCard:201`, `PhoneRow:40`, `search:247`, `Showcase:253`) - `var(--film-text)`.
  - `Showcase.svelte:228,251`: кнопка `background: var(--film); color: var(--on-film)`, без градиента, размеры телефона прежние.
  - `Showcase.svelte`: убрать `const accent`, `--acc` и установку `--hero-tint`; `.eyebrow` и `.btn.glass.on` - цветом `var(--film-text)`.
  - `RewindHero:276`, `MobileHero:253` и главная кнопка `SeriesHero` (найти по «Смотреть»): `var(--film)` / `var(--on-film)`. Состояния `:hover` проверить глазами: при белом токене вид прежний.
  - `Seekbar:128-129` `.played` - `var(--film-text)`, бегунок остаётся белым.
  - У перечисленных элементов добавить `transition: color .5s, background-color .5s, border-color .5s`, не теряя существующих `transition` на `transform`.
  - Удалить `--m-accent` из `app.css`.

- [ ] **Step 5: Поставить цвет на экранах.** Везде `$effect(() => setFilm(<цвет>))`, `$effect` сам вызовет отмену:
  - `Showcase`: `selected?.card?.accent ?? selected?.ext?.accent`.
  - `feed/+page`: цвет карточки с ключом `current`.
  - `title/[id]`: `c?.accent`. `ext/[id]`: `c?.accent`.
  - `PlayerShell`: `info?.accent`. Комната использует тот же `PlayerShell`, отдельной правки не нужно.
  - `watch/link/[id]`: `accent` из ответа подробностей на строке 73 (`card.accent` для ленты, `accent` для своего).

- [ ] **Step 6: Запустить.** В `web/`: `npx vitest run`, `npm run check`. Ожидание: зелёное. Собрать (см. Global Constraints), `npx playwright test film-color --project=desktop --project=phone`. Ожидание: PASS. Шаг `film-color` с локальными `Hero`-вкладками ещё не проверяет, это Task 4.

- [ ] **Step 7: Коммит.** `feat: цвет фильма вместо оранжевого на всех экранах`.

---

### Task 4: Один герой на все ширины

**Files:**
- Delete: `web/src/lib/ui/Hero.svelte`
- Modify: `web/src/routes/+page.svelte`, `web/src/lib/ui/Showcase.svelte`, `web/src/lib/ui/ShowcaseTrailer.svelte`, `web/src/routes/profile/+page.svelte:152`, `web/e2e/catalog.spec.ts:29`

**Interfaces:**
- Consumes: `.strip` в `+page.svelte`, `Showcase({ items: { card?, ext? }[] })`.

- [ ] **Step 1: Написать падающие e2e** в `catalog.spec.ts`:
  - `полоса разделов видна на всех ширинах`: `getByRole('navigation', { name: 'Разделы кинотеатра' })` видна на `/` во всех проектах; строку 29 заменить этим же локатором без ветки `desktop`.
  - `герой вкладки - те же кнопки`: на `/?tab=movies` в `section[aria-label="Рекомендуем"]` есть «Смотреть» или «Продолжить» и кнопка `Не интересно`.
  - `пустая библиотека`: подменить `**/api/library` пустым каталогом, полоса разделов видна и её верх не выше низа шапки (`boundingBox`).

- [ ] **Step 2: Запустить.** Собрать, `npx playwright test catalog --project=desktop`. Ожидание: новые тесты FAIL.

- [ ] **Step 3: Главная.**
  - `+page.svelte:196`: `<Showcase items={hero.map((card) => ({ card }))} />`, импорт `Hero` убрать, файл удалить. Проверить `grep Hero.svelte`.
  - Правила `.strip` и `.home { padding-top }` вынести из `@media (max-width: 1000px)` на все ширины, размер шрифта `clamp(17px, 1.5vw, 20px)`.
  - `.home { --row: 0px }` на всех ширинах; убрать правило `--row: clamp(...)` и `margin-top` у `.rows`, комментарии про Apple TV обновить. `.ph-hero` и `.ph-hero.wait` той же высоты, что и `.showcase` на этой ширине.
  - `.welcome` и `.flat .rows`: верхний отступ без `--header`, шапка уже учтена в `.home`.

- [ ] **Step 4: Кнопки и трейлер.**
  - `Showcase`: убрать `.nope { display: none }` и условие `phone` у трейлера, `Trailer` грузить всегда лениво. Круг и `.lbl` скрытая подпись - вне медиазапроса, круг 52 на широких, 47 до 640 px. Добавить `title` «Хочу» и «Не интересно». `padding-top` героя без шапки.
  - `Showcase` и `ShowcaseTrailer`: комментарии про «только телефон» обновить.
  - `profile/+page.svelte:152`: «Трейлеры в герое», ключ `nova-trailers` прежний, текст подсказки без слова «телефон».

- [ ] **Step 5: Запустить.** В `web/`: `npm run check`. Собрать, `npx playwright test catalog smooth speed layout`. Ожидание: зелёное, главная ≤ 80 КБ, CLS < 0,02. Если бюджет превышен: смотреть вывод `speed.spec`, ужимать импорт главной, сам бюджет не менять.

- [ ] **Step 6: Коммит.** `feat: один герой и полоса разделов на всех ширинах`.

---

### Task 5: Верхняя панель вместо капсулы

**Files:**
- Modify: `web/src/lib/ui/Header.svelte`, `web/src/lib/library.ts:9-14`, `web/e2e/layout.spec.ts`

**Interfaces:**
- Consumes: `HeaderMenu` kind `bell` и `top`, `--film-text`.

- [ ] **Step 1: Найти зависимости.** `grep -r "pill\|TABS" web/src web/e2e`, особенно `Tour.svelte` и `Onboarding.svelte`. Селекторы, что ведут на капсулу, перевести на новую панель.

- [ ] **Step 2: Написать падающий e2e** в `layout.spec.ts`, `верхняя панель на 1024 и 1240`: только `desktop`, `setViewportSize` на обе ширины; видны «Главное», «Что смотреть», «Моё», кнопки поиска и уведомлений, аватар; `expectClean`; прямоугольник ссылки «Моё» левее кнопки поиска без пересечения.

- [ ] **Step 3: Реализовать.**
  - `<nav class="pill">` заменить на `<nav class="top" aria-label="Разделы">`: «Главное» `/` (выбрано на `/` и `/discover`), «Что смотреть» `/feed`, «Моё» `/my` и `/lists`. Шрифт 15 px, 600, серый `#8e8e93`, выбранный `var(--film-text)`, `aria-current="page"`. Сетка шапки `auto 1fr auto`, панель сразу за знаком.
  - Удалить все правила `.pill`, `--hero-tint` и `.find`.
  - Справа на всех ширинах: поиск (`.find-m`), колокольчик (`.bell`), затем прежние кнопки хоста и аватар. Правила ≤ 640 px не менять. Ниже 1240 px подпись «Nova» уже скрыта.
  - `tab`, `TABS` в `Header` больше не нужны; убрать импорт, `TABS` оставить, если нужен полосе, иначе удалить из `library.ts` вместе с тестом.
  - Верхний комментарий файла обновить.

- [ ] **Step 4: Запустить.** В `web/`: `npm run check`, `npx vitest run`. Собрать, `npx playwright test layout integrity catalog beta6 selfcheck`. Ожидание: зелёное.

- [ ] **Step 5: Коммит.** `feat: верхняя панель с теми же разделами, что и на телефоне`.

---

### Task 6: Ряды, «Что смотреть» и поиск на широких экранах

**Files:**
- Delete: `web/src/lib/ui/PhoneRow.svelte`, `web/src/lib/ui/PhonePoster.svelte`
- Modify: `web/src/lib/discover/PosterRow.svelte`, `web/src/lib/discover/Poster.svelte`, `web/src/routes/my/+page.svelte`, `web/src/lib/ui/FeedCard.svelte`, `web/src/routes/feed/+page.svelte`, `web/src/routes/search/+page.svelte:49-59,131-146,247`

**Interfaces:**
- Consumes: `PosterRow({ title, items, href, ranked, i, note })`.

- [ ] **Step 1: Написать падающие e2e.**
  - В `layout.spec.ts`, `что смотреть на всю ширину`: только `desktop`, ширина `.fc` на `/feed` больше 80 % окна, кадр левее текста без пересечения.
  - `одна карточка играет`: на `/feed` не больше одной `.fc` с элементом `video` или трейлером, используя подмену трейлера из `integrity.spec.ts`.
  - `пустой поиск везде с подборками`: на `/search` во всех проектах виден `section[aria-label="Категории"]`.

- [ ] **Step 2: Запустить.** Собрать, `npx playwright test layout --project=desktop`. Ожидание: новые тесты FAIL.

- [ ] **Step 3: Ряды.**
  - `PosterRow`: заголовок 20 px 700, «Все» 16 px 600 цветом `var(--film-text)`.
  - `Poster`: плашка оценки как у `PhonePoster`, ширина `clamp(148px, 13vw, 220px)`. Сохранить `QuickMarks`, `rank`, `local`.
  - `/my`: `PhoneRow` заменить на `PosterRow`, файлы `PhoneRow` и `PhonePoster` удалить.

- [ ] **Step 4: «Что смотреть».** `feed/+page`: снять `max-width: 720px` и `margin-inline`. `FeedCard` больше 1000 px: сетка `minmax(0, 3fr) minmax(0, 2fr)`, кадр слева, `.info` справа, высота карточки не больше `calc(100svh - var(--header) - 48px)`. Порог `IntersectionObserver` 0,6 оставить: активна одна карточка. Комментарии про телефон обновить.

- [ ] **Step 5: Поиск.** Убрать условие `compact` у блока подборок и у `$effect` загрузки. `compact` оставить только для placeholder, автофокуса и значка фильтров. Стили `.suggest`, `.tiles`, `.cats` вынести из медиазапроса, плитки на широких переносятся строками.

- [ ] **Step 6: Запустить.** `npm run check`, собрать, `npx playwright test layout integrity smooth speed`. Ожидание: зелёное.

- [ ] **Step 7: Коммит.** `feat: один вид рядов, «Что смотреть» и поиск на широких экранах`.

---

### Task 7: Цвет на телевизоре

**Files:**
- Modify: `web/tv/title.ts:59-60`, `web/tv/tv.css:148-151`

**Interfaces:**
- Consumes: `filmTones` из `web/src/lib/filmColor.ts` (чистый TypeScript, без Svelte; `setFilm` на телевизоре не используется).

- [ ] **Step 1: Реализовать.** В `title.ts` посчитать `filmTones(d.accent)` и дать кнопке `.btn.pri` встроенный `style="background:…;color:…"`, если оттенок есть. Правило `.btn:focus` в `tv.css` остаётся белым: фокус на телевизоре важнее цвета. `d.accent` приходит из `/api/titles/{id}`, при необходимости дописать в тип `web/tv`.

- [ ] **Step 2: Проверить.** В `web/`: `npm run build:tv`. Ожидание: сборка без ошибок. В файлах `web/static/tv` нет `color-mix`. Если Chromium 69 (снимок `Win/576753`) есть под рукой - открыть `/tv/` по DevTools и посмотреть кнопку; если нет - сказать в отчёте, что вид не проверен.

- [ ] **Step 3: Коммит.** `feat: цвет фильма на кнопке телевизора`.

---

### Task 8: Итоговая проверка и снимки

**Files:**
- Modify: `CLAUDE.md`, `CHANGELOG.md`, при необходимости `web/.shots/*` (не в git)

- [ ] **Step 1: Документация.** `CLAUDE.md`: заметка «Единый стиль (0.7.4)»: токены и `setFilm`, белый без фильма, `ext_titles.accent` и `discover::accents`, верхняя панель, удалённые `Hero`, `PhoneRow`, `PhonePoster`. Поправить устаревшее: «Навигация (1.0)», `--m-accent` в заметке 0.7.3, `ui/Hero.svelte` в разделе «Главная». `CHANGELOG.md`: секция «Не выпущено» простыми словами.

- [ ] **Step 2: Все проверки.** `cargo fmt --all --check`, `cargo clippy -p fsfy-server --all-targets -- -D warnings`, `cargo test -p fsfy-server`, в `web/`: `npm run check`, `npm test`, `npm run build`, затем `cargo build --release -p fsfy-server` и `npx playwright test`. Вывод писать в `web/.shots/style-e2e.txt`. Ожидание: зелёное. Упавшее чинить; нестабильное называть отдельно с результатом повтора.

- [ ] **Step 3: Снимки.** Демо (`web/.shots/serve-demo.mts`) + `vite dev` с `FSFY_URL`, сессия `e2e/.state.json`. Экраны: главная, фильмы, что смотреть, моё, поиск, профиль, фильм, плеер на 390, 768, 1024, 1440. В демо у постеров должен быть цвет; если его нет, подменить `accent` при съёмке и сказать об этом. Трейлеры в демо без интернета не играют - это тоже сказать.

- [ ] **Step 4: Проверка всей ветки.** Отдельный проверяющий смотрит весь диапазон коммитов по спецификации и Review Focus (`superpowers:requesting-code-review`). Critical и Important - чинить с тестом, который сначала падал.

- [ ] **Step 5: Показать владельцу.** Отправить снимки (`SendUserFile`), перечислить что не проверено. Сборка, установка и выпуск - только по его слову. Падение CI на `e0add23` разобрать по логу до выпуска.
