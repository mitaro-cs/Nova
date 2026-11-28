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
    goto('/my', { replace: true });
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
  const total = $derived((l?.items ?? []).reduce((n, i) => n + (mins(i) ?? 0), 0));
  const meta = $derived(
    l
      ? [`${l.items.length} ${plural(l.items.length, 'фильм', 'фильма', 'фильмов')}`, total ? runtime(total) : null]
          .filter(Boolean)
          .join(' · ')
      : '',
  );
  const me = $derived(session.me?.user.profile.id ?? 0);
  type Order = 'votes' | 'new' | 'short';
  const ORDERS: [Order, string][] = [['votes', 'По голосам'], ['new', 'Новые'], ['short', 'Короткие']];
  let view = $state<'want' | 'done'>('want');
  let order = $state<Order>('votes');
  let group = $state<number | null>(null);
  let q = $state('');
  let more = $state(false);
  const want = $derived((l?.items ?? []).filter((i) => !i.watchedAt));
  const done = $derived((l?.items ?? []).filter((i) => !!i.watchedAt));
  const low = (s: string) => s.toLowerCase().replaceAll('ё', 'е');
  const rows = $derived.by(() => {
    const t = low(q.trim());
    const list = (view === 'want' ? want : done).filter((i) => (group === null || i.groupId === group) && (!t || low(title(i)).includes(t)));
    const by: Record<Order, (a: SharedItem, b: SharedItem) => number> = {
      votes: (a, b) => (b.votes ?? 0) - (a.votes ?? 0) || b.addedAt - a.addedAt,
      new: (a, b) => b.addedAt - a.addedAt,
      short: (a, b) => (mins(a) ?? 999) - (mins(b) ?? 999),
    };
    return list.slice().sort(view === 'done' ? (a, b) => (b.watchedAt ?? 0) - (a.watchedAt ?? 0) : by[order]);
  });
  function random() {
    const i = want[Math.floor(Math.random() * want.length)];
    if (i?.ext) remember(i.ext);
    if (i) goto(href(i));
  }
  const leader = $derived(want.slice().sort((a, b) => (b.votes ?? 0) - (a.votes ?? 0) || b.addedAt - a.addedAt)[0] ?? null);
  const voters = (i: SharedItem) => (i.voters ?? []).map((v) => l?.members.find((m) => m.id === v)).filter((m): m is NonNullable<typeof m> => !!m);
  // Кадры для шапки: широкие кадры своих фильмов, иначе постеры.
  const shots = $derived(
    (l?.items ?? [])
      .slice()
      .sort((a, b) => (b.votes ?? 0) - (a.votes ?? 0))
      .map((i) => i.card?.backdrop ?? i.card?.backdropSmall ?? poster(i))
      .filter((x): x is string => !!x)
      .slice(0, 4),
  );
  // «Что нового» - из самих фильмов: кто что добавил и что посмотрели вместе.
  const feed = $derived.by(() => {
    const ev = (l?.items ?? []).flatMap((i) => {
      const out: { key: string; at: number; text: string; who: SharedItem['addedBy']; poster: string | null }[] = [];
      if (i.addedBy) out.push({ key: `a${key(i)}`, at: i.addedAt, who: i.addedBy, poster: poster(i), text: `${i.addedBy.id === me ? 'Вы добавили' : `${i.addedBy.name} добавил(а)`} «${title(i)}»` });
      if (i.watchedAt) out.push({ key: `w${key(i)}`, at: i.watchedAt, who: null, poster: poster(i), text: `Посмотрели вместе «${title(i)}»` });
      return out;
    });
    return ev.sort((a, b) => b.at - a.at).slice(0, 5);
  });

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

<div class="pl">
  {#if error}
    <div class="empty"><Icon name="info" size={36} /> {error}</div>
  {:else if !l}
    <div class="ph-head skeleton"></div>
  {:else}
    <!-- Совместная подборка (выбор владельца 28.11): сверху кадры фильмов подборки, огромное название, участники и
         действия, справа лидер голосования; ниже вкладки «Хотим посмотреть / Посмотрели вместе», список по голосам с
         лицами проголосовавших, сбоку «Что нового» и участники. -->
    <section class="hero" aria-label="Подборка">
      <div class="bgs" class:few={shots.length < 2} aria-hidden="true">
        {#each shots as a, k (k)}<img src={artW(a, 960)} alt="" decoding="async" />{/each}
      </div>
      <div class="in">
        <div class="main">
          <a class="back" href="/my"><Icon name="back" size={16} /> Подборки</a>
          <span class="kicker">Общая подборка · {meta}</span>
          <h1>{l.name}</h1>
          <div class="who">
            <span class="avs">{#each l.members.slice(0, 5) as m (m.id)}<a href="/people/{m.id}" title={m.name}><Avatar avatar={m.avatar} photo={m.photo} size={34} /></a>{/each}</span>
            <span class="names">{l.members.map((m) => (m.id === me ? 'вы' : m.name)).join(', ')}{l.ownerName ? ` · создал(а) ${l.ownerName}` : ''}</span>
          </div>
          <div class="acts">
            <button class="btn primary" onclick={together} disabled={starting}><Icon name="play" size={16} /> Смотреть вместе</button>
            <a class="btn" href="/discover"><Icon name="plus" size={16} /> Добавить фильм</a>
            {#if want.length > 1}<button class="btn" onclick={random}>Случайный</button>{/if}
            <button class="btn round" aria-label="Ещё" aria-expanded={more} onclick={() => (more = !more)}><Icon name="dots" size={18} /></button>
          </div>
          {#if more}
            <div class="more">
              <button class="btn sm" onclick={() => ((newName = l?.name ?? ''), (renaming = true), (more = false))}><Icon name="edit" size={14} /> Переименовать</button>
              <button class="btn sm" onclick={() => (editGroup({ id: null, name: '' }), (more = false))}><Icon name="plus" size={14} /> Новая группа</button>
              <button class="btn sm danger" onclick={leave}><Icon name={l.mine ? 'trash' : 'logout'} size={14} /> {l.mine ? 'Удалить подборку' : 'Выйти'}</button>
            </div>
          {/if}
        </div>
        {#if leader && (leader.votes ?? 0) > 0}
          <a class="leader" href={href(leader)} onclick={() => leader?.ext && remember(leader.ext)}>
            {#if poster(leader)}<img src={artW(poster(leader), 185)} alt="" decoding="async" />{/if}
            <span><span class="kicker">Лидер голосования</span><b>{title(leader)}</b><small>{info(leader)}</small>
              <span class="votes"><span class="avs sm">{#each voters(leader).slice(0, 4) as m (m.id)}<Avatar avatar={m.avatar} photo={m.photo} size={24} />{/each}</span><small>{leader.votes} из {l.members.length} {plural(l.members.length, 'хочет', 'хотят', 'хотят')}</small></span>
            </span>
          </a>
        {/if}
      </div>
    </section>

    <div class="cols">
      <div class="body">
        <nav class="tabs" aria-label="Фильмы подборки">
          <button aria-current={view === 'want' ? 'page' : undefined} onclick={() => (view = 'want')}>Хотим посмотреть <em>{want.length}</em></button>
          <button aria-current={view === 'done' ? 'page' : undefined} onclick={() => (view = 'done')}>Посмотрели вместе <em>{done.length}</em></button>
        </nav>
        <div class="tools">
          <div class="chips" role="group" aria-label="Порядок">
            {#each ORDERS as [k, label] (k)}<button type="button" class="chip" aria-pressed={order === k} onclick={() => (order = k)}>{label}</button>{/each}
            {#each l.groups as g (g.id)}<button type="button" class="chip" aria-pressed={group === g.id} onclick={() => (group = group === g.id ? null : g.id)}>{g.name}</button>{/each}
          </div>
          <input class="input find" type="search" bind:value={q} placeholder="Найти в подборке" aria-label="Найти в подборке" />
        </div>
        {#if !l.items.length}
          <div class="empty">
            <Icon name="film" size={40} />
            <p>Подборка пуста. Откройте фильм в «Каталоге» и добавьте его в подборку.</p>
            <a class="btn sm" href="/discover">Открыть каталог</a>
          </div>
        {:else if !rows.length}
          <p class="faint none">{view === 'done' ? 'Вместе ещё ничего не посмотрели - отметьте фильм галочкой.' : 'Ничего не нашлось.'}</p>
        {:else}
          <ol class="list">
            {#each rows as i, n (key(i))}
              <li class="item" class:done={!!i.watchedAt}>
                <span class="pos">{n + 1}</span>
                <a class="po" href={href(i)} onclick={() => i.ext && remember(i.ext)} tabindex="-1" aria-hidden="true">{#if poster(i)}<img src={artW(poster(i), 185)} alt="" loading="lazy" decoding="async" />{/if}</a>
                <span class="ttl">
                  <a href={href(i)} onclick={() => i.ext && remember(i.ext)}><b>{title(i)}</b></a>
                  <small>{info(i)}{mins(i) ? ` · ${runtime(mins(i))}` : ''}{#if i.addedBy} · <Avatar avatar={i.addedBy.avatar} photo={i.addedBy.photo} size={18} /> {i.addedBy.id === me ? 'добавили вы' : `добавил(а) ${i.addedBy.name}`}{/if}{i.watchedAt ? ` · посмотрели ${day(i.watchedAt)}` : ''}</small>
                </span>
                <button class="vote" aria-pressed={!!i.voted} aria-label="Хочу смотреть: {i.votes ?? 0}" title="Хочу смотреть" onclick={() => vote(i)}>
                  <Icon name="heart" size={16} /> {i.votes ?? 0}
                  {#if voters(i).length}<span class="avs sm">{#each voters(i).slice(0, 3) as m (m.id)}<Avatar avatar={m.avatar} photo={m.photo} size={22} />{/each}</span>{/if}
                </button>
                <span class="tools2">
                  <button class="mini" class:on={!!i.watchedAt} aria-label={i.watchedAt ? 'Не смотрели' : 'Посмотрели вместе'} title={i.watchedAt ? 'Снять отметку' : 'Посмотрели вместе'} onclick={() => watched(i)}><Icon name="check" size={15} stroke={2.6} /></button>
                  {#if l.groups.length}
                    <Select class="move" variant="inline" label="Группа" value={String(i.groupId ?? '')} onchange={(v) => move(i, v)} options={[{ value: '', label: 'Без группы' }, ...l.groups.map((x) => ({ value: String(x.id), label: x.name }))]} />
                  {/if}
                  <button class="mini" aria-label="Убрать из подборки" title="Убрать из подборки" onclick={() => remove(i)}><Icon name="close" size={15} /></button>
                </span>
              </li>
            {/each}
          </ol>
        {/if}
      </div>

      <aside class="side">
        {#if feed.length}
          <h2>Что нового</h2>
          <ul class="feed">
            {#each feed as e (e.key)}
              <li>{#if e.who}<Avatar avatar={e.who.avatar} photo={e.who.photo} size={34} />{:else}<span class="dot"><Icon name="check" size={14} /></span>{/if}<span>{e.text}<small>{day(e.at)}</small></span>{#if e.poster}<img src={artW(e.poster, 92)} alt="" loading="lazy" />{/if}</li>
            {/each}
          </ul>
        {/if}
        <h2>Участники</h2>
        <ul class="memb">
          {#each l.members as m (m.id)}<li><a href="/people/{m.id}"><Avatar avatar={m.avatar} photo={m.photo} size={52} /><span>{m.id === me ? 'Вы' : m.name}</span></a></li>{/each}
          <li><button class="inv" onclick={openInvite}><span class="plus"><Icon name="plus" size={20} /></span><span>Позвать</span></button></li>
        </ul>
      </aside>
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
  .pl { padding: calc(var(--header) + 10px) var(--gutter) calc(56px + var(--tabbar)); max-width: 1680px; margin: 0 auto; }
  .ph-head { height: 460px; border-radius: 30px; }
  .empty { display: grid; justify-items: center; gap: 12px; padding: 48px 16px; color: var(--muted); text-align: center; }
  .faint { color: var(--faint); }
  .none { margin: 18px 0; }

  /* Шапка - кадры фильмов подборки, как герой страницы фильма. */
  .hero { position: relative; isolation: isolate; display: flex; align-items: flex-end; min-height: 460px; border-radius: 30px; overflow: hidden; background: #0b0b0c; }
  .bgs { position: absolute; inset: 0; z-index: -2; display: grid; grid-template-columns: repeat(auto-fit, minmax(0, 1fr)); }
  .bgs img { width: 100%; max-width: none; height: 100%; object-fit: cover; }
  .hero::after { content: ''; position: absolute; inset: 0; z-index: -1; background: linear-gradient(90deg, #000f 0%, #000c 38%, #0005 75%, #0008 100%), linear-gradient(0deg, #000 0%, #0000 50%); }
  .in { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 30px; align-items: end; width: 100%; padding: 32px 40px 40px; }
  .main { min-width: 0; }
  .back { display: inline-flex; align-items: center; gap: 6px; margin-bottom: 28px; color: #d1d1d6; font-size: 14px; font-weight: 600; }
  .kicker { display: block; color: #d1d1d6; font-size: 12px; font-weight: 800; letter-spacing: 0.12em; text-transform: uppercase; }
  h1 { margin: 10px 0 12px; font-size: clamp(40px, 6vw, 88px); font-weight: 900; letter-spacing: -0.045em; line-height: 0.92; overflow-wrap: anywhere; }
  .who { display: flex; align-items: center; gap: 12px; min-width: 0; color: #e5e5ea; font-weight: 600; }
  .avs { display: flex; flex: none; }
  .avs > :global(*) { border-radius: 50%; box-shadow: 0 0 0 2px #000; }
  .avs > :global(*:not(:first-child)) { margin-left: -9px; }
  .avs.sm > :global(*:not(:first-child)) { margin-left: -7px; }
  .names { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acts { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 22px; }
  .btn.round { width: 48px; padding: 0; justify-content: center; }
  .more { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 12px; }
  .danger { color: #ff6961; }
  .leader { display: grid; grid-template-columns: 92px minmax(0, 1fr); gap: 14px; align-items: center; width: 330px; padding: 16px; border-radius: 24px; background: #0009; color: inherit; transition: transform 0.3s var(--ease); }
  .leader:hover { transform: translateY(-3px); }
  .leader img { width: 92px; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 12px; }
  .leader b { display: block; margin: 4px 0; font-size: 18px; font-weight: 800; }
  .leader small { color: var(--muted); }
  .votes { display: flex; align-items: center; gap: 10px; margin-top: 10px; }

  .cols { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 36px; align-items: start; margin-top: 34px; }
  .body { min-width: 0; }
  .tabs { display: flex; gap: 28px; margin-bottom: 14px; border-bottom: 1px solid var(--line); overflow-x: auto; scrollbar-width: none; }
  .tabs button { position: relative; flex: none; padding: 0 0 12px; border: 0; background: none; color: #6e6e73; font: inherit; font-size: 17px; font-weight: 700; white-space: nowrap; cursor: pointer; }
  .tabs button[aria-current='page'] { color: #fff; }
  .tabs button[aria-current='page']::after { content: ''; position: absolute; left: 0; right: 0; bottom: -1px; height: 3px; border-radius: 3px; background: #fff; }
  .tabs em { margin-left: 6px; color: #6e6e73; font-style: normal; }
  .tools { display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap; margin-bottom: 10px; }
  .chips { display: flex; gap: 8px; flex-wrap: wrap; }
  .chip { height: 40px; padding: 0 18px; border: 0; border-radius: 99px; background: #ffffff12; color: #d1d1d6; font: inherit; font-size: 14px; font-weight: 700; cursor: pointer; }
  .chip[aria-pressed='true'] { background: #fff; color: #000; }
  .find { width: 240px; min-height: 40px; height: 40px; border-radius: 99px; }
  .list { margin: 0; padding: 0; list-style: none; }
  .item { display: grid; grid-template-columns: 26px 64px minmax(0, 1fr) auto auto; gap: 18px; align-items: center; padding: 12px 10px; border-radius: 20px; transition: background 0.2s; }
  .item:hover { background: #ffffff0a; }
  .item.done { opacity: 0.6; }
  .pos { color: #6e6e73; font-size: 18px; font-weight: 900; text-align: center; font-variant-numeric: tabular-nums; }
  .po img { width: 64px; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 10px; }
  .ttl { min-width: 0; }
  .ttl b { display: block; font-size: 16.5px; font-weight: 800; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ttl small { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; color: var(--muted); font-size: 13px; }
  .vote { display: inline-flex; align-items: center; gap: 8px; height: 40px; padding: 0 14px 0 12px; border: 0; border-radius: 99px; background: #ffffff12; color: inherit; font: inherit; font-weight: 800; font-variant-numeric: tabular-nums; cursor: pointer; transition: background 0.2s, color 0.2s; }
  .vote[aria-pressed='true'] { background: #fff; color: #000; }
  .vote .avs > :global(*) { box-shadow: 0 0 0 2px #1c1c1e; }
  .vote[aria-pressed='true'] .avs > :global(*) { box-shadow: 0 0 0 2px #fff; }
  .tools2 { display: flex; align-items: center; gap: 4px; }
  .mini { display: grid; place-items: center; width: 34px; height: 34px; border: 0; border-radius: 50%; background: none; color: var(--muted); cursor: pointer; transition: background 0.2s, color 0.2s; }
  .mini:hover { background: #ffffff14; color: #fff; }
  .mini.on { background: #fff; color: #000; }

  .side h2 { margin: 0 0 14px; font-size: 18px; font-weight: 800; }
  .feed { display: grid; gap: 16px; margin: 0 0 34px; padding: 0; list-style: none; }
  .feed li { display: grid; grid-template-columns: 34px minmax(0, 1fr) 40px; gap: 12px; align-items: center; font-size: 14px; }
  .feed small { display: block; color: #6e6e73; font-size: 12px; }
  .feed img { width: 40px; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 6px; }
  .dot { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 50%; background: #ffffff14; }
  .memb { display: flex; flex-wrap: wrap; gap: 14px; margin: 0; padding: 0; list-style: none; }
  .memb a, .inv { display: grid; justify-items: center; gap: 6px; width: 60px; padding: 0; border: 0; background: none; color: inherit; font: inherit; font-size: 12px; font-weight: 700; text-align: center; cursor: pointer; }
  .memb span { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .plus { display: grid; place-items: center; width: 52px; height: 52px; border: 2px dashed #ffffff33; border-radius: 50%; color: var(--muted); }

  .rename { display: flex; gap: 10px; }
  .rename .input { flex: 1; min-width: 0; }
  .pick { display: grid; gap: 4px; margin: 0; padding: 0; list-style: none; }
  .pick button { display: flex; align-items: center; gap: 12px; width: 100%; padding: 8px; border: 0; border-radius: 12px; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .pick button:hover { background: #ffffff12; }

  @media (max-width: 1000px) {
    .cols { grid-template-columns: minmax(0, 1fr); }
    .in { grid-template-columns: minmax(0, 1fr); padding: 22px 22px 26px; }
    .leader { width: 100%; }
  }
  @media (max-width: 700px) {
    .pl { padding-top: calc(env(safe-area-inset-top) + 12px); }
    .hero { min-height: 520px; margin-inline: calc(var(--gutter) * -1); border-radius: 0 0 28px 28px; }
    .bgs { grid-template-columns: 1fr 1fr; }
    .item { grid-template-columns: 52px minmax(0, 1fr) auto; gap: 12px; padding: 10px 4px; }
    .pos, .vote .avs { display: none; }
    .po img { width: 52px; }
    .tools2 { grid-column: 2 / -1; justify-content: flex-end; }
    .find { flex: 1; width: auto; min-width: 0; }
  }
</style>
