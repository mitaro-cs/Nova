<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api, errorText, post } from '$lib/api';
  import PlayerShell from '$lib/player/PlayerShell.svelte';
  import type { PlayerSync } from '$lib/player/sync';
  import { session } from '$lib/session.svelte';
  import { clock } from '$lib/fmt';
  import type { RoomChat, RoomEvent, RoomItem, RoomMark, RoomState, Who } from '$lib/types';
  import EmojiPicker from '$lib/room/EmojiPicker.svelte';
  import { Voice } from '$lib/room/voice.svelte';
  import Avatar from '$lib/ui/Avatar.svelte';
  import Icon from '$lib/ui/Icon.svelte';

  // Смотреть вместе: у всех в комнате одно место в видео - пауза, перемотка и следующая серия у
  // всех сразу. Сбоку - кто в комнате и чат, внизу - реакции, летящие вверх по экрану. Связь -
  // поток событий сервера (EventSource), действия - обычные запросы.
  const id = $derived(page.params.id ?? '');
  // Комната «Зал» (23.11): у каждого свои смайлики (профиль, «Мои реакции»), реакция всплывает с аватаром и
  // остаётся отметкой на полосе времени; зрители сидят внизу кадра (кто говорит - светится); голоса «что дальше» в
  // очереди; время в чате («0:41:20») - ссылка на момент.
  const DEFAULT_REACTIONS = ['❤️', '😂', '😮', '😢', '🔥', '👏'];
  const myReactions = $derived(session.me?.reactions?.length ? session.me.reactions : DEFAULT_REACTIONS);
  let picking = $state(false);
  let marks = $state<RoomMark[]>([]);

  let room = $state<RoomState | null>(null);
  let receivedAt = 0;
  let titleId = $state(0);
  let members = $state<Who[]>([]);
  let chat = $state<RoomChat[]>([]);
  let error = $state('');
  // На телефоне чат сначала закрыт: кадр важнее (26.11 «всё ужато»); открывается кнопкой справа сверху.
  let side = $state(typeof matchMedia === 'undefined' || !matchMedia('(max-height: 500px), (max-width: 640px)').matches);
  // Очередь просмотра: порядок задаёт создатель, остальные предлагают. Вкладка боковой панели - «Очередь» или «Чат».
  let queue = $state<RoomItem[]>([]);
  let current = $state<number | null>(null);
  let ownerId = $state(0);
  let tab = $state<'queue' | 'chat'>('chat');
  let newUrl = $state('');
  let adding = $state(false);
  let connection = $state('connecting');
  let text = $state('');
  let inviting = $state(false);
  let friends = $state<Who[] | null>(null);
  let invited = $state<number[]>([]);
  let flying = $state<{ key: number; emoji: string; x: number; who: Who }[]>([]);
  let feed = $state<HTMLElement>();
  // Видео по ссылке (фильм из Обзора): поток через сервер - узнаём один раз.
  let linkSrc = $state<{ url: string; hls: boolean; dash?: boolean; title: string; linkId: number; start: number } | null>(null);
  $effect(() => {
    const l = room?.linkId;
    if (!l || linkSrc?.linkId === l) return;
    Promise.all([api<{ url: string; hls: boolean; dash?: boolean }>(`/api/links/${l}/stream/source`), api<{ label: string; display?: string }>(`/api/links/${l}`)]).then(
      ([s, link]) => (linkSrc = { ...s, title: link.display || link.label, linkId: l, start: room ? expected(room).position : 0 }),
      (x) => (error = errorText(x)),
    );
  });
  // Голосовой чат - заводится, когда пришёл снимок комнаты.
  let voice = $state<Voice | null>(null);

  const listeners = new Set<(s: { playing: boolean; position: number }) => void>();
  /** Где сейчас должно быть видео: при «смотрим» место идёт дальше с момента события. */
  const expected = (s: RoomState) => ({
    playing: s.playing,
    position: s.playing ? s.position + (Date.now() - receivedAt) / 1000 : s.position,
  });

  let acceptedRevision = -1;
  let sending = false;
  let queued: {fileId:number;playing:boolean;position:number} | null = null;
  let deferred: RoomState | null = null;
  function setState(s: RoomState) {
    if((s.revision??0) < acceptedRevision)return;
    acceptedRevision=s.revision??0;
    room = s;
    receivedAt = Date.now();
    const e = expected(s);
    listeners.forEach((l) => l(e));
  }

  let key = 0;
  function fly(emoji: string, who: Who) {
    const k = ++key;
    flying = [...flying, { key: k, emoji, x: 10 + Math.random() * 70, who }];
    setTimeout(() => (flying = flying.filter((f) => f.key !== k)), 2600);
  }

  function scrollChat() {
    requestAnimationFrame(() => feed?.scrollTo({ top: feed.scrollHeight, behavior: 'smooth' }));
  }

  $effect(() => {
    const room_ = id;
    error = '';
    const es = new EventSource(`/api/rooms/${room_}/events`);
    es.onmessage = (m) => {
      connection = 'online';
      const e = JSON.parse(m.data) as RoomEvent;
      if (e.type === 'snapshot') {
        const me = session.me?.user.profile?.id;
        if (me && !voice) voice = new Voice(room_, me);
        voice?.setIds(e.voice ?? []);
        titleId = e.titleId;
        ownerId = e.ownerId;
        queue = e.queue ?? [];
        current = e.current ?? null;
        members = e.members;
        chat = e.chat;
        marks = e.marks ?? [];
        setState(e.state);
        scrollChat();
      } else if (e.type === 'state') {if(sending)deferred=e.state;else setState(e.state);}
      else if (e.type === 'members') members = e.members;
      else if (e.type === 'queue') {
        queue = e.queue;
        current = e.current;
      } else if (e.type === 'now') {
        marks = [];
        titleId = e.titleId;
        current = e.current;
        acceptedRevision = -1;
        setState(e.state);
      }
      else if (e.type === 'chat') {
        chat = [...chat.slice(-59), { from: e.from, body: e.body, at: e.at }];
        scrollChat();
      } else if (e.type === 'react') {
        fly(e.emoji, e.from);
        if (e.position != null) marks = [...marks.slice(-299), { position: e.position, emoji: e.emoji, from: e.from.id }];
      }
      else if (e.type === 'voice') voice?.setIds(e.ids);
      else if (e.type === 'signal' && e.to === session.me?.user.profile?.id) voice?.signal(e.from, e.data);
    };
    // Обрыв: EventSource переподключается сам; комнаты больше нет - сказать и не стучаться.
    es.onerror = () => {
      connection = 'reconnecting';
      api('/api/rooms/'+room_).catch((x) => {
        if (x?.status === 404 || x?.status === 401 || x?.status === 403) { error=errorText(x);es.close(); }
      });
    };
    return () => {
      es.close();
      voice?.leave();
      voice = null;
    };
  });

  // Раз в 5 с, пока смотрим, - сверить место (плеер перематывает только при заметном разрыве).
  $effect(() => {
    const t = setInterval(() => {
      if (room?.playing && !sending) listeners.forEach((l) => l(expected(room!)));
    }, 5000);
    return () => clearInterval(t);
  });

  async function flushState() {
    if(sending)return;sending=true;
    try {
      while(queued){const command=queued;queued=null;
        try {const state=await post<RoomState>(`/api/rooms/${id}/state`,command);if(!queued){const latest=deferred && (deferred.revision??0)>(state.revision??0)?deferred:state;setState(latest);deferred=null;}}
        catch {connection='reconnecting';}
      }
    } finally {sending=false;}
  }

  const sync: PlayerSync = {
    local(s) {
      if (!room) return;
      const next: RoomState = { ...room, fileId: s.fileId ?? room.fileId, playing: s.playing, position: s.position, at: Date.now() };
      room = next;
      receivedAt = Date.now();
      queued={fileId:next.fileId,playing:s.playing,position:s.position};
      void flushState();
    },
    listen(apply) {
      listeners.add(apply);
      if (room) apply(expected(room));
      return () => listeners.delete(apply);
    },
    back() {
      goto(titleId ? `/title/${titleId}` : '/');
    },
  };

  async function say(e: SubmitEvent) {
    e.preventDefault();
    const body = text.trim();
    if (!body) return;
    text = '';
    await post(`/api/rooms/${id}/chat`, { body }).catch((x) => (error = errorText(x)));
  }

  function react(emoji: string) {
    post(`/api/rooms/${id}/react`, { emoji, position: room ? expected(room).position : null }).catch(() => {});
  }
  // Время в чате: «0:41:20» или «41:20» - ссылка на момент (перематывает всех, как обычная перемотка).
  const TIME = /\b(\d{1,2}:\d{2}(?::\d{2})?)\b/g;
  function parts(body: string): { text: string; at?: number }[] {
    const out: { text: string; at?: number }[] = [];
    let last = 0;
    for (const m of body.matchAll(TIME)) {
      const sec = m[1].split(':').reduce((a, n) => a * 60 + Number(n), 0);
      if (m.index! > last) out.push({ text: body.slice(last, m.index) });
      out.push({ text: m[1], at: sec });
      last = m.index! + m[1].length;
    }
    if (last < body.length) out.push({ text: body.slice(last) });
    return out;
  }
  function seekTo(at: number) {
    if (!room) return;
    sync.local({ fileId: room.fileId, playing: room.playing, position: at });
    listeners.forEach((l) => l(expected(room!)));
  }
  function stamp() {
    if (!room) return;
    text = `${clock(expected(room).position)} ${text}`.trimEnd() + ' ';
  }

  async function openInvite() {
    inviting = !inviting;
    if (!inviting || friends) return;
    const r = await api<{ items: { person: Who }[] }>('/api/chats').catch(() => ({ items: [] }));
    friends = r.items.map((i) => i.person);
  }

  async function invite(p: Who) {
    try {
      await post(`/api/rooms/${id}/invite`, { profileId: p.id });
      invited = [...invited, p.id];
    } catch (x) {
      error = errorText(x);
    }
  }

  let copied = $state(false);
  let showRoomLink = $state(false);
  // Выйти из комнаты и смотреть одному - с того же места (место сохраняем, чтобы плеер начал отсюда).
  let leaving = $state(false);
  async function alone() {
    if (!room || leaving) return;
    leaving = true;
    const v = document.querySelector<HTMLVideoElement>('.player video');
    const position = v && isFinite(v.currentTime) ? v.currentTime : expected(room).position;
    const duration = v && isFinite(v.duration) && v.duration > 0 ? v.duration : 0;
    const url = room.linkId ? `/api/links/${room.linkId}/progress` : `/api/progress/${room.fileId}`;
    if (duration && position > 5) await api(url, { method: 'PUT', json: { position, duration } }).catch(() => {});
    goto(room.linkId ? `/watch/link/${room.linkId}` : `/watch/${room.fileId}`, { replace: true });
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(location.href);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      showRoomLink = true;
    }
  }
  const me = $derived(session.me?.user.profile?.id);
  const owner = $derived(me != null && me === ownerId);
  const ready = $derived(queue.filter((q) => q.approved));
  const asks = $derived(queue.filter((q) => !q.approved));

  async function act(path: string, method: 'POST' | 'PUT' | 'DELETE' = 'POST', json: unknown = {}) {
    try {
      await api(`/api/rooms/${id}/queue${path}`, { method, json });
    } catch (x) {
      error = errorText(x);
    }
  }
  /** Ссылка: сначала проверка сервером (Rutube, VK, YouTube, прямой файл, сайты хоста), потом - в очередь. */
  async function addLink(e: SubmitEvent) {
    e.preventDefault();
    const url = newUrl.trim();
    if (!url || adding) return;
    adding = true;
    error = '';
    try {
      const l = await post<{ id: number }>('/api/links', { url });
      await post(`/api/rooms/${id}/queue`, { linkId: l.id });
      newUrl = '';
    } catch (x) {
      error = errorText(x);
    } finally {
      adding = false;
    }
  }
  const move = (i: number, d: -1 | 1) => {
    const ids = ready.map((q) => q.id);
    const j = i + d;
    if (j < 0 || j >= ids.length) return;
    [ids[i], ids[j]] = [ids[j], ids[i]];
    void act('', 'PUT', { ids });
  };
</script>

{#if error && !room}
  <div class="page empty">
    <Icon name="party" size={40} />
    <p>{error}</p>
    <a class="btn" href="/">На главную</a>
  </div>
{:else if room && !room.fileId && !room.linkId}
  <!-- Пустая комната (21.11): сначала комната, видео потом. Первое видео создателя включается у всех. -->
  <main class="emptyroom">
    <section class="er-card" aria-labelledby="er-h">
      <span class="er-ic"><Icon name="party" size={26} /></span>
      <h1 id="er-h">Комната готова</h1>
      <p class="muted">{owner ? 'Позовите друзей и добавьте, что смотреть: видео пойдёт у всех одновременно, есть чат и голос.' : 'Ждём, когда создатель комнаты включит видео. Можно предложить своё.'}</p>
      <div class="er-row">
        <button class="btn" type="button" onclick={copy}><Icon name={copied ? 'check' : 'link'} size={15} /> {copied ? 'Ссылка скопирована' : 'Скопировать ссылку'}</button>
      </div>
      {#if showRoomLink}<label class="room-link">Ссылка для приглашения <input class="input" readonly value={location.href} onclick={(e) => e.currentTarget.select()} /></label>{/if}
      <form class="er-add" onsubmit={addLink}>
        <label for="er-url">{owner ? 'Добавить видео' : 'Предложить видео'}</label>
        <div class="er-in">
          <input id="er-url" class="input" bind:value={newUrl} placeholder="Ссылка: Rutube, VK, YouTube, файл .mp4 или .m3u8" maxlength="500" autocomplete="off" />
          <button class="btn primary" disabled={adding || !newUrl.trim()}>{adding ? 'Проверяем…' : owner ? 'Включить' : 'Предложить'}</button>
        </div>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        <small class="muted">Фильм с Nova - откройте его и нажмите «Смотреть вместе» в плеере.</small>
      </form>
      {#if members.length}
        <ul class="er-who" aria-label="В комнате">{#each members as m (m.id)}<li>{m.name}</li>{/each}</ul>
      {/if}
    </section>
  </main>
{:else if room}
  {#key `${id}:${room.linkId ?? room.fileId}`}
    {#if room.linkId && (!linkSrc || linkSrc.linkId !== room.linkId)}
      <div class="boot" aria-busy="true"><Icon name="party" size={36} /> Готовим видео с сайта…</div>
    {:else}
    <PlayerShell fileId={room.fileId} src={room.linkId ? linkSrc : null} {sync} sidebar={side} {marks}>
      <div class="room" class:closed={!side}>
        <div class="room-status" role="status"><Icon name="party" size={14}/><span>Смотрим вместе</span><i class:online={connection === 'online'}></i><small>{error || (connection === 'online' ? `${members.length} участн.` : 'Восстанавливаем связь…')}</small></div>
        <button class="toggle" aria-label={side ? 'Спрятать чат' : 'Показать чат'} aria-expanded={side} onclick={() => (side = !side)}>
          <Icon name={side ? 'right' : 'mail'} size={18} />
          {#if !side && members.length > 1}<i>{members.length}</i>{/if}
        </button>
        {#if side}
          <aside class="side" aria-label="Смотрим вместе">
            <header>
              <b><Icon name="party" size={16} /> Смотрим вместе</b>
              <span class="faces">
                {#each members.slice(0,4) as m, i (m.id)}
                  <span class="face" class:talk={voice?.speaking[m.id]} class:mic={voice?.ids.includes(m.id)} style:--i={i} title="{m.name}{voice?.ids.includes(m.id) ? ' · в голосовом чате' : ''}"><Avatar avatar={m.avatar} photo={m.photo} size={26} /></span>
                {/each}
                {#if members.length>4}<span class="extra-members">+{members.length-4}</span>{/if}
              </span>
            </header>
            <div class="tools">
              {#if voice?.on}
                <button class="btn sm" class:off={voice.muted} aria-pressed={!voice.muted} onclick={() => voice?.toggleMute()} title={voice.muted ? 'Включить микрофон' : 'Выключить микрофон'}>
                  <Icon name={voice.muted ? 'mute' : 'audio'} size={14} /> {voice.muted ? 'Микрофон выкл.' : 'Говорю'}
                </button>
                <button class="btn sm ghost" onclick={() => voice?.leave()}><Icon name="close" size={14} /> Выйти из голоса</button>
              {:else}
                <button class="btn sm white" disabled={!voice || voice.busy} onclick={() => voice?.join()}><Icon name="audio" size={14} /> Голос{voice?.ids.length ? ` · ${voice.ids.length}` : ''}</button>
              {/if}
              <button class="btn sm" aria-expanded={inviting} onclick={openInvite}><Icon name="plus" size={14} /> Позвать</button>
              <button class="btn sm ghost" disabled={leaving} onclick={alone}><Icon name="user" size={14} /> Смотреть одному</button>
              <button class="btn sm ghost" onclick={copy}><Icon name={copied ? 'check' : 'link'} size={14} /> {copied ? 'Скопировано' : 'Ссылка'}</button>
            </div>
            {#if showRoomLink}<label class="room-link">Ссылка для приглашения <input class="input" readonly value={location.href} onclick={(e) => e.currentTarget.select()} /></label>{/if}
            {#if inviting}
              <ul class="invite">
                {#if friends === null}
                  <li class="muted">Загружаем друзей…</li>
                {:else if !friends.length}
                  <li class="muted">Друзей пока нет - отправьте ссылку.</li>
                {/if}
                {#each friends ?? [] as f (f.id)}
                  <li>
                    <Avatar avatar={f.avatar} photo={f.photo} size={24} />
                    <span>{f.name}</span>
                    <button class="btn sm" disabled={invited.includes(f.id) || members.some((m) => m.id === f.id)} onclick={() => invite(f)}>
                      {members.some((m) => m.id === f.id) ? 'Тут' : invited.includes(f.id) ? 'Позвали' : 'Позвать'}
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
            <div class="tabs2" role="tablist" aria-label="Панель комнаты">
              <button role="tab" aria-selected={tab === 'queue'} class:on={tab === 'queue'} onclick={() => (tab = 'queue')}>Очередь{#if queue.length}<sup>{queue.length}</sup>{/if}</button>
              <button role="tab" aria-selected={tab === 'chat'} class:on={tab === 'chat'} onclick={() => (tab = 'chat')}>Чат</button>
            </div>
            {#if tab === 'queue'}
              <p class="who">{owner ? 'Вы создатель: порядок задаёте вы, остальные могут предложить' : 'Порядок задаёт создатель комнаты; вы можете предложить видео'}</p>
              <ol class="queue" aria-label="Очередь просмотра">
                {#each ready as q, i (q.id)}
                  <li class:now={q.id === current}>
                    <span class="src">{q.source}</span>
                    <span class="nm"><b>{q.label}</b><small>{q.id === current ? 'играет · ' : ''}{q.byName}</small></span>
                    {#if q.id !== current}<button class="vote" class:on={!!me && !!q.votes?.includes(me)} aria-pressed={!!me && !!q.votes?.includes(me)} aria-label="Голос за «{q.label}»" onclick={() => act(`/${q.id}/vote`)}>▲ {q.votes?.length ?? 0}</button>{/if}
                    {#if owner}
                      <button class="sq" aria-label="Выше" disabled={i === 0} onclick={() => move(i, -1)}>↑</button>
                      <button class="sq" aria-label="Ниже" disabled={i === ready.length - 1} onclick={() => move(i, 1)}>↓</button>
                      {#if q.id !== current}<button class="sq" aria-label="Включить сейчас" onclick={() => act(`/${q.id}/play`)}><Icon name="play" size={12} /></button>{/if}
                      <button class="sq" aria-label="Убрать из очереди" onclick={() => act(`/${q.id}`, 'DELETE')}><Icon name="close" size={12} /></button>
                    {/if}
                  </li>
                {/each}
                {#each asks as q (q.id)}
                  <li class="ask">
                    <span class="src">{q.source}</span>
                    <span class="nm"><b>{q.label}</b><small>предложил(а) {q.byName}</small></span>
                    {#if owner}<button class="btn sm" onclick={() => act(`/${q.id}/approve`)}>Принять</button>{/if}
                    {#if owner || q.by === me}<button class="sq" aria-label="Убрать предложение" onclick={() => act(`/${q.id}`, 'DELETE')}><Icon name="close" size={12} /></button>{/if}
                  </li>
                {/each}
                {#if !queue.length}<li class="muted">Очередь пуста</li>{/if}
              </ol>
              {#if owner && ready.length > 1}<button class="btn sm" onclick={() => act('/next')}>Следующее</button>{/if}
              <form class="addq" onsubmit={addLink}>
                <input class="input" bind:value={newUrl} placeholder="Ссылка на видео (Rutube, VK, YouTube, файл)" aria-label="Ссылка на видео для очереди" maxlength="500" />
                <button class="btn sm primary" disabled={!newUrl.trim() || adding}>{adding ? '…' : owner ? 'Добавить' : 'Предложить'}</button>
              </form>
              <p class="safe">Ссылки проверяет сервер: чужие страницы внутри Nova не открываются</p>
              {#if error}<p class="error">{error}</p>{/if}
            {:else}
            <ol class="feed" bind:this={feed} aria-live="polite">
              {#if room.by}<li class="sys">Последним управлял(а) {room.by}</li>{/if}
              {#each chat as m, i (i)}
                <li class="msg" class:mine={m.from.id === me}>
                  <Avatar avatar={m.from.avatar} photo={m.from.photo} size={22} />
                  <span><b>{m.from.name}</b> {#each parts(m.body) as p, k (k)}{#if p.at != null}<button class="ts" onclick={() => seekTo(p.at!)} aria-label="Перейти к {p.text}">{p.text}</button>{:else}{p.text}{/if}{/each}</span>
                </li>
              {/each}
            </ol>
            {#if voice?.error}<p class="error">{voice.error}</p>{/if}
            {#if error}<p class="error">{error}</p>{/if}
            <form class="say" onsubmit={say}>
              <button class="btn sm icon ghost" type="button" aria-label="Отметить момент" title="Вставить текущее время - его можно нажать" onclick={stamp}><Icon name="clock" size={15} /></button>
              <input class="input" bind:value={text} maxlength="500" placeholder="Сообщение…" aria-label="Сообщение в комнату" />
              <button class="btn sm icon" aria-label="Отправить" disabled={!text.trim()}><Icon name="send" size={16} /></button>
            </form>
            {/if}
          </aside>
        {/if}
        <!-- Зал: кто смотрит - внизу кадра; кто говорит в голосовом чате - светится. -->
        {#if members.length > 1}
          <ul class="seats" aria-label="Зрители в зале">
            {#each members.slice(0, 8) as m (m.id)}
              <li class:talk={voice?.speaking[m.id]} class:mic={voice?.ids.includes(m.id)} title="{m.name}{voice?.ids.includes(m.id) ? ' · в голосовом чате' : ''}"><Avatar avatar={m.avatar} photo={m.photo} size={34} /><small>{m.name}</small></li>
            {/each}
          </ul>
        {/if}
        <div class="reacts" role="group" aria-label="Реакции">
          {#each myReactions as r (r)}<button onclick={() => react(r)} aria-label="Реакция {r}">{r}</button>{/each}
          <button class="edit" onclick={() => (picking = true)} aria-label="Мои реакции" title="Выбрать свои смайлики"><Icon name="pencil" size={14} /></button>
        </div>
        {#each flying as f (f.key)}
          <span class="fly" style:left="{f.x}%" aria-hidden="true"><span class="fa"><Avatar avatar={f.who.avatar} photo={f.who.photo} size={22} /><em>{f.emoji}</em></span><small>{f.who.name}</small></span>
        {/each}
      </div>
    </PlayerShell>
    {/if}
  {/key}
{:else}
  <div class="boot" aria-busy="true"><Icon name="party" size={36} /> Входим в комнату…</div>
{/if}
<EmojiPicker bind:open={picking} />

<style>
  /* Зал: зрители внизу кадра, свои смайлики, голоса и время в чате. */
  .seats { position: absolute; left: 50%; bottom: calc(var(--bottom-h, 120px) + 64px); z-index: 4; display: flex; gap: 10px; margin: 0; padding: 0; list-style: none; transform: translateX(-50%); pointer-events: none; transition: opacity 0.2s; }
  .seats li { display: grid; justify-items: center; gap: 3px; }
  .seats li :global(img), .seats li :global(.avatar) { border-radius: 50%; box-shadow: 0 0 0 2px #000; transition: box-shadow 0.2s; }
  .seats li.mic :global(img), .seats li.mic :global(.avatar) { box-shadow: 0 0 0 2px #000, 0 0 0 3px #ffffff66; }
  .seats li.talk :global(img), .seats li.talk :global(.avatar) { box-shadow: 0 0 0 2px #000, 0 0 0 4px #fff; }
  .seats small { max-width: 72px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #ffffffcc; font-size: 11px; text-shadow: 0 1px 3px #000; }
  :global(.player.idle) .seats { opacity: 0; }
  .reacts .edit { display: grid; place-items: center; color: #c9c9cf; }
  .fa { display: flex; align-items: center; gap: 4px; padding: 3px 8px 3px 3px; border-radius: 999px; background: #00000080; }
  .fa :global(img), .fa :global(.avatar) { border-radius: 50%; }
  .vote { flex: none; padding: 4px 8px; border: 1px solid #ffffff26; border-radius: 999px; background: none; color: #c9c9cf; font: inherit; font-size: 11px; font-weight: 700; cursor: pointer; font-variant-numeric: tabular-nums; }
  .vote.on { background: #fff; color: #000; border-color: #fff; }
  .ts { display: inline; padding: 0 6px; border: 0; border-radius: 999px; background: #fff; color: #000; font: inherit; font-size: 11.5px; font-weight: 700; cursor: pointer; font-variant-numeric: tabular-nums; }
  .boot {
    min-height: 100dvh;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 12px;
    color: var(--muted);
    animation: pulse 1.6s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.5;
    }
  }
  /* Слой комнаты поверх видео: сам не ловит касания, только свои кнопки и панель. */
  .room {
    position: absolute;
    inset: 0;
    z-index: 6;
    pointer-events: none;
  }
  .room > :global(*:not(.fly)) {
    pointer-events: auto;
  }
  .toggle {
    position: absolute;
    top: 76px;
    right: 16px;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border: 0;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
    cursor: pointer;
    transition: transform 0.2s cubic-bezier(0.2, 0.9, 0.3, 1.5);
  }
  .toggle:active {
    transform: scale(0.88);
  }
  .toggle i {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    border-radius: 99px;
    background: var(--ok);
    color: #000;
    font-size: 11px;
    font-style: normal;
    font-weight: 800;
    line-height: 18px;
  }
  .side {
    position: absolute;
    top: 124px;
    right: 16px;
    bottom: 120px;
    width: min(320px, calc(100vw - 32px));
    /* Колонка, а не сетка из шести строк: блоки появляются и пропадают (ссылка, «Позвать», очередь или чат), и
       в сетке переключатель «Очередь/Чат» попадал в растягиваемую строку и раздувался в огромные овалы. */
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-radius: 18px;
    background: rgba(14, 14, 16, 0.86);
    box-shadow:
      0 0 0 1px rgba(255, 255, 255, 0.08),
      0 30px 60px -20px rgba(0, 0, 0, 0.8);
    animation: slide 0.35s cubic-bezier(0.2, 0.9, 0.3, 1.15);
  }
  @keyframes slide {
    from {
      opacity: 0;
      transform: translateX(24px);
    }
  }
  .side header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .side header b {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
  }
  .faces {
    display: flex;
  }
  .face {
    display: block;
    border-radius: 50%;
    box-shadow: 0 0 0 2px #0e0e10;
    animation: pop 0.4s cubic-bezier(0.2, 0.9, 0.3, 1.6) both;
    animation-delay: calc(var(--i) * 60ms);
  }
  .face:not(:first-child) {
    margin-left: -7px;
  }
  /* В голосовом чате - зелёная точка, говорит - зелёное кольцо пульсирует. */
  .face {
    position: relative;
    transition: transform 0.2s;
  }
  .face.mic::after {
    content: '';
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
    box-shadow: 0 0 0 2px #0e0e10;
  }
  .face.talk {
    z-index: 1;
    box-shadow:
      0 0 0 2px #0e0e10,
      0 0 0 4px var(--ok);
    transform: scale(1.12);
  }
  .tools {
    flex-wrap: wrap;
  }
  .tools .off {
    color: var(--danger);
  }
  @keyframes pop {
    from {
      transform: scale(0);
    }
  }
  .tools {
    display: flex;
    gap: 6px;
  }
  .invite {
    list-style: none;
    margin: 0;
    padding: 6px;
    max-height: 180px;
    overflow-y: auto;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.06);
    display: grid;
    gap: 4px;
  }
  .invite li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .invite li span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .feed {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 8px;
    font-size: 13.5px;
  }
  .sys {
    color: var(--faint);
    font-size: 12px;
    text-align: center;
  }
  .msg {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    animation: in 0.3s cubic-bezier(0.2, 0.9, 0.3, 1.2);
  }
  .msg span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .msg b {
    font-weight: 700;
    margin-right: 4px;
  }
  .msg.mine b {
    color: var(--ok);
  }
  @keyframes in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .say {
    display: flex;
    gap: 6px;
  }
  .say .input {
    height: 36px;
    min-width: 0;
  }
  .error {
    margin: 0;
    font-size: 12.5px;
  }
  /* Реакции - у нижнего края, над управлением плеера. */
  .reacts {
    position: absolute;
    left: 50%;
    bottom: 96px;
    display: flex;
    gap: 4px;
    padding: 5px;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.55);
    transform: translateX(-50%);
  }
  .reacts button {
    width: 36px;
    height: 36px;
    border: 0;
    border-radius: 50%;
    background: none;
    font-size: 20px;
    cursor: pointer;
    transition: transform 0.2s cubic-bezier(0.2, 0.9, 0.3, 1.6);
  }
  .reacts button:hover {
    transform: translateY(-4px) scale(1.2);
  }
  .reacts button:active {
    transform: scale(0.85);
  }
  .fly {
    position: absolute;
    bottom: 140px;
    display: grid;
    justify-items: center;
    pointer-events: none;
    animation: up 2.6s cubic-bezier(0.2, 0.7, 0.3, 1) forwards;
  }
  .fly em {
    font-size: 40px;
    font-style: normal;
    line-height: 1;
  }
  .fly small {
    padding: 1px 8px;
    border-radius: 99px;
    background: rgba(0, 0, 0, 0.6);
    font-size: 11px;
    color: #fff;
  }
  @keyframes up {
    0% {
      opacity: 0;
      transform: translateY(20px) scale(0.6);
    }
    12% {
      opacity: 1;
      transform: translateY(0) scale(1.15);
    }
    100% {
      opacity: 0;
      transform: translateY(-46vh) scale(0.9) rotate(-12deg);
    }
  }
  /* В плеере управление прячется - комната тоже уходит из-под пальца, кроме реакций. */
  :global(.player.idle) .room .toggle,
  :global(.player.idle) .reacts {
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
    transition: opacity 0.4s;
  }
  @media (max-width: 640px) {
    .side {
      top: auto;
      bottom: 150px;
      height: 46vh;
    }
    .toggle {
      top: 70px;
    }
    .reacts {
      bottom: 92px;
      max-width: calc(100vw - 24px);
      overflow-x: auto;
    }
  }

  .side { background: linear-gradient(145deg, #1c2029fa, #11141afa); border: 1px solid #bbc9eb33; border-radius: 22px; padding: 18px; width: min(360px, calc(100vw - 32px)); }
  .side header { padding-bottom: 10px; border-bottom: 1px solid var(--line); }
  .toggle { border: 1px solid var(--line); background: #1c2029; }

  .room-status {position:absolute;top:84px;left:clamp(14px,3vw,36px);display:flex;align-items:center;gap:8px;padding:8px 12px;border:1px solid #ffffff12;border-radius:999px;background:#101012b8;backdrop-filter:blur(12px);color:#d7d7dc;font-size:12px;pointer-events:none;}
  .room-status small {color:#a5a5ad}.room-status i {width:6px;height:6px;border-radius:50%;background:#b9994f}.room-status i.online {background:#46c992}
  .side {top:132px;bottom:132px;border:1px solid #ffffff12;background:#111114ed;backdrop-filter:blur(18px);border-radius:24px;box-shadow:0 18px 70px #0008;animation-duration:140ms;}
  .reacts {padding:5px 10px;gap:2px;border:1px solid #ffffff10;background:#111114b8;backdrop-filter:blur(12px);border-radius:999px;transition:opacity 150ms;}
  .reacts button {width:36px;height:36px;border-radius:50%;transition:background 100ms;}
  :global(.player.idle) .room-status {opacity:0;visibility:hidden;}
  @media(max-width:640px) {.room-status {top:78px;font-size:11px}.room-status small {font-size:10px}.side {top:130px;bottom:145px;right:12px;width:calc(100vw - 24px);border-radius:20px}.reacts button {width:30px;height:36px}.reacts {max-width:calc(100vw - 24px);}}
  .extra-members {font-size:11px;color:#b8b8c0;padding-left:4px;}
  @media(max-height:500px) {.side {top:76px;bottom:100px;overflow:auto;}.room-status {top:68px;}.reacts {bottom:94px;}}
  /* Поверх видео - строго между панелями плеера: их высоту плеер отдаёт в --top-h и --bottom-h. */
  .room-status {
    top: calc(var(--top-h, 80px) + 8px);
    left: clamp(12px, 3vw, 36px);
    max-width: calc(100% - 92px);
    overflow: hidden;
    white-space: nowrap;
    transition: opacity 0.3s ease, transform 0.3s ease;
  }
  .room-status small {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .toggle {
    top: calc(var(--top-h, 80px) + 4px);
    right: clamp(12px, 3vw, 36px);
    transition: opacity 0.3s ease, transform 0.3s ease;
  }
  .side {
    top: calc(var(--top-h, 80px) + 56px);
    bottom: calc(var(--bottom-h, 120px) + 64px);
    height: auto;
    animation: side-in 0.25s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  .reacts {
    bottom: calc(var(--bottom-h, 120px) + 10px);
    transition: opacity 0.3s ease, transform 0.3s ease;
  }
  .fly {
    bottom: calc(var(--bottom-h, 120px) + 124px);
  }
  :global(.player.idle) .room-status,
  :global(.player.idle) .room .toggle {
    opacity: 0;
    visibility: hidden;
    transform: translateY(-8px);
    transition: opacity 0.3s ease, transform 0.3s ease, visibility 0s 0.3s;
  }
  :global(.player.idle) .reacts {
    transform: translate(-50%, 8px);
    transition: opacity 0.3s ease, transform 0.3s ease, visibility 0s 0.3s;
  }
  @keyframes side-in {
    from {
      opacity: 0;
      transform: translateX(16px);
    }
  }
  @media (max-width: 640px) {
    .side {
      top: calc(var(--top-h, 80px) + 52px);
      bottom: calc(var(--bottom-h, 120px) + 60px);
      height: auto;
    }
  }
  @media (min-width: 801px) and (min-height: 501px) {
    .room:not(.closed) .side { top:0; bottom:0; right:0; width:360px; border-radius:0; height:100%; padding-top:24px; }
    .room:not(.closed) .toggle { right:372px; }
    .room:not(.closed) .reacts { left:calc((100% - 360px) / 2); }
  }
  .side > :global(*) { flex: none; }
  .side > .queue, .side > .feed { flex: 1 1 auto; min-height: 60px; overflow-y: auto; }
  .tabs2 { display: flex; gap: 6px; padding: 4px; border-radius: 999px; background: #17171a; }
  .tabs2 button { flex: 1; padding: 8px 0; border: 0; border-radius: 999px; background: none; color: #c4c4cc; font: inherit; font-size: 13px; font-weight: 700; cursor: pointer; }
  .tabs2 button.on { background: #fff; color: #0a0a0b; }
  .tabs2 sup { margin-left: 4px; font-size: 10px; }
  .who { margin: 10px 2px 6px; color: var(--muted); font-size: 12px; line-height: 1.4; }
  .queue { display: grid; align-content: start; gap: 6px; margin: 0; padding: 0; list-style: none; overflow-y: auto; flex: 1; min-height: 0; }
  .queue li { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border: 1px solid #ffffff14; border-radius: 16px; background: #17171a; }
  .queue li.now { border-color: #fff; }
  .queue li.ask { border-style: dashed; }
  .src { flex: none; padding: 2px 8px; border-radius: 999px; background: #ffffff1a; font-size: 10px; font-weight: 700; }
  .nm { display: grid; flex: 1; min-width: 0; }
  .nm b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .nm small { color: var(--muted); font-size: 11px; }
  .sq { display: grid; place-items: center; flex: none; width: 26px; height: 26px; padding: 0; border: 0; border-radius: 50%; background: #ffffff14; color: #fff; font-size: 12px; cursor: pointer; }
  .sq:disabled { opacity: 0.3; cursor: default; }
  .addq { display: flex; gap: 6px; margin-top: 8px; }
  .addq .input { flex: 1; min-width: 0; }
  .safe { margin: 4px 2px 0; color: var(--muted); font-size: 11px; line-height: 1.4; }
  /* Пустая комната. */
  .emptyroom { display: grid; place-items: center; min-height: 100dvh; padding: calc(var(--header) + 24px) var(--gutter) calc(40px + var(--tabbar)); background: #000; }
  .er-card { display: grid; gap: 16px; width: min(560px, 100%); padding: clamp(24px, 4vw, 40px); border: 1px solid #ffffff1a; border-radius: 32px; background: #0d0d10; }
  .er-ic { display: grid; place-items: center; width: 56px; height: 56px; border-radius: 50%; background: #ffffff12; }
  .er-card h1 { margin: 0; font-size: clamp(28px, 4vw, 40px); font-weight: 800; letter-spacing: -0.03em; }
  .er-card > p { margin: 0; line-height: 1.5; }
  .er-row { display: flex; flex-wrap: wrap; gap: 10px; }
  .er-add { display: grid; gap: 8px; margin-top: 6px; padding-top: 18px; border-top: 1px solid #ffffff14; }
  .er-add label { font-size: 13px; font-weight: 700; }
  .er-in { display: flex; gap: 8px; }
  .er-in .input { flex: 1; min-width: 0; }
  .er-add small { font-size: 12px; }
  .er-who { display: flex; flex-wrap: wrap; gap: 6px; margin: 0; padding: 0; list-style: none; }
  .er-who li { padding: 6px 12px; border-radius: 999px; background: #ffffff12; font-size: 12.5px; font-weight: 600; }
  @media (max-width: 480px) { .er-in { flex-direction: column; } }

  /* ---------- Телефон (26.11): ничего не налезает на управление ---------- */
  @media (max-height: 500px), (max-width: 640px) {
    /* «Смотрим вместе · N участн.» - в чате, на кадре не нужен. */
    .room-status { display: none; }
    /* Реакции - столбиком у правого края между верхней и нижней строкой плеера; прячутся вместе с управлением. */
    .reacts {
      left: auto !important;
      right: max(10px, env(safe-area-inset-right));
      top: calc(var(--top-h, 60px) + 54px);
      bottom: auto !important;
      max-height: calc(100% - var(--top-h, 60px) - var(--bottom-h, 90px) - 62px);
      transform: none !important;
      flex-direction: column;
      justify-content: flex-start;
      max-width: none !important;
      width: 44px;
      padding: 4px !important;
      overflow-y: auto;
      scrollbar-width: none;
      border-radius: 22px !important;
    }
    .reacts::-webkit-scrollbar { display: none; }
    .reacts button { flex: none; width: 36px !important; height: 36px !important; font-size: 19px; }
    :global(.player.idle) .reacts { opacity: 0; visibility: hidden; }
    .seats { display: none; }
    .fly { bottom: calc(var(--bottom-h, 90px) + 20px); }
    /* Размытие под чатом и реакциями на телефоне - лишняя работа видеокарте поверх видео: плотный фон. */
    .reacts, .side { backdrop-filter: none !important; background: #111114f2 !important; }
  }
</style>
