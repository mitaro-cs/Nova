<script lang="ts">
  import { onMount } from 'svelte';
  import { api, errorText, post } from '$lib/api';
  import { catalog, loadCatalog } from '$lib/catalog.svelte';
  import { kindLabel, plural, runtime } from '$lib/fmt';
  import { badges } from '$lib/social/badges.svelte';
  import type { ExtCard } from '$lib/types';
  import { artSet, artW, placeholder } from '$lib/ui/art';
  import Avatar from '$lib/ui/Avatar.svelte';
  import { fadeIn } from '$lib/ui/fade';
  import { toast } from '$lib/ui/toast.svelte';
  import { attention } from './attention';
  import { href } from './nav';
  import type { Health, Metrics, Status } from './types';

  // Сводка «как сайт Nova» (выбор владельца 28.11): сверху карусель эфира, как герой главной, - кадр того, что смотрят,
  // огромное название, кто смотрит, соседние зрители видны по бокам; когда тихо - свежее в библиотеке. Ниже «Требует
  // внимания» широкими карточками (как «Продолжить просмотр»), бета большой цифрой и лицами зрителей, неделя цифрами
  // через тонкие черты и свежие постеры.
  let { status, health, reload }: { status: Status; health: Health | null; reload: () => void } = $props();

  interface U { id: number; name: string; role: string; disabled: boolean; seenAt: number | null; avatar: number; photo: string | null }
  interface Wish { card: ExtCard; count: number; people: string }
  let users = $state<U[] | null>(null);
  let metrics = $state<Metrics | null>(null);
  let wishes = $state<Wish[]>([]);
  let pick = $state(0);
  let busy = $state('');

  function load() {
    api<U[]>('/api/admin/users').then((u) => (users = u), () => {});
    api<Metrics>(`/api/admin/metrics?tz=${-new Date().getTimezoneOffset()}`).then((m) => (metrics = m), () => {});
    api<Wish[]>('/api/admin/wishes').then((w) => (wishes = w), () => {});
  }
  onMount(() => {
    load();
    loadCatalog();
    const fast = setInterval(reload, 5000);
    const slow = setInterval(load, 30000);
    return () => {
      clearInterval(fast);
      clearInterval(slow);
    };
  });

  const fresh = $derived((catalog.items ?? []).filter((c) => c.poster).slice().sort((a, b) => b.addedAt - a.addedAt).slice(0, 12));
  const pct = (w: Status['watching'][number]) => (w.duration > 0 ? Math.min(100, Math.round((w.position / w.duration) * 100)) : 0);
  const mins = (s: number) => runtime(Math.max(1, Math.round(s / 60)));
  const quality = (q: string | undefined) => (!q ? '' : q === 'orig' ? 'оригинал' : q === 'auto' ? 'авто' : q);

  // Слайды: кто смотрит сейчас; никого - три свежих фильма с кадром.
  type Slide = { key: string; title: string; art: string | null; tag: string; meta: string; pct: number | null; href: string; avatar?: number; photo?: string | null };
  const slides = $derived.by<Slide[]>(() => {
    const w = status.watching;
    if (w.length)
      return w.map((x, i) => ({
        key: `w${i}${x.userId ?? x.user}`,
        title: x.title,
        art: x.backdrop ?? x.poster ?? null,
        tag: `Сейчас смотрит · ${w.length} в эфире`,
        meta: [x.user, x.episode, quality(x.net?.quality), x.duration ? `${mins(x.position)} из ${mins(x.duration)}` : ''].filter(Boolean).join(' · '),
        pct: pct(x),
        href: x.href ?? '#',
        avatar: x.avatar ?? 0,
        photo: x.photo ?? null,
      }));
    return fresh
      .filter((c) => c.backdrop)
      .slice(0, 3)
      .map((c) => ({ key: `f${c.id}`, title: c.title, art: c.backdrop, tag: 'Сейчас тихо · новое в библиотеке', meta: [c.year, kindLabel(c.kind)].filter(Boolean).join(' · '), pct: null, href: `/title/${c.id}` }));
  });
  const cur = $derived(slides.length ? Math.min(pick, slides.length - 1) : 0);
  const place = (i: number) => {
    const n = slides.length;
    const d = (i - cur + n) % n;
    return d === 0 ? 'c' : d === 1 ? 'r' : d === n - 1 ? 'l' : 'h';
  };
  function key(e: KeyboardEvent) {
    if (slides.length < 2) return;
    if (e.key === 'ArrowRight') pick = (cur + 1) % slides.length;
    if (e.key === 'ArrowLeft') pick = (cur - 1 + slides.length) % slides.length;
  }

  // Дела: у просьбы - кадр фильма, у остальных - крупная цифра на тёмном.
  const items = $derived(attention(status, health, badges.wishes));
  const wishArt = $derived(wishes[0]?.card.poster ?? null);
  const LEVEL = { bad: 'Важно', warn: 'Стоит заглянуть', info: 'Подсказка' } as const;
  const ghost = (t: string) => t.match(/\d+/)?.[0] ?? '';
  async function act(id: string) {
    if (busy) return;
    busy = id;
    try {
      await post('/api/admin/backup');
      toast('Копия базы готова');
      reload();
    } catch (e) {
      toast(errorText(e));
    } finally {
      busy = '';
    }
  }

  // Бета: лица - сначала те, кто смотрит, потом по последнему входу.
  const seats = $derived(status.seats ?? null);
  const live = $derived(new Set(status.watching.map((w) => w.userId)));
  const viewers = $derived((users ?? []).filter((u) => u.role !== 'host'));
  const faces = $derived(viewers.slice().sort((a, b) => Number(live.has(b.id)) - Number(live.has(a.id)) || (b.seenAt ?? 0) - (a.seenAt ?? 0)).slice(0, 11));
  const DAY = 86_400_000;
  function seen(u: U) {
    if (live.has(u.id)) return 'смотрит';
    if (!u.seenAt) return 'не заходил';
    const d = Math.floor((Date.now() - u.seenAt) / DAY);
    return d < 1 ? 'сегодня' : d < 2 ? 'вчера' : `${d} ${plural(d, 'день', 'дня', 'дней')} назад`;
  }

  const DOW = ['воскресенье', 'понедельник', 'вторник', 'среда', 'четверг', 'пятница', 'суббота'];
  const week = $derived.by(() => {
    const days = metrics?.activity.days.slice(-7) ?? [];
    const top = days.reduce((a, d) => (d.views > (a?.views ?? 0) ? d : a), null as (typeof days)[number] | null);
    const d = top ? new Date(top.day).getDay() : 0;
    return { views: days.reduce((n, x) => n + x.views, 0), top: top ? DOW[d] : null, short: ['Вс', 'Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб'][d] };
  });
</script>

<svelte:window onkeydown={key} />

<div class="live">
  <section class="car" aria-label="Эфир" aria-roledescription="карусель">
    {#each slides as s, i (s.key)}
      {@const p = place(i)}
      <div class="slide {p}" style:background={placeholder(s.title)} aria-hidden={p !== 'c'}>
        {#if s.art}<img use:fadeIn src={artW(s.art, 1280)} srcset={artSet(s.art)} sizes="(max-width: 1000px) 100vw, 66vw" alt="" decoding="async" />{/if}
        {#if p === 'l' || p === 'r'}<button class="peek" type="button" aria-label="Показать: {s.title}" tabindex="-1" onclick={() => (pick = i)}></button>{/if}
        <div class="over">
          <span class="tag"><i class:on={s.pct !== null} aria-hidden="true"></i>{s.tag}</span>
          <h3>{s.title}</h3>
          <div class="meta">{#if s.avatar !== undefined}<Avatar avatar={s.avatar} photo={s.photo ?? null} size={26} />{/if}<span>{s.meta}</span></div>
          {#if s.pct !== null}<div class="bar" aria-hidden="true"><i style:transform="scaleX({s.pct / 100})"></i></div>{/if}
          <div class="acts">
            <a class="btn primary" href={s.href} tabindex={p === 'c' ? 0 : -1}>Открыть фильм</a>
            <a class="btn" href={href('viewers', 'people')} tabindex={p === 'c' ? 0 : -1} data-sveltekit-replacestate data-sveltekit-noscroll>Зрители</a>
          </div>
        </div>
      </div>
    {:else}
      <div class="slide c empty-car">
        <div class="over"><span class="tag"><i aria-hidden="true"></i>Сейчас тихо</span><h3>Nova ждёт зрителей</h3><div class="meta"><span>Когда кто-то включит фильм, он появится здесь</span></div>
          <div class="acts"><a class="btn primary" href={href('library', 'folders')} data-sveltekit-replacestate data-sveltekit-noscroll>Добавить фильмы</a></div></div>
      </div>
    {/each}
  </section>
  {#if slides.length > 1}
    <div class="dots" role="group" aria-label="Слайды эфира">
      {#each slides as s, i (s.key)}<button type="button" aria-label={s.title} aria-current={i === cur ? 'true' : undefined} onclick={() => (pick = i)}></button>{/each}
    </div>
  {/if}

  <section aria-labelledby="att-h">
    <h2 class="row" id="att-h">Требует внимания {#if items.length}<small>{items.length} {plural(items.length, 'дело', 'дела', 'дел')}</small>{/if}</h2>
    <div class="strip">
      {#each items as a (a.id)}
        {@const art = a.id === 'wishes' ? wishArt : null}
        <article class="wide" class:plain={!art}>
          {#if art}<img src={artW(art, 640)} alt="" loading="lazy" decoding="async" />{:else if ghost(a.title)}<span class="ghost" aria-hidden="true">{ghost(a.title)}</span>{/if}
          <span class="lv {a.level}"><i aria-hidden="true"></i>{LEVEL[a.level]}</span>
          <b>{a.title}</b>
          <p>{a.text}</p>
          <div class="row2">
            {#if a.act}<button class="btn primary sm" type="button" disabled={busy === a.id} onclick={() => act(a.id)}>{busy === a.id ? 'Делаем…' : a.label}</button>
            {:else if a.href}<a class="btn primary sm" href={a.href} data-sveltekit-replacestate data-sveltekit-noscroll>{a.label}</a>{/if}
          </div>
        </article>
      {:else}
        <article class="wide plain calm"><span class="lv ok"><i aria-hidden="true"></i>Всё в порядке</span><b>Ничего не ждёт вашего решения</b><p>Источники работают, просьб нет, копия базы свежая</p></article>
      {/each}
    </div>
  </section>

  <section aria-labelledby="beta-h">
    <h2 class="row" id="beta-h">{seats ? 'Бета' : 'Зрители'} {#if seats}<small>первые {seats.total} зрителей - бесплатно</small>{/if}</h2>
    <div class="beta">
      <div>
        <div class="bignum">{seats ? seats.used : viewers.length}{#if seats}<small> / {seats.total}</small>{/if}</div>
        {#if seats}<div class="meter" aria-hidden="true"><i style:transform="scaleX({Math.min(1, seats.used / Math.max(1, seats.total))})"></i></div>{/if}
        <p>
          {#if !seats}Без лимита мест.
          {:else if seats.left > 0}Осталось {seats.left} {plural(seats.left, 'место', 'места', 'мест')}. Когда займут все, регистрация закроется сама.
          {:else}Все места заняты - регистрация закрыта.{/if}
        </p>
        <div class="row2">
          <a class="btn primary sm" href={href('viewers', 'people')} data-sveltekit-replacestate data-sveltekit-noscroll>Все зрители</a>
          <a class="btn sm" href={href('viewers', 'signup')} data-sveltekit-replacestate data-sveltekit-noscroll>Правила входа</a>
        </div>
      </div>
      <ul class="faces" aria-label="Зрители">
        {#each faces as u (u.id)}
          <li class:now={live.has(u.id)}><Avatar avatar={u.avatar} photo={u.photo} size={72} /><b>{u.name}</b><small>{seen(u)}</small></li>
        {/each}
        {#if viewers.length > faces.length}<li class="more"><span class="circle">+{viewers.length - faces.length}</span><b>ещё</b></li>{/if}
        {#if seats && seats.left > 0}<li class="free"><span class="circle">{seats.left}</span><b>свободно</b></li>{/if}
      </ul>
    </div>
  </section>

  <section aria-labelledby="week-h">
    <h2 class="row" id="week-h">Неделя в Nova</h2>
    <div class="facts">
      <div><strong>{metrics ? week.views : '-'}</strong><span>{plural(week.views, 'просмотр', 'просмотра', 'просмотров')}</span></div>
      <div><strong>{metrics?.people.activeWeek ?? '-'}</strong><span>{plural(metrics?.people.activeWeek ?? 0, 'зритель', 'зрителя', 'зрителей')}</span></div>
      <div><strong>{metrics?.library.addedWeek ?? '-'}</strong><span>новых в библиотеке</span></div>
      {#if week.top}<div><strong>{week.short}</strong><span>самый кинодень - {week.top}</span></div>{/if}
    </div>
  </section>

  {#if fresh.length}
    <section aria-labelledby="fresh-h">
      <h2 class="row" id="fresh-h">Свежее в библиотеке <small>{status.titles} {plural(status.titles, 'фильм и сериал', 'фильма и сериала', 'фильмов и сериалов')}</small></h2>
      <div class="strip">
        {#each fresh as c (c.id)}
          <a class="poster" href="/title/{c.id}"><img src={artW(c.poster, 342)} alt="" loading="lazy" decoding="async" /><b>{c.title}</b><small>{[c.year, kindLabel(c.kind)].filter(Boolean).join(' · ')}</small></a>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .live { display: grid; grid-template-columns: minmax(0, 1fr); min-width: 0; }

  /* Карусель - как герой главной: центральная карточка, соседи видны по бокам. */
  .car { position: relative; height: min(62vh, 560px); min-height: 380px; overflow: hidden; margin-inline: calc(var(--gutter) * -1); }
  .slide { position: absolute; top: 0; bottom: 0; left: 17%; width: 66%; border-radius: 28px; overflow: hidden; isolation: isolate; transition: transform 0.7s var(--ease), opacity 0.7s var(--ease), left 0.7s var(--ease); }
  .slide img { position: absolute; inset: 0; width: 100%; max-width: none; height: 100%; object-fit: cover; z-index: -1; }
  .slide::after { content: ''; position: absolute; inset: 0; z-index: -1; background: radial-gradient(80% 90% at 50% 60%, #0000 10%, #000b 100%), linear-gradient(0deg, #000d, #0000 55%); }
  .slide.l { left: -52%; opacity: 0.45; }
  .slide.r { left: 86%; opacity: 0.45; }
  .slide.h { left: 86%; opacity: 0; pointer-events: none; }
  .peek { position: absolute; inset: 0; z-index: 2; border: 0; background: none; cursor: pointer; }
  .over { position: absolute; inset: auto 0 0; display: flex; flex-direction: column; align-items: center; gap: 12px; padding: 0 40px 36px; text-align: center; opacity: 0; transition: opacity 0.4s; }
  .slide.c .over { opacity: 1; transition-delay: 0.2s; }
  .tag { display: inline-flex; align-items: center; gap: 7px; padding: 5px 12px; border-radius: 99px; background: #0009; font-size: 11.5px; font-weight: 800; letter-spacing: 0.1em; text-transform: uppercase; }
  .tag i { width: 7px; height: 7px; border-radius: 50%; background: #8e8e93; }
  .tag i.on { background: #ff3b30; animation: pulse 1.6s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  h3 { margin: 2px 0 0; max-width: 100%; font-size: clamp(34px, 6vw, 84px); font-weight: 900; letter-spacing: -0.04em; line-height: 0.95; text-transform: uppercase; overflow-wrap: anywhere; }
  .meta { display: flex; align-items: center; justify-content: center; gap: 10px; flex-wrap: wrap; color: #e5e5ea; font-weight: 600; }
  .bar { width: min(420px, 80%); height: 4px; border-radius: 4px; background: #ffffff38; overflow: hidden; }
  .bar i { display: block; height: 100%; background: #fff; transform-origin: 0 50%; }
  .acts { display: flex; gap: 10px; flex-wrap: wrap; justify-content: center; margin-top: 6px; }
  .empty-car { background: radial-gradient(90% 120% at 50% 0%, #26262b, #0c0c0e 70%); }
  .dots { display: flex; justify-content: center; gap: 8px; margin-top: 18px; }
  .dots button { width: 8px; height: 8px; padding: 0; border: 0; border-radius: 99px; background: #ffffff40; cursor: pointer; transition: width 0.3s var(--ease), background 0.3s; }
  .dots button[aria-current] { width: 34px; background: #fff; }

  .row { display: flex; align-items: baseline; gap: 14px; flex-wrap: wrap; margin: 48px 0 16px; font-size: 26px; font-weight: 800; letter-spacing: -0.02em; }
  .row small { color: var(--muted); font-size: 15px; font-weight: 600; letter-spacing: 0; }
  .strip { display: flex; gap: 16px; overflow-x: auto; margin: -6px calc(var(--gutter) * -1) 0; padding: 6px var(--gutter) 14px; scrollbar-width: none; }
  .row2 { display: flex; gap: 8px; flex-wrap: wrap; }

  /* Дела - широкие карточки. */
  .wide { position: relative; isolation: isolate; flex: none; display: flex; flex-direction: column; justify-content: flex-end; width: 340px; aspect-ratio: 16 / 10; padding: 18px; border-radius: 20px; overflow: hidden; background: #141416; transition: transform 0.3s var(--ease); }
  .wide:hover { transform: translateY(-4px); }
  .wide > img { position: absolute; inset: 0; width: 100%; max-width: none; height: 100%; object-fit: cover; z-index: -2; }
  .wide::before { content: ''; position: absolute; inset: 0; z-index: -1; background: linear-gradient(0deg, #000e 20%, #0004 70%, #0000); }
  .wide.plain { background: radial-gradient(120% 90% at 100% 0%, #2a2a2e, #141416 60%); }
  .wide.plain::before { content: none; }
  .ghost { position: absolute; top: 12px; right: 18px; z-index: -1; font-size: 72px; font-weight: 900; letter-spacing: -0.05em; line-height: 1; color: #ffffff17; }
  .lv { position: absolute; top: 16px; left: 16px; display: inline-flex; align-items: center; gap: 7px; padding: 5px 11px; border-radius: 99px; background: #000a; font-size: 12px; font-weight: 700; }
  .lv i { width: 8px; height: 8px; border-radius: 50%; background: #8e8e93; }
  .lv.bad i { background: var(--danger); }
  .lv.warn i { background: #ffd60a; }
  .lv.ok i { background: var(--ok); }
  .wide b { font-size: 18px; font-weight: 800; letter-spacing: -0.01em; line-height: 1.2; }
  .wide p { margin: 4px 0 12px; color: #d1d1d6; font-size: 13.5px; line-height: 1.4; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .wide.calm p { margin-bottom: 0; }

  /* Бета - большая цифра и лица. */
  .beta { display: grid; grid-template-columns: minmax(240px, 340px) minmax(0, 1fr); gap: 32px; align-items: center; padding: 30px; border-radius: 28px; background: radial-gradient(90% 140% at 0% 0%, #1e1e22, #0d0d0f 70%); }
  .bignum { font-size: clamp(72px, 9vw, 132px); font-weight: 900; letter-spacing: -0.06em; line-height: 0.85; font-variant-numeric: tabular-nums; }
  .bignum small { font-size: 0.32em; color: var(--muted); letter-spacing: -0.02em; }
  .meter { height: 6px; margin-top: 18px; border-radius: 6px; background: #ffffff1f; overflow: hidden; }
  .meter i { display: block; height: 100%; background: #fff; transform-origin: 0 50%; }
  .beta p { margin: 14px 0 18px; color: var(--muted); }
  .faces { display: grid; grid-template-columns: repeat(auto-fill, minmax(84px, 1fr)); gap: 18px 12px; min-width: 0; margin: 0; padding: 0; list-style: none; }
  .faces li { display: flex; flex-direction: column; align-items: center; gap: 8px; min-width: 0; text-align: center; }
  .faces li :global(.ava) { transition: transform 0.25s var(--ease); }
  .faces li:hover :global(.ava) { transform: translateY(-3px); }
  .faces li.now :global(.ava) { box-shadow: 0 0 0 3px #0d0d0f, 0 0 0 5px #ff3b30; }
  .faces b { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .faces small { margin-top: -6px; color: #6e6e73; font-size: 11.5px; }
  .faces li.now small { color: #ff6961; }
  .circle { display: grid; place-items: center; width: 72px; height: 72px; border-radius: 50%; font-weight: 800; font-size: 18px; }
  .more .circle { background: #ffffff1a; }
  .free .circle { border: 2px dashed #ffffff33; color: var(--muted); }

  .facts { display: flex; flex-wrap: wrap; gap: 18px 0; }
  .facts div { padding: 0 32px; border-left: 1px solid var(--line); }
  .facts div:first-child { padding-left: 0; border-left: 0; }
  .facts strong { display: block; font-size: 44px; font-weight: 900; letter-spacing: -0.04em; line-height: 1; font-variant-numeric: tabular-nums; }
  .facts span { color: var(--muted); font-size: 14px; }

  .poster { flex: none; width: 168px; color: inherit; }
  .poster img { width: 100%; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 18px; transition: transform 0.3s var(--ease); }
  .poster:hover img { transform: translateY(-4px); }
  .poster b { display: block; margin-top: 10px; font-size: 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .poster small { color: #6e6e73; font-size: 12.5px; }

  @media (max-width: 1000px) {
    .slide { left: 0; width: 100%; border-radius: 0; }
    .slide.l { left: -105%; }
    .slide.r, .slide.h { left: 105%; }
    .beta { grid-template-columns: minmax(0, 1fr); }
  }
  @media (max-width: 640px) {
    .car { height: 460px; min-height: 0; }
    .over { padding: 0 18px 26px; }
    .wide { width: 280px; }
    .poster { width: 132px; }
    .beta { padding: 20px; }
    .faces { grid-template-columns: repeat(4, minmax(0, 1fr)); }
    .faces li :global(.ava), .circle { width: 60px; height: 60px; }
    .facts div { padding: 0 18px; }
    .facts strong { font-size: 32px; }
  }
</style>
