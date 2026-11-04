<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, errorText } from '$lib/api';
  import Access from '$lib/admin/Access.svelte';
  import Catalog from '$lib/admin/Catalog.svelte';
  import Folders from '$lib/admin/Folders.svelte';
  import Overview from '$lib/admin/Overview.svelte';
  import People from '$lib/admin/People.svelte';
  import Settings from '$lib/admin/Settings.svelte';
  import Version from '$lib/admin/Version.svelte';
  import Wishes from '$lib/admin/Wishes.svelte';
  import { badges } from '$lib/social/badges.svelte';
  import type { Status } from '$lib/admin/types';
  import { session } from '$lib/session.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';

  const TABS: [string, string, IconName][] = [
    ['overview', 'Обзор', 'pulse'],
    ['folders', 'Папки', 'folder'],
    ['catalog', 'Каталог', 'film'],
    ['people', 'Люди', 'users'],
    ['wishes', 'Просьбы', 'flag'],
    ['access', 'Доступ', 'globe'],
    ['settings', 'Настройки', 'sliders'],
    ['version', 'Версия', 'download'],
  ];
  // Значки разделов - белые и серые, без цветов-украшений (решение владельца, 2 ноября): цвет в управлении значит
  // только состояние - зелёный «работает», красный «ошибка».
  const TONE: Record<string, string> = Object.fromEntries(['overview', 'folders', 'catalog', 'people', 'wishes', 'access', 'settings', 'version'].map((id) => [id, '#e4e4e7']));

  // Что в каждом разделе и что там можно сделать - хост видит это сверху раздела.
  const INFO: Record<string, { hint: string; title: string; text: string; can: string[] }> = {
    overview: { hint: 'Всё ли работает', title: 'Обзор', text: 'Состояние Nova одним взглядом: что работает, что требует внимания и кто сейчас смотрит.', can: ['Проверить, всё ли в порядке', 'Скопировать адрес для друзей', 'Увидеть, кто и что смотрит'] },
    folders: { hint: 'Где лежат фильмы', title: 'Папки', text: 'Папки на этом компьютере или внешнем диске, из которых Nova берёт фильмы и сериалы. Новые файлы подхватываются сами.', can: ['Добавить или убрать папку', 'Перетащить файлы в своё место для фильмов', 'Посмотреть, как называть папки и файлы'] },
    catalog: { hint: 'Описания и обложки', title: 'Каталог', text: 'Все фильмы и сериалы на сайте. Если описание нашлось не то - выберите верное; можно сменить обложку и фон или скрыть фильм.', can: ['Найти правильное описание', 'Изменить название, год, жанры', 'Скрыть от зрителей или убрать фильм без файлов'] },
    people: { hint: 'Зрители и права', title: 'Люди', text: 'Кто может заходить в Nova. Приглашайте ссылкой, сбрасывайте пароль, давайте право скачивать.', can: ['Пригласить ссылкой или добавить вручную', 'Сбросить пароль без почты', 'Закрыть доступ или удалить'] },
    wishes: { hint: 'Что просят добавить', title: 'Просьбы', text: 'Фильмы, которые зрители попросили добавить. Положите фильм в папку - просьба уйдёт сама.', can: ['Посмотреть, что просят', 'Отклонить просьбу'] },
    access: { hint: 'Вход из интернета', title: 'Доступ', text: 'Как друзья попадают в Nova не из вашей домашней сети. Рекомендуемый способ - CloudPub: постоянный адрес без настройки роутера.', can: ['Включить доступ из интернета', 'Сменить способ доступа', 'Узнать адрес в домашней сети'] },
    settings: { hint: 'Сервер и источники', title: 'Настройки', text: 'Как Nova работает: регистрация, откуда брать описания, ffmpeg, свои сайты с видео, почта, ночной режим.', can: ['Открыть или закрыть регистрацию', 'Выбрать источник описаний', 'Указать путь к ffmpeg'] },
    version: { hint: 'Обновления', title: 'Версия', text: 'Какая версия Nova установлена и как обновиться. Фильмы, люди и настройки при обновлении сохраняются.', can: ['Узнать версию', 'Обновить Nova'] },
  };

  const tab = $derived(page.url.searchParams.get('tab') ?? 'overview');
  const info = $derived(INFO[tab] ?? INFO.overview);
  let status = $state<Status | null>(null);
  let statusError = $state('');

  function reload() {
    api<Status>('/api/admin/status').then((s) => { status = s; statusError = ''; }, (e) => { statusError = errorText(e); });
  }
  $effect(reload);

  $effect(() => {
    if (session.me && !session.me.manage) goto('/', { replace: true });
  });

  // Короткие «лампочки» состояния в шапке: зелёная - в порядке, жёлтая - в процессе или стоит заглянуть, красная - не работает.
  type Lamp = { label: string; state: 'ok' | 'wait' | 'bad'; hint: string; href?: string };
  const lamps = $derived.by<Lamp[]>(() => {
    if (!status) return [];
    const low = status.system?.diskFree != null && status.system.diskFree < 10 * 1024 ** 3;
    const update = status.update?.state === 'available';
    return [
      { label: 'Сервер', state: 'ok', hint: 'Nova раздаёт фильмы' },
      { label: status.publicUrl ? 'Интернет' : 'Только дом', state: status.publicUrl ? 'ok' : 'wait', hint: status.publicUrl ? 'Доступ из интернета включён' : 'Доступ из интернета выключен', href: '?tab=access' },
      { label: 'ffmpeg', state: status.ffmpeg && status.ffprobe ? 'ok' : 'bad', hint: status.ffmpeg && status.ffprobe ? 'Видео любых форматов играет' : 'Нет ffmpeg - не играют MKV и AVI', href: '?tab=settings' },
      { label: 'Диск', state: low ? 'bad' : 'ok', hint: low ? 'Мало места на диске' : 'Места достаточно' },
      { label: update ? 'Есть обновление' : 'Версия', state: update ? 'wait' : 'ok', hint: update ? 'Доступна новая версия' : 'Установлена последняя', href: '?tab=version' },
    ];
  });
</script>

<div class="rw-page admin">
  <header class="rw-head">
    <div>
      <h1>Управление</h1>
      {#if status}<p>Nova {status.version}</p>{/if}
    </div>
    {#if lamps.length}
      <ul class="lamps" aria-label="Состояние Nova">
        {#each lamps as l (l.label)}
          <li class={l.state} title={l.hint}>
            {#if l.href}<a href={l.href} data-sveltekit-replacestate data-sveltekit-noscroll><i></i>{l.label}</a>{:else}<span><i></i>{l.label}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </header>
  <nav class="sections" aria-label="Разделы управления">
    {#each TABS as [id, label, icon] (id)}
      <a href="?tab={id}" class:on={tab === id} style:--tone={TONE[id]} data-sveltekit-replacestate data-sveltekit-noscroll>
        <span class="ic"><Icon name={icon} size={19} /></span><span class="lbl">{label}<small>{INFO[id]?.hint}</small></span>
        {#if id === 'version' && status?.update?.state === 'available'}<i class="new" aria-label="есть обновление"></i>{/if}
        {#if id === 'wishes' && badges.wishes}<i class="new" aria-label="есть просьбы"></i>{/if}
      </a>
    {/each}
  </nav>
  <div class="cols">
  <section class="panel rw-plate" aria-label={info.title}>
    <header class="tab-head rw-block" style:--tone={TONE[tab] ?? TONE.overview}>
      <div class="title">
        <span class="badge"><Icon name={TABS.find((x) => x[0] === tab)?.[2] ?? 'pulse'} size={20} /></span>
        <h2 class="rw-h2">{info.title}</h2>
      </div>
      <p>{info.text}</p>
      <ul aria-label="Что здесь можно сделать">{#each info.can as c (c)}<li>{c}</li>{/each}</ul>
    </header>
    {#if statusError}<p class="error" role="alert">Не удалось обновить показатели: {statusError} <button class="btn sm" onclick={reload}>Повторить</button></p>{/if}
    {#if status}
      {#if tab === 'overview'}<Overview {status} {reload} />
      {:else if tab === 'folders'}<Folders {status} window={!!session.me?.window} {reload} />
      {:else if tab === 'catalog'}<Catalog />
      {:else if tab === 'people'}<People />
      {:else if tab === 'wishes'}<Wishes />
      {:else if tab === 'access'}<Access />
      {:else if tab === 'settings'}<Settings />
      {:else if tab === 'version'}<Version {status} {reload} />
      {/if}
    {/if}
  </section>
  </div>
</div>

<style>
  /* «Витрина»: разделы - плитки одной строкой сверху (на узком экране - лента), под ними один лист с содержимым.
     Никаких боковых колонок и залезающих друг на друга плит: всё идёт сверху вниз. */
  .sections {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(128px, 1fr));
    gap: 10px;
    margin-bottom: 18px;
  }
  .sections a {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 12px 14px;
    border-radius: 20px;
    background: #ffffff0d;
    color: var(--muted);
    font-weight: 600;
    font-size: 14px;
    transition: background 0.25s var(--ease), color 0.25s, transform 0.3s var(--ease);
  }
  .sections a:hover {
    color: var(--text);
    background: #ffffff16;
    transform: translateY(-2px);
  }
  .sections a:active {
    transform: scale(0.97);
  }
  .ic {
    flex: none;
    width: 38px;
    height: 38px;
    display: grid;
    place-items: center;
    border-radius: 13px;
    background: color-mix(in srgb, var(--tone) 20%, transparent);
    color: var(--tone);
    transition: background 0.25s, color 0.25s;
  }
  .sections a.on {
    color: var(--text);
    background: var(--sheet);
  }
  .sections a.on .ic {
    background: var(--tone);
    color: #000;
  }
  .new {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #ff5c7a;
  }
  /* Лист содержимого: свой, со скруглением по всем углам, во всю ширину страницы. */
  .cols .panel.rw-plate {
    min-width: 0;
    margin-inline: 0;
    padding: calc(10 * var(--rw)) clamp(18px, 2.4vw, 34px) clamp(18px, 2.4vw, 34px);
    border-radius: 28px;
  }
  @media (max-width: 860px) {
    /* minmax(0, …): лента разделов прокручивается, а не распирает страницу. */
    .sections {
      display: flex;
      overflow-x: auto;
      margin-inline: calc(var(--gutter) * -1);
      padding-inline: var(--gutter);
    }
    .sections a {
      flex: none;
      white-space: nowrap;
    }
    .lbl small {
      display: none;
    }
    .cols .panel.rw-plate {
      margin-inline: calc(var(--gutter) * -1);
      border-radius: 28px 28px 0 0;
    }
  }

  /* Подпись под пунктом меню и шапка раздела с пояснением. */
  .lbl {
    display: grid;
    min-width: 0;
  }
  .lbl small {
    display: none;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tab-head {
    margin-bottom: 8px;
    padding-block: 4px 18px;
  }
  .tab-head .title {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }
  .tab-head .title :global(.rw-h2) {
    margin: 0;
  }
  .badge {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--tone) 22%, transparent);
    color: var(--tone);
  }
  .tab-head p {
    max-width: 72ch;
    margin: 0 0 12px;
    color: var(--muted);
  }
  /* Что здесь можно сделать - подписи-плашки, а не флажки: ничего не отмечают, только подсказывают. */
  .tab-head ul {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
  }
  .tab-head li {
    padding: 4px 12px;
    border-radius: 99px;
    border: 1px solid color-mix(in srgb, var(--tone) 35%, transparent);
    background: color-mix(in srgb, var(--tone) 9%, transparent);
    color: var(--text);
  }
  /* Лампочки состояния справа в шапке страницы. */
  .lamps {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  @media (max-width: 860px) {
    .lamps {
      justify-content: flex-start;
    }
  }
  .lamps a,
  .lamps span {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border: 1px solid var(--line);
    border-radius: 99px;
    background: rgba(255, 255, 255, 0.05);
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
  }
  .lamps a:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .lamps i {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--ok);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--ok) 22%, transparent);
  }
  .lamps .wait i {
    background: #8e8e93;
    box-shadow: 0 0 0 4px #8e8e9333;
  }
  .lamps .bad i {
    background: var(--danger);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--danger) 22%, transparent);
    animation: lamp 1.6s ease-in-out infinite;
  }
  @keyframes lamp {
    50% {
      opacity: 0.45;
    }
  }
</style>