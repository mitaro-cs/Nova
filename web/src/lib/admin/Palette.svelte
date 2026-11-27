<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, errorText, post } from '$lib/api';
  import Avatar from '$lib/ui/Avatar.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';
  import Modal from '$lib/ui/Modal.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import { href, SECTIONS } from './nav';

  // Поиск по «Управлению» (Ctrl K): любой раздел, настройка, действие или зритель за два нажатия, стрелками и Enter.
  let { open = $bindable(false), reload }: { open: boolean; reload: () => void } = $props();

  interface Item { group: 'Действия' | 'Настройки' | 'Разделы' | 'Зрители'; title: string; note: string; words?: string; icon?: IconName; to?: string; act?: 'backup'; avatar?: number }
  const SETTINGS: [string, string, string, string][] = [
    ['Регистрация и места беты', 'viewers', 'signup', 'мест лимит бета открыть закрыть приглашения'],
    ['Журнал входов', 'viewers', 'logins', 'вход пароль неверный'],
    ['Тема недели', 'library', 'showcase', 'витрина главная'],
    ['Видео по ссылке и свои сайты', 'sources', 'video', 'rutube vk сайт lordfilm источник'],
    ['Описания и постеры: IMDb, TMDB, Кинопоиск', 'sources', 'meta', 'ключ tmdb кинопоиск imdb описание'],
    ['ffmpeg и ночной режим', 'system', 'server', 'ffmpeg ночь нагрузка кэш копия'],
    ['Адрес для зрителей', 'system', 'net', 'cloudpub интернет доступ адрес домашняя сеть'],
    ['Почта и безопасность', 'system', 'security', 'почта smtp два шага управлять из интернета'],
    ['Обновить Nova', 'system', 'update', 'версия обновление'],
  ];
  const base: Item[] = [
    { group: 'Действия', title: 'Сделать копию базы', note: 'Система', icon: 'download', act: 'backup', words: 'бэкап резервная' },
    { group: 'Действия', title: 'Добавить фильмы', note: 'Библиотека', icon: 'plus', to: href('library', 'folders'), words: 'папка загрузить' },
    { group: 'Действия', title: 'Пригласить зрителя', note: 'Зрители', icon: 'mail', to: href('viewers', 'people'), words: 'приглашение ссылка позвать' },
    { group: 'Действия', title: 'Мастер первой настройки', note: 'Пять шагов', icon: 'sparkle', to: '?tab=setup', words: 'настройка' },
    ...SETTINGS.map(([title, s, sub, words]): Item => ({ group: 'Настройки', title, note: SECTIONS.find((x) => x.id === s)!.label, icon: 'sliders', to: href(s, sub), words })),
    ...SECTIONS.flatMap((s) => s.subs.map((x): Item => ({ group: 'Разделы', title: `${s.label} · ${x.label}`, note: s.hint, icon: s.icon, to: href(s.id, x.id) }))),
  ];

  let q = $state('');
  let sel = $state(0);
  let users = $state<Item[]>([]);
  let input: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (!open) return;
    q = '';
    sel = 0;
    setTimeout(() => input?.focus(), 30);
    api<{ id: number; login: string; name: string; role: string }[]>('/api/admin/users').then(
      (u) => (users = u.map((x) => ({ group: 'Зрители', title: x.name, note: '@' + x.login, icon: 'user', to: href('viewers', 'people') + '&find=' + encodeURIComponent(x.login) }))),
      () => {},
    );
  });

  const low = (s: string) => s.toLowerCase().replaceAll('ё', 'е');
  const items = $derived.by(() => {
    const t = low(q.trim());
    const all = [...base, ...users];
    const hit = t ? all.filter((i) => low(`${i.title} ${i.note} ${i.words ?? ''}`).includes(t)) : all.filter((i) => i.group !== 'Зрители' && i.group !== 'Разделы');
    return hit.slice(0, 14);
  });
  $effect(() => {
    if (sel >= items.length) sel = Math.max(0, items.length - 1);
  });

  async function run(i: Item) {
    open = false;
    if (i.act === 'backup') {
      try {
        await post('/api/admin/backup');
        toast('Копия базы готова');
        reload();
      } catch (e) {
        toast(errorText(e));
      }
    } else if (i.to) goto(i.to, { replace: true, reset: false });
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); sel = Math.min(sel + 1, items.length - 1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); sel = Math.max(sel - 1, 0); }
    else if (e.key === 'Enter' && items[sel]) { e.preventDefault(); run(items[sel]); }
  }
</script>

<Modal bind:open title="Найти в управлении">
  <input bind:this={input} class="input q" bind:value={q} onkeydown={key} placeholder="Зритель, настройка или действие" aria-label="Что найти" aria-controls="pal-list" autocomplete="off" />
  <ul id="pal-list" class="list" role="listbox" aria-label="Найдено">
    {#each items as i, n (i.group + i.title + i.note)}
      {#if n === 0 || items[n - 1].group !== i.group}<li class="grp" role="presentation">{i.group}</li>{/if}
      <li role="option" aria-selected={n === sel}>
        <button type="button" class:sel={n === sel} onmouseenter={() => (sel = n)} onclick={() => run(i)}>
          {#if i.avatar != null}<Avatar avatar={i.avatar} size={24} />{:else}<span class="ic"><Icon name={i.icon ?? 'right'} size={16} /></span>{/if}
          <b>{i.title}</b><small>{i.note}</small>
        </button>
      </li>
    {:else}
      <li class="grp" role="presentation">Ничего не нашлось</li>
    {/each}
  </ul>
</Modal>

<style>
  .q { width: 100%; margin-bottom: 8px; font-size: 16px; }
  .list { display: grid; gap: 2px; max-height: min(56vh, 520px); margin: 0; padding: 0; overflow-y: auto; list-style: none; }
  .grp { padding: 10px 10px 4px; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--faint, #71717a); }
  button { display: grid; grid-template-columns: 24px minmax(0, 1fr) auto; gap: 12px; align-items: center; width: 100%; padding: 9px 10px; border: 0; border-radius: 12px; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  button.sel { background: #ffffff14; }
  .ic { display: grid; place-items: center; width: 24px; height: 24px; border-radius: 8px; background: #ffffff12; }
  b { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  small { color: var(--muted); font-size: 12.5px; white-space: nowrap; }
</style>
