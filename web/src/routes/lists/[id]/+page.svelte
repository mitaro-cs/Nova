<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, del, errorText, patch, post, put } from '$lib/api';
  import { day } from '$lib/library';
  import type { Member, SharedItem, SharedListFull } from '$lib/types';
  import Avatar from '$lib/ui/Avatar.svelte';
  import { ask } from '$lib/ui/dialogs.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import Select from '$lib/ui/Select.svelte';
  import { session } from '$lib/session.svelte';
  import { artW } from '$lib/ui/art';
  import { plural, runtime } from '$lib/fmt';
  import Modal from '$lib/ui/Modal.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import { extSheet, remember } from '$lib/discover/sheet.svelte';

  // Общая подборка «Плейлист» (выбор владельца 26.11): слева обложка из четырёх постеров, название, сколько фильмов и
  // времени, участники и кнопки («Смотреть вместе», «Добавить», «Позвать»); справа - список как плейлист: номер, постер,
  // фильм, кто добавил, голоса «хочу смотреть» и длительность. Порядок - сначала не просмотренное с большим числом голосов.
  // Группы - разделами списка. Меняют все участники.
  const id = $derived(Number(page.params.id));
  let l = $state<SharedListFull | null>(null);
  let error = $state('');
  let inviting = $state(false);
  let friends = $state<Member[]>([]);

  function load() {
    api<SharedListFull>(`/api/lists/${id}`).then((r) => (l = r), (e) => (error = errorText(e)));
  }
  $effect(() => {
    void id;
    void extSheet.version;
    load();
  });

  const key = (i: SharedItem) => (i.ext ? i.ext.id : `t${i.card!.id}`);
  const path = (i: SharedItem) => (i.ext ? `/api/lists/${id}/ext/${i.ext.id}` : `/api/lists/${id}/items/${i.card!.id}`);
  // Группы по порядку, «Без группы» - последней (или единственной, если групп нет).
  // Внутри - сначала то, что ещё не смотрели.
  const sections = $derived.by(() => {
    if (!l) return [];
    const order = (a: SharedItem, b: SharedItem) =>
      Number(!!a.watchedAt) - Number(!!b.watchedAt) || (b.votes ?? 0) - (a.votes ?? 0) || b.addedAt - a.addedAt;
    const out = l.groups.map((g) => ({ id: g.id as number | null, name: g.name, items: l!.items.filter((i) => i.groupId === g.id).sort(order) }));
    const rest = l.items.filter((i) => !l!.groups.some((g) => g.id === i.groupId)).sort(order);
    if (rest.length || !out.length) out.push({ id: null, name: l.groups.length ? 'Без группы' : '', items: rest });
    return out;
  });

  async function watched(i: SharedItem) {
    const before = i.watchedAt;
    i.watchedAt = before ? null : Date.now();
    try {
      await put(`${path(i)}/watched`, { value: !before });
    } catch {
      i.watchedAt = before;
    }
  }

  async function remove(i: SharedItem) {
    if (!l) return;
    l.items = l.items.filter((x) => x !== i);
    await put(path(i), { value: false }).catch(load);
  }

  async function move(i: SharedItem, value: string) {
    const group = value ? Number(value) : null;
    const before = i.groupId;
    i.groupId = group;
    try {
      await put(`/api/lists/${id}/move`, { titleId: i.card?.id ?? null, extId: i.ext?.id ?? null, group });
    } catch (x) {
      i.groupId = before;
      toast(errorText(x));
    }
  }

  let grouping = $state<{ id: number | null; name: string } | null>(null);
  let groupOpen = $state(false);
  const editGroup = (g: { id: number | null; name: string }) => ((grouping = g), (groupOpen = true));
  async function saveGroup(e: SubmitEvent) {
    e.preventDefault();
    const g = grouping;
    if (!g || !g.name.trim()) return;
    try {
      if (g.id) await patch(`/api/lists/${id}/groups/${g.id}`, { name: g.name });
      else await post(`/api/lists/${id}/groups`, { name: g.name });
      groupOpen = false;
      load();
    } catch (x) {
      toast(errorText(x));
    }
  }

  async function dropGroup(g: { id: number | null; name: string }) {
    if (!g.id || !(await ask(`Удалить группу «${g.name}»? Фильмы останутся в подборке.`, 'Удалить', true))) return;
    await del(`/api/lists/${id}/groups/${g.id}`).catch((x) => toast(errorText(x)));
    load();
  }

  let renaming = $state(false);
  let newName = $state('');
  async function rename(e: SubmitEvent) {
    e.preventDefault();
    if (!l || !newName.trim()) return;
    try {
      await patch(`/api/lists/${id}`, { name: newName });
      l.name = newName.trim();
      renaming = false;
    } catch (x) {
      toast(errorText(x));
    }
  }

  async function leave() {
    if (!l) return;
    const q = l.mine ? `Удалить подборку «${l.name}» у всех участников?` : `Выйти из подборки «${l.name}»?`;
    if (!(await ask(q, l.mine ? 'Удалить' : 'Выйти', true))) return;
    await del(`/api/lists/${id}`).catch(() => {});
    goto('/lists', { replace: true });
  }

  async function openInvite() {
    inviting = true;
    friends = (await api<Member[]>('/api/members').catch(() => [] as Member[])).filter(
      (m) => m.relation === 'friend' && !l?.members.some((x) => x.id === m.id),
    );
  }

  async function invite(m: Member) {
    try {
      await post(`/api/lists/${id}/members`, { id: m.id });
      inviting = false;
      toast(`${m.name} теперь в подборке`);
      load();
    } catch (x) {
      toast(errorText(x));
    }
  }

  const poster = (i: SharedItem) => (i.ext ? i.ext.poster : (i.card!.poster ?? i.card!.posterLarge));
  const title = (i: SharedItem) => (i.ext ? i.ext.title : i.card!.title);
  const href = (i: SharedItem) => (i.ext ? `/ext/${i.ext.id}` : `/title/${i.card!.id}`);
  const info = (i: SharedItem) => {
    const c = i.ext ?? i.card!;
    return [c.year, c.kind === 'series' ? 'Сериал' : null, (c.genres ?? [])[0]].filter(Boolean).join(' · ');
  };
  const mins = (i: SharedItem) => (i.card?.runtime ?? null);
  const cover = $derived((l?.items ?? []).filter((i) => poster(i)).slice(0, 4).map(poster));
  const total = $derived((l?.items ?? []).reduce((n, i) => n + (mins(i) ?? 0), 0));
  const meta = $derived(
    l
      ? [`${l.items.length} ${plural(l.items.length, 'фильм', 'фильма', 'фильмов')}`, total ? runtime(total) : null, l.ownerName ? `создал(а) ${l.ownerName}` : null]
          .filter(Boolean)
          .join(' · ')
      : '',
  );
  const leader = $derived(sections.flatMap((g) => g.items).find((i) => !i.watchedAt) ?? null);

  async function vote(i: SharedItem) {
    const before = { votes: i.votes ?? 0, voted: !!i.voted };
    i.voted = !before.voted;
    i.votes = before.votes + (i.voted ? 1 : -1);
    try {
      const r = await put<{ votes: number; voted: boolean }>(`/api/lists/${id}/vote`, { item: i.ext ? `e${i.ext.id}` : `t${i.card!.id}`, value: i.voted });
      i.votes = r.votes;
    } catch (x) {
      i.votes = before.votes;
      i.voted = before.voted;
      toast(errorText(x));
    }
  }

  // «Смотреть вместе»: пустая комната, позвать всех участников, свой фильм-лидер голосования - сразу в очередь.
  let starting = $state(false);
  async function together() {
    if (!l || starting) return;
    starting = true;
    try {
      const room = await post<{ id: string }>('/api/rooms', {});
      const me = session.me?.user.profile.id;
      await Promise.all(l.members.filter((m) => m.id !== me).map((m) => post(`/api/rooms/${room.id}/invite`, { profileId: m.id }).catch(() => {})));
      if (leader?.card) {
        const t = await api<{ files?: { id: number }[] }>(`/api/titles/${leader.card.id}`).catch(() => null);
        const f = t?.files?.[0];
        if (f) await post(`/api/rooms/${room.id}/queue`, { fileId: f.id }).catch(() => {});
      }
      goto(`/room/${room.id}`);
    } catch (x) {
      toast(errorText(x));
    } finally {
      starting = false;
    }
  }
</script>

<div class="rw-page pl">
  <a class="back muted" href="/lists"><Icon name="back" size={16} /> Все подборки</a>
  {#if error}
    <div class="empty"><Icon name="info" size={36} /> {error}</div>
  {:else if !l}
    <div class="ph-head skeleton"></div>
  {:else}
    <div class="layout">
      <aside class="head">
        <div class="cover" class:one={cover.length < 4} aria-hidden="true">
          {#each cover.length >= 4 ? cover : cover.slice(0, 1) as a, i (i)}<img src={artW(a, 342)} alt="" decoding="async" />{/each}
          {#if !cover.length}<span class="none"><Icon name="film" size={40} /></span>{/if}
        </div>
        <span class="lab">Общая подборка</span>
        <h1>{l.name}</h1>
        <p class="muted meta">{meta}</p>
        <div class="faces">
          <span class="avs">{#each l.members.slice(0, 5) as m (m.id)}<a href="/people/{m.id}" title={m.name}><Avatar avatar={m.avatar} photo={m.photo} size={36} /></a>{/each}</span>
          <span class="muted names">{l.members.map((m) => m.name).join(', ')}</span>
        </div>
        <div class="acts">
          <button class="btn primary" onclick={together} disabled={starting}><Icon name="play" size={16} /> Смотреть вместе</button>
          <a class="btn" href="/discover"><Icon name="plus" size={16} /> Добавить</a>
          <button class="btn" onclick={openInvite}><Icon name="users" size={16} /> Позвать</button>
        </div>
        <div class="acts small">
          <button class="btn sm ghost" onclick={() => ((newName = l?.name ?? ''), (renaming = true))}><Icon name="edit" size={14} /> Переименовать</button>
          <button class="btn sm ghost" onclick={() => editGroup({ id: null, name: '' })}><Icon name="plus" size={14} /> Новая группа</button>
          <button class="btn sm ghost" onclick={leave}><Icon name={l.mine ? 'trash' : 'logout'} size={14} /> {l.mine ? 'Удалить' : 'Выйти'}</button>
        </div>
      </aside>

      <div class="tracks" class:grp={l.groups.length > 0} style:--tools={l.groups.length ? '230px' : '72px'}>
        {#if !l.items.length}
          <div class="empty">
            <Icon name="film" size={40} />
            <p>Подборка пуста. Откройте фильм здесь или в «Каталоге» и добавьте его в подборку.</p>
            <a class="btn sm" href="/discover">Открыть каталог</a>
          </div>
        {:else}
          <div class="tr th" aria-hidden="true"><span>#</span><span></span><span>Фильм</span><span class="by">Добавил(а)</span><span>Голоса</span><span class="len">Время</span><span></span></div>
        {/if}
        {#each sections as g (g.id ?? 'rest')}
          {#if g.items.length || g.id}
            <section aria-label={g.name || 'Фильмы'}>
              {#if g.name}
                <div class="ghead">
                  <h2>{g.name} <span class="faint">{g.items.length}</span></h2>
                  {#if g.id}
                    <button class="mini" aria-label="Переименовать группу" title="Переименовать" onclick={() => editGroup({ id: g.id, name: g.name })}><Icon name="edit" size={14} /></button>
                    <button class="mini" aria-label="Удалить группу" title="Удалить группу" onclick={() => dropGroup(g)}><Icon name="trash" size={14} /></button>
                  {/if}
                </div>
              {/if}
              {#if g.items.length}
                <ol class="list">
                  {#each g.items as i, n (key(i))}
                    <li class="tr" class:done={!!i.watchedAt}>
                      <span class="num faint">{n + 1}</span>
                      <a class="po" href={href(i)} onclick={() => i.ext && remember(i.ext)} tabindex="-1" aria-hidden="true">{#if poster(i)}<img src={artW(poster(i), 185)} alt="" loading="lazy" decoding="async" />{/if}</a>
                      <span class="ttl">
                        <a href={href(i)} onclick={() => i.ext && remember(i.ext)}><b>{title(i)}</b></a>
                        <small>{info(i)}{i.watchedAt ? ` · посмотрели ${day(i.watchedAt)}` : ''}</small>
                      </span>
                      <span class="by">{#if i.addedBy}<Avatar avatar={i.addedBy.avatar} photo={i.addedBy.photo} size={24} /><span>{i.addedBy.name}</span>{/if}</span>
                      <span><button class="vote" class:on={i.voted} aria-pressed={!!i.voted} aria-label="Хочу смотреть: {i.votes ?? 0}" title="Хочу смотреть" onclick={() => vote(i)}><Icon name="thumbup" size={14} /> {i.votes ?? 0}</button></span>
                      <span class="len faint">{mins(i) ? runtime(mins(i)) : ''}</span>
                      <span class="tools">
                        <button class="mini" class:on={!!i.watchedAt} aria-label={i.watchedAt ? 'Не смотрели' : 'Посмотрели вместе'} title={i.watchedAt ? 'Снять отметку' : 'Посмотрели вместе'} onclick={() => watched(i)}><Icon name="check" size={15} stroke={2.6} /></button>
                        {#if l.groups.length}
                          <Select class="move" variant="inline" label="Группа" value={String(i.groupId ?? '')} onchange={(v) => move(i, v)} options={[{ value: '', label: 'Без группы' }, ...l.groups.map((x) => ({ value: String(x.id), label: x.name }))]} />
                        {/if}
                        <button class="mini" aria-label="Убрать из подборки" title="Убрать из подборки" onclick={() => remove(i)}><Icon name="close" size={15} /></button>
                      </span>
                    </li>
                  {/each}
                </ol>
              {:else}
                <p class="faint none">Пусто - переложите сюда фильмы выбором группы в строке фильма.</p>
              {/if}
            </section>
          {/if}
        {/each}
      </div>
    </div>
  {/if}
</div>

<Modal bind:open={renaming} title="Название подборки">
  <form class="rename" onsubmit={rename}>
    <input class="input" bind:value={newName} maxlength="60" required aria-label="Название" />
    <button class="btn primary">Сохранить</button>
  </form>
</Modal>

<Modal bind:open={groupOpen} title={grouping?.id ? 'Группа' : 'Новая группа'}>
  {#if grouping}
    <form class="rename" onsubmit={saveGroup}>
      <input class="input" bind:value={grouping.name} maxlength="60" required placeholder="Например: Марвел, На выходные" aria-label="Название группы" />
      <button class="btn primary">{grouping.id ? 'Сохранить' : 'Создать'}</button>
    </form>
  {/if}
</Modal>

<Modal bind:open={inviting} title="Позвать друга">
  {#if friends.length}
    <ul class="pick">
      {#each friends as f (f.id)}
        <li><button onclick={() => invite(f)}><Avatar avatar={f.avatar} photo={f.photo} size={32} /> <span>{f.name}</span></button></li>
      {/each}
    </ul>
  {:else}
    <p class="muted">Все друзья уже здесь - или друзей пока нет. Добавьте их в разделе «Люди».</p>
  {/if}
</Modal>

<style>
  .back { display: inline-flex; align-items: center; gap: 4px; font-size: 14px; margin-bottom: 18px; }
  .layout { display: grid; grid-template-columns: minmax(0, 320px) minmax(0, 1fr); gap: clamp(24px, 3vw, 44px); align-items: start; }
  .head { display: grid; gap: 10px; align-content: start; }
  @media (min-width: 861px) and (min-height: 900px) { .head { position: sticky; top: calc(var(--header, 64px) + 16px); } }
  .cover { display: grid; grid-template-columns: 1fr 1fr; aspect-ratio: 1; border-radius: 24px; overflow: hidden; background: var(--bg-3, #18181b); margin-bottom: 8px; }
  .cover.one { grid-template-columns: 1fr; }
  .cover img { width: 100%; max-width: none; height: 100%; min-height: 0; object-fit: cover; }
  .cover .none { display: grid; place-items: center; color: var(--faint); }
  .lab { font-size: 12px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; color: var(--faint); }
  h1 { margin: 0; font-size: clamp(30px, 3.2vw, 44px); font-weight: 900; letter-spacing: -0.04em; line-height: 1.04; overflow-wrap: anywhere; }
  .meta { margin: 0; font-size: 14px; }
  .faces { display: flex; align-items: center; gap: 12px; min-width: 0; margin-top: 6px; }
  .avs { display: flex; flex: none; }
  .avs a { display: block; border-radius: 50%; box-shadow: 0 0 0 3px var(--bg, #000); }
  .avs a:not(:first-child) { margin-left: -10px; }
  .names { min-width: 0; font-size: 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 10px; }
  .acts.small { margin-top: 0; gap: 4px; }

  .tracks { min-width: 0; display: grid; gap: 8px; }
  .list { margin: 0; padding: 0; list-style: none; display: grid; gap: 2px; }
  .tr {
    display: grid;
    grid-template-columns: 28px 46px minmax(0, 1fr) 150px 74px 72px var(--tools, 72px);
    gap: 14px;
    align-items: center;
    min-width: 0;
    padding: 8px 12px;
    border-radius: 14px;
    transition: background 0.15s;
  }
  .list .tr:hover { background: #ffffff0a; }
  .th { color: var(--faint); font-size: 12px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; padding-block: 4px; }
  .num { text-align: center; font-variant-numeric: tabular-nums; font-size: 14px; }
  .po { display: block; width: 46px; aspect-ratio: 2 / 3; border-radius: 8px; overflow: hidden; background: var(--bg-3, #18181b); }
  .po img { width: 100%; max-width: none; height: 100%; object-fit: cover; }
  .ttl { min-width: 0; display: grid; gap: 2px; }
  .ttl b, .ttl small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ttl a { display: block; min-width: 0; }
  .ttl a:hover b { text-decoration: underline; }
  .ttl small { color: var(--muted); font-size: 13px; }
  .tr.done .po, .tr.done .ttl b { opacity: 0.55; }
  .by { display: flex; align-items: center; gap: 8px; min-width: 0; font-size: 14px; }
  .by span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .len { font-size: 14px; white-space: nowrap; }
  .vote {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border: 0;
    border-radius: 999px;
    background: #ffffff14;
    color: var(--text);
    font: inherit;
    font-weight: 700;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    transition: background 0.15s, color 0.15s, transform 0.2s var(--ease, ease);
  }
  .vote:hover { background: #ffffff24; }
  .vote:active { transform: scale(0.94); }
  .vote.on { background: #fff; color: #000; }
  .tools { display: flex; align-items: center; gap: 6px; justify-content: flex-end; }
  .tools :global(.move) { max-width: 140px; font-size: 12px; }
  .ghead { display: flex; align-items: center; gap: 6px; margin: 18px 12px 6px; }
  .ghead h2 { margin: 0 6px 0 0; font-size: 18px; letter-spacing: -0.02em; }
  .none { font-size: 14px; margin: 0 12px; }
  .mini {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 50%;
    border: 1px solid var(--line);
    background: var(--surface);
    color: var(--text);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .mini:hover { background: var(--surface-2); }
  .mini.on { background: #fff; color: #000; }
  .ph-head { height: 360px; border-radius: 24px; }
  .empty .btn { margin-top: 6px; }

  @media (max-width: 1180px) {
    .tr { grid-template-columns: 24px 46px minmax(0, 1fr) 74px var(--tools, 72px); }
    .tr .by, .tr .len { display: none; }
  }
  @media (max-width: 860px) {
    .layout { grid-template-columns: minmax(0, 1fr); }
    .head { position: static; grid-template-columns: 132px minmax(0, 1fr); column-gap: 18px; }
    .cover { grid-row: span 4; margin: 0; border-radius: 18px; }
    .head > :not(.cover) { grid-column: 2; }
    .head .acts { grid-column: 1 / -1; }
  }
  @media (max-width: 560px) {
    .head { grid-template-columns: 104px minmax(0, 1fr); }
    .tr { grid-template-columns: 40px minmax(0, 1fr) auto auto; gap: 10px; padding-inline: 4px; }
    .tr .num { display: none; }
    .th { display: none; }
    .po { width: 40px; }
    .grp .tr > .tools { grid-column: 2 / -1; justify-content: flex-start; }
    .tools :global(.move) { max-width: none; flex: 1; }
  }

  .rename { display: grid; gap: 14px; }
  .rename .btn { justify-self: end; }
  .pick { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; }
  .pick button { width: 100%; display: flex; align-items: center; gap: 10px; padding: 8px 10px; border: 0; border-radius: 10px; background: none; cursor: pointer; text-align: left; }
  .pick button:hover { background: var(--surface-2); }
</style>
