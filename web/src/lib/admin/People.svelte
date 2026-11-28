<script lang="ts">
  import { api, del, errorText, patch, post } from '$lib/api';
  import { ago, plural } from '$lib/fmt';
  import { artW } from '$lib/ui/art';
  import Avatar from '$lib/ui/Avatar.svelte';
  import { session } from '$lib/session.svelte';
  import { ask } from '$lib/ui/dialogs.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Modal from '$lib/ui/Modal.svelte';
  import Select from '$lib/ui/Select.svelte';

  interface U {
    id: number;
    login: string;
    name: string;
    role: 'host' | 'admin' | 'viewer';
    canDownload: boolean;
    disabled: boolean;
    seenAt: number | null;
    hasPassword: boolean;
    avatar?: number;
    photo?: string | null;
    hours?: number;
    watched?: number;
    recent?: (string | null)[];
  }
  interface Inv {
    code: string;
    link: string | null;
    note: string | null;
    usesLeft: number | null;
    expiresAt: number | null;
    used: number;
    active: boolean;
  }

  let users = $state<U[]>([]);
  let invites = $state<Inv[]>([]);
  let error = $state('');
  let copied = $state('');
  let resetResult = $state<{name:string;password:string}|null>(null);

  let invOpen = $state(false);
  let inv = $state({ note: '', uses: '1', days: '7' });
  let created = $state<{ code: string; link: string | null } | null>(null);

  let userOpen = $state(false);
  let nu = $state({ name: '', login: '', password: '', role: 'viewer' });

  const ROLES = { host: 'Хост', admin: 'Администратор', viewer: 'Зритель' };
  // Поиск и фильтры (27.11): при сотне зрителей список без них не читается. `find` - из поиска по управлению (Ctrl K).
  let { find = '', seats = null }: { find?: string; seats?: { used: number; total: number } | null } = $props();
  let open = $state<U | null>(null);
  let pOpen = $state(false);
  let q = $state('');
  let filter = $state<'all' | 'online' | 'admins' | 'idle' | 'off'>('all');
  $effect(() => { q = find; });
  const DAY = 86_400_000;
  const FILTERS = [['all', 'Все'], ['online', 'Заходили за сутки'], ['admins', 'Администраторы'], ['idle', 'Давно не заходили'], ['off', 'Доступ закрыт']] as const;
  const match = (u: U, f: typeof filter) =>
    f === 'all' ? true : f === 'online' ? !!u.seenAt && Date.now() - u.seenAt < DAY : f === 'admins' ? u.role !== 'viewer' : f === 'idle' ? u.role !== 'host' && (!u.seenAt || Date.now() - u.seenAt > 30 * DAY) : u.disabled;
  const shown = $derived.by(() => {
    const t = q.trim().toLowerCase();
    return users.filter((u) => match(u, filter) && (!t || u.name.toLowerCase().includes(t) || u.login.toLowerCase().includes(t)));
  });
  const isHost = $derived(session.me?.user.role === 'host');

  function load() {
    api<U[]>('/api/admin/users').then((u) => { users = u; if (open) open = u.find((x) => x.id === open!.id) ?? null; }, (e) => (error = errorText(e)));
    api<Inv[]>('/api/admin/invites').then((i) => (invites = i), () => {});
  }
  $effect(load);

  async function copy(text: string) {
    await navigator.clipboard?.writeText(text).catch(() => {});
    copied = text;
    setTimeout(() => (copied = ''), 1500);
  }

  async function createInvite(e: SubmitEvent) {
    e.preventDefault();
    try {
      created = await post('/api/admin/invites', { note: inv.note, uses: Number(inv.uses) || null, days: Number(inv.days) || null });
      load();
    } catch (x) {
      error = errorText(x);
    }
  }

  async function removeInvite(code: string) {
    await del(`/api/admin/invites/${code}`).catch(() => {});
    load();
  }

  async function createUser(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    try {
      await post('/api/admin/users', nu);
      userOpen = false;
      nu = { name: '', login: '', password: '', role: 'viewer' };
      load();
    } catch (x) {
      error = errorText(x);
    }
  }

  async function update(u: U, body: Record<string, unknown>) {
    error = '';
    try {
      await patch(`/api/admin/users/${u.id}`, body);
      load();
    } catch (x) {
      error = errorText(x);
    }
  }

  async function remove(u: U) {
    if (!(await ask(`Удалить ${u.name}? Его прогресс и список тоже удалятся.`, 'Удалить', true))) return;
    await del(`/api/admin/users/${u.id}`).catch((x) => (error = errorText(x)));
    load();
  }

  async function resetPassword(u: U) {
    const alphabet = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789';
    const bytes = crypto.getRandomValues(new Uint8Array(18));
    const pw = Array.from(bytes, b => alphabet[b % alphabet.length]).join('');
    if (!(await ask(`Сбросить пароль для ${u.name}? Все его сеансы будут завершены. После сброса новый пароль появится здесь - передайте его человеку лично.`, 'Сменить'))) return;
    error = '';
    try { await patch(`/api/admin/users/${u.id}`, {password: pw}); load(); } catch (x) { error = errorText(x); return; }
    resetResult = {name: u.name, password: pw};
  }
</script>

{#if error}<p class="error">{error}</p>{/if}
{#if resetResult}<section class="card pane" role="status"><h3>Новый пароль · {resetResult.name}</h3><p>Передайте пароль человеку лично. После входа его можно изменить в профиле.</p><code style="overflow-wrap:anywhere">{resetResult.password}</code><button class="btn" onclick={() => copy(resetResult!.password)}>{copied === resetResult.password ? 'Скопировано' : 'Скопировать пароль'}</button><button class="btn ghost" onclick={() => resetResult = null}>Скрыть</button></section>{/if}

<!-- Зрители карточками, как актёры на странице фильма (28.11): портрет, когда был, часы, последние постеры, роль.
     Нажатие - окно человека с правами и действиями. -->
<div class="vhead">
  <div>
    {#if seats}<div class="bignum">{seats.used}<small> из {seats.total} мест</small></div><div class="meter" aria-hidden="true"><i style:transform="scaleX({Math.min(1, seats.used / Math.max(1, seats.total))})"></i></div>
    {:else}<div class="bignum">{users.filter((u) => u.role !== 'host').length}<small> {plural(users.filter((u) => u.role !== 'host').length, 'зритель', 'зрителя', 'зрителей')}</small></div>{/if}
  </div>
  <div class="hacts">
    <button class="btn primary" onclick={() => { created = null; invOpen = true; }}><Icon name="plus" size={16} /> Пригласить</button>
    <button class="btn" onclick={() => (userOpen = true)}>Добавить вручную</button>
  </div>
</div>
<div class="find">
  <div class="chips" role="group" aria-label="Кого показать">
    {#each FILTERS as [id, label] (id)}
      <button type="button" class="chip" aria-pressed={filter === id} onclick={() => (filter = id)}>{label} <span>{users.filter((u) => match(u, id)).length}</span></button>
    {/each}
  </div>
  <input class="input" type="search" bind:value={q} placeholder="Имя или логин" aria-label="Найти зрителя" />
</div>
{#if !shown.length}<p class="muted small">Никого не нашлось</p>{/if}
<ul class="cards" aria-label="Зрители">
  {#each shown as u, n (u.id)}
    <li style:--i={n}>
      <button type="button" class="person" class:off={u.disabled} onclick={() => { open = u; pOpen = true; }} aria-label="{u.name}, {ROLES[u.role]}">
        <Avatar avatar={u.avatar ?? 0} photo={u.photo ?? null} size={96} />
        <b>{u.name}</b>
        <small>{u.disabled ? 'доступ закрыт' : u.seenAt ? `был ${ago(u.seenAt)}` : 'ещё не заходил'}</small>
        <small>{u.hours ?? 0} ч · {u.watched ?? 0} {plural(u.watched ?? 0, 'фильм', 'фильма', 'фильмов')}</small>
        {#if u.recent?.length}<span class="mini" aria-hidden="true">{#each u.recent as r, k (k)}<img src={artW(r, 92)} alt="" loading="lazy" />{/each}</span>{/if}
        <span class="role" class:v={u.role === 'viewer'}>{ROLES[u.role]}</span>
      </button>
    </li>
  {/each}
</ul>

<Modal bind:open={pOpen} title={open?.name ?? 'Зритель'}>
  {#if open}
    {@const u = open}
    <div class="sheet">
      <div class="who"><Avatar avatar={u.avatar ?? 0} photo={u.photo ?? null} size={88} /><div><b>{u.name}</b><small>@{u.login} · {ROLES[u.role]} · {u.disabled ? 'доступ закрыт' : u.seenAt ? `был ${ago(u.seenAt)}` : 'ещё не заходил'}</small></div></div>
      <div class="facts"><div><strong>{u.hours ?? 0}</strong><span>часов</span></div><div><strong>{u.watched ?? 0}</strong><span>{plural(u.watched ?? 0, 'фильм', 'фильма', 'фильмов')}</span></div></div>
      {#if u.role !== 'host'}
        <label class="srow"><span><b>Может скачивать</b><small>Архив сезона или фильма</small></span><input type="checkbox" role="switch" class="tg" checked={u.canDownload} onchange={(e) => update(u, { canDownload: (e.target as HTMLInputElement).checked })} /></label>
        <label class="srow"><span><b>Доступ к сайту</b><small>Выключено - не войдёт, место беты сохраняется</small></span><input type="checkbox" role="switch" class="tg" checked={!u.disabled} aria-label="Доступ" onchange={(e) => update(u, { disabled: !(e.target as HTMLInputElement).checked })} /></label>
        {#if isHost}
          <div class="srow"><span><b>Роль</b><small>Администратор видит «Управление»</small></span><Select variant="sm" fit label="Роль" value={u.role} onchange={(v) => update(u, { role: v })} options={[{ value: 'viewer', label: 'Зритель' }, { value: 'admin', label: 'Администратор' }]} /></div>
        {/if}
        <div class="acts">
          <button class="btn sm" aria-label="Сбросить пароль" onclick={() => resetPassword(u)}><Icon name="refresh" size={15} /> Сбросить пароль</button>
          <button class="btn sm ghost danger" aria-label="Удалить" onclick={() => remove(u)}><Icon name="trash" size={15} /> Удалить</button>
        </div>
      {:else}
        <p class="muted small">Хост - владелец сервера. Его права не меняются.</p>
      {/if}
    </div>
  {/if}
</Modal>


<section class="card pane">
  <div class="head">
    <h3><Icon name="mail" size={18} /> Приглашения</h3>
    <button
      class="btn primary sm"
      onclick={() => {
        created = null;
        invOpen = true;
      }}><Icon name="plus" size={15} /> Пригласить</button
    >
  </div>
  <p class="muted small">Ссылка-приглашение: человек откроет её, придумает логин и пароль - и сразу смотрит.</p>
  {#if invites.length}
    <ul class="inv">
      {#each invites as i (i.code)}
        <li class:off={!i.active}>
          <code>{i.code}</code>
          <span class="muted">
            {i.note ?? ''}
            {i.usesLeft === null ? 'без ограничений' : `осталось ${i.usesLeft}`}{i.expiresAt ? ` · до ${new Date(i.expiresAt).toLocaleDateString('ru-RU')}` : ''}
            · вошли {i.used}
          </span>
          <span class="grow"></span>
          {#if i.link && i.active}
            <button class="btn sm" onclick={() => copy(i.link!)}><Icon name={copied === i.link ? 'check' : 'link'} size={14} /> Ссылка</button>
          {/if}
          <button class="btn sm icon ghost" aria-label="Удалить" onclick={() => removeInvite(i.code)}><Icon name="trash" size={15} /></button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<Modal bind:open={invOpen} title="Новое приглашение">
  {#if created}
    <div class="created">
      <p>Отправьте ссылку другу:</p>
      {#if created.link}
        <code class="big">{created.link}</code>
        <button class="btn primary" onclick={() => copy(created!.link!)}><Icon name="copy" size={15} /> {copied ? 'Скопировано' : 'Копировать ссылку'}</button>
      {:else}
        <p class="muted">Код: <code>{created.code}</code>. Включите доступ из интернета, чтобы появилась ссылка.</p>
      {/if}
    </div>
  {:else}
    <form class="form" onsubmit={createInvite}>
      <label class="field"><span>Для кого (необязательно)</span><input class="input" bind:value={inv.note} maxlength="80" placeholder="Например: Саша" /></label>
      <div class="two">
        <label class="field">
          <span>Сколько человек</span>
          <Select label="Сколько человек" bind:value={inv.uses} options={[{ value: '1', label: 'Один' }, { value: '5', label: 'До 5' }, { value: '0', label: 'Без ограничений' }]} />
        </label>
        <label class="field">
          <span>Действует</span>
          <Select label="Действует" bind:value={inv.days} options={[{ value: '1', label: '1 день' }, { value: '7', label: 'Неделю' }, { value: '30', label: 'Месяц' }, { value: '0', label: 'Всегда' }]} />
        </label>
      </div>
      <button class="btn primary">Создать ссылку</button>
    </form>
  {/if}
</Modal>

<Modal bind:open={userOpen} title="Добавить человека">
  <form class="form" onsubmit={createUser}>
    <label class="field"><span>Имя</span><input class="input" bind:value={nu.name} required /></label>
    <label class="field"><span>Логин</span><input class="input" bind:value={nu.login} required minlength="3" autocomplete="off" /></label>
    <label class="field"><span>Пароль</span><input class="input" bind:value={nu.password} required minlength="6" autocomplete="new-password" /></label>
    {#if isHost}
      <label class="field">
        <span>Роль</span>
        <Select label="Роль" bind:value={nu.role} options={[{ value: 'viewer', label: 'Зритель' }, { value: 'admin', label: 'Администратор - управляет каталогом и людьми' }]} />
      </label>
    {/if}
    <button class="btn primary">Добавить</button>
  </form>
</Modal>

<style>
  /* Карточки зрителей (28.11). */
  .vhead { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 24px; align-items: end; margin-bottom: 8px; }
  .bignum { font-size: clamp(64px, 8vw, 120px); font-weight: 900; letter-spacing: -0.06em; line-height: 0.85; font-variant-numeric: tabular-nums; }
  .bignum small { font-size: 0.3em; color: var(--muted); letter-spacing: -0.02em; }
  .meter { max-width: 520px; height: 6px; margin-top: 16px; border-radius: 6px; background: #ffffff1f; overflow: hidden; }
  .meter i { display: block; height: 100%; background: #fff; transform-origin: 0 50%; }
  .hacts { display: flex; gap: 10px; flex-wrap: wrap; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 18px; margin: 0 0 30px; padding: 0; list-style: none; }
  .person { display: flex; flex-direction: column; align-items: center; gap: 4px; width: 100%; min-width: 0; height: 100%; padding: 22px 16px 18px; border: 0; border-radius: 24px; background: radial-gradient(120% 80% at 50% 0%, #1d1d21, #111113 70%); color: inherit; font: inherit; text-align: center; cursor: pointer; transition: transform 0.3s var(--ease); }
  .person:hover { transform: translateY(-4px); }
  .person.off { opacity: 0.55; }
  .person :global(.ava) { margin-bottom: 10px; }
  .person b { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 17px; font-weight: 800; }
  .person small { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); font-size: 13px; }
  .mini { display: flex; gap: 6px; margin-top: 12px; }
  .mini img { width: 44px; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 7px; }
  .role { margin-top: auto; padding: 4px 10px; border-radius: 99px; background: #fff; color: #000; font-size: 11px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; }
  .person .role { margin-top: 12px; }
  .role.v { background: #ffffff14; color: var(--muted); }
  .sheet { display: grid; gap: 4px; }
  .sheet .who { display: flex; align-items: center; gap: 16px; margin-bottom: 8px; }
  .sheet .who b { display: block; font-size: 26px; font-weight: 900; letter-spacing: -0.03em; }
  .sheet .who small { color: var(--muted); }
  .sheet .facts { display: flex; margin: 6px 0 8px; }
  .sheet .facts div { padding: 0 22px; border-left: 1px solid var(--line); }
  .sheet .facts div:first-child { padding-left: 0; border-left: 0; }
  .sheet .facts strong { display: block; font-size: 30px; font-weight: 900; letter-spacing: -0.04em; line-height: 1.1; }
  .sheet .facts span { color: var(--muted); font-size: 13px; }
  .srow { display: flex; justify-content: space-between; align-items: center; gap: 14px; padding: 14px 0; border-top: 1px solid var(--line); cursor: pointer; }
  .srow > span { display: grid; min-width: 0; }
  .srow small { color: var(--muted); font-size: 13px; }
  .tg { appearance: none; flex: none; position: relative; width: 50px; height: 30px; margin: 0; border-radius: 99px; background: #3a3a3c; cursor: pointer; transition: background 0.2s; }
  .tg::after { content: ''; position: absolute; top: 3px; left: 3px; width: 24px; height: 24px; border-radius: 50%; background: #fff; transition: transform 0.25s var(--ease); }
  .tg:checked { background: var(--ok); }
  .tg:checked::after { transform: translateX(20px); }
  .sheet .acts { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 14px; }
  @media (max-width: 640px) {
    .vhead { grid-template-columns: minmax(0, 1fr); }
    .cards { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
    .person :global(.ava) { width: 72px; height: 72px; }
  }
  .find { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 10px; margin: 4px 0 12px; }
  .find .input { flex: 0 1 240px; min-width: 0; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { display: inline-flex; align-items: center; gap: 6px; padding: 6px 12px; border: 0; border-radius: 99px; background: #ffffff12; color: var(--muted); font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
  .chip span { opacity: 0.7; font-variant-numeric: tabular-nums; }
  .chip[aria-pressed='true'] { background: #fff; color: #000; }
  .pane {
    padding: 18px 20px;
    margin-bottom: 16px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
  }
  .small {
    font-size: 13px;
    margin: 6px 0 12px;
  }
  .inv {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .inv li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: rgba(0, 0, 0, 0.2);
    font-size: 13px;
  }
  .inv li.off {
    opacity: 0.5;
  }
  .grow {
    flex: 1;
  }
  code {
    padding: 2px 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.35);
    font-size: 13px;
    letter-spacing: 0.05em;
  }
  .form {
    display: grid;
    gap: 14px;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .form .btn {
    justify-self: start;
  }
  .created {
    display: grid;
    gap: 12px;
    justify-items: start;
  }
  .created p {
    margin: 0;
  }
  .big {
    display: block;
    width: 100%;
    padding: 12px;
    font-size: 14px;
    word-break: break-all;
    letter-spacing: 0;
  }
</style>
