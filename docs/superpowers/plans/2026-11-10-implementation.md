# План реализации Nova 1.1 (после одобрения макетов)

База: `docs/superpowers/specs/2026-11-10-master-design.md`, макеты `docs/superpowers/mockups/`.
Порядок по заданию владельца. После каждого этапа: fmt/clippy/test, npm check/test/build,
релевантные e2e, замеры perf, CHANGELOG, Conventional Commit.

## Этап 0 - исправления аудита (мелкие, безопасные)
- `web/e2e/setup.ts`: поиск бинарника с `.exe` на Windows (или `fsfy.exe` fallback).
- `server/src/video_health.rs`: восстановить читаемую кодировку (UTF-8) всех строк.
- `server/src/web.rs`: `accepts()` - учитывать `q=0` и `*` в Accept-Encoding.
- CHANGELOG: Unreleased - fix.

## Этап 1 - профиль и вход (задача 1)
- `PATCH /api/me`: поле `login` - `auth::check_name` + уникальность (COLLATE NOCASE),
  мгновенное обновление в сессиях/чатах (имя профиля = имя аккаунта уже так работает).
- `profile/+page.svelte` + `Security.svelte`: показывать блок безопасности всегда
  (не только при `hasPassword`); три карточки Имя/Логин/Пароль (вариант A);
  смена логина с подтверждением, смена пароля (текущий + новый), остальные сеансы
  закрываются при смене пароля/логина.
- Тесты: `server/tests/security.rs` (смена логина: ок/занят/невалиден/сессии),
  e2e на форму.

## Этап 2 - настройки хоста (задача 2)
- Манифест `web/src/lib/admin/manifest.ts`: группы (10), у каждого пункта
  key/hint/default/тип/валидация; `GET /api/admin/settings` отдаёт `defaults`
  и `overrides`; `PUT` по одному ключу с валидацией диапазонов.
- `Settings.svelte` переписать на манифест: поиск `/`, «Сбросить», подтверждения
  через `dialogs.svelte.ts`, подсказки. Опасная зона внизу.
- Новые ключи: `chat.poll_ms` (3000), `chat.history` (60), `room.idle_min` (10),
  `media.trickplay`, `discover.feeds` - вывести в UI.

## Этап 3 - источники (задача 3, самая большая)
См. спеку §3. Подэтапы:
- 3a. `server/src/sources/`: trait Source, реестр, `settings video.sources`
  (custom + builtins enabled/order), `GET /api/admin/sources`,
  `POST /api/admin/sources/{id}/check`, CRUD своих сайтов с автодетектом типа.
- 3b. Адаптер `lordfilm-dle`: обобщение `lordfilm.rs` (список хостов,
  `/filmy`, `/serialy`, `index.php?do=search`), поиск серий через embed.
- 3c. Адаптер `embed-playlist` (namy/nextembed/stloadi/newplayjj…): парсер
  `seasons:` JSON → серии {hls, audio.names, cc[], duration}; прокси `p?u=`
  с allowlist хостов из ответа embed; VTT-субтитры через тот же прокси.
- 3d. 6 источников владельца как встроенные карточки (выключены по умолчанию):
  the-bear, friends, south-park, mg-films, lordserials, hp.
- 3e. `video_health`: ключи по id источника, статусы ok/down/unknown.
- 3f. Фронт: `admin/Sources.svelte` (карточки, тумблеры, приоритет, «Проверить»,
  «+ Добавить сайт» с автодетектом); плеер: дорожки озвучек из `audio.names`,
  субтитры `cc` как VTT-треки, качество из hls.js.
- Тесты: фикстуры HTML/JSON по каждому семейству, парсер плейлиста,
  allowlist прокси, health circuit-breaker.

## Этап 4 - чаты (задача 4)
- Тесты утечки: social.rs (не-друг, after=0, чужая комната), party.rs.
- Мелочи: backoff-реконнект опроса, индикатор связи, ORDER BY id.

## Этап 5 - безопасность (задача 5)
- `GET /api/me/export` (JSON: профиль, прогресс, списки, оценки, отзывы),
  `DELETE /api/me` (удаление аккаунта; хост - только при втором admin).
- Аудит эндпоинтов: email/IP не утекают, vis_* соблюдаются, EXIF нет.

## Этап 6 - поиск (задача 6)
- `catalog_search`: варианты запроса - раскладка ru↔en, транслит,
  Levenshtein ≤1 только при пустом точном результате; suggest-эндпоинт
  `GET /api/search/suggest`, «вы имели в виду», недавние запросы (localStorage).
- Замеры `search-time.mjs` до/после.

## Этап 7 - подсказки (задача 7)
- Пустые состояния и ошибки с «что делать дальше» на ключевых страницах;
  онбординг/тур не ломать, дополнить при необходимости.

## Этап 8 - плеер (задача 8)
- Чипы настроек в один ряд (макет), синхронизация при drift>5с, ТВ - крупный
  фокус и чат сбоку. Движок и Safari/Windows-правила не трогаем.

## Этап 9 - профиль (задача 9)
- По текстовому описанию владельца (файла нет): коллаж постеров, онлайн-точка,
  чипы часы/фильмы/серии, медали, жанры, вкладки со счётчиками, цель «Киноман»,
  нижняя панель; ПК - липкая карточка слева. Белый акцент, без анимации ободка.

## Этап 10 - устройства и ТВ (задача 10)
- ТВ: `.wgt` обёртка, вход по 6-значному коду с телефона, фокус, Chrome 69-синтаксис.
- Телефон/планшет: единая панель, safe areas.

## Этап 11 - анимации и скорость (задача 11)
- Skeleton-плашки, blur-up постеров, единая motion; замеры perf до/после;
  LCP<2с, INP<200мс, CLS<0.1, бюджет 80КБ.

## Этап 12 - зависимости (задача 12)
- Kit 3.1.x / Vite 8.3.5 (свежие минорные - после 7 дней), Tauri 2.12.x,
  rust-стабильные patch; `tauri-plugin-updater`/zip - ждать фикс или зафиксировать;
  npm audit / cargo audit, lock-файлы.

## Этап 13 - README (задача 13)
- Структура сохраняется; SVG-анимации как картинки, скриншоты через make screenshots.

## Финал
- Полный прогон проверок, `CHANGELOG.md`, коммиты Conventional Commits,
- `make desktop` (или cargo build + tauri build) → установщик/бинарь на Рабочий стол.
