<script lang="ts">
  import { untrack } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { heroSet } from '$lib/ui/art';
  import { fitTitle } from '$lib/ui/fitTitle';
  import { getDetail, peekDetail } from '$lib/discover/details';
  import { genreRu } from '$lib/genres';
  import { motionOn } from '$lib/motion';
  import { toggle, toggleMark } from '$lib/marks';
  import { localized } from '$lib/localized.svelte';
  import type { Card, ExtCard, ExtDetail } from '$lib/types';
  import type { Component } from 'svelte';
  import Icon from './Icon.svelte';

  let { items }: { items: { card?: Card; ext?: ExtCard }[] } = $props();

  // Поверх кадра - трейлер без звука (`ShowcaseTrailer`, грузится лениво: бюджет JS главной);
  // при экономии трафика и выключенной настройке - только кадр.
  let Trailer = $state<Component<any> | null>(null);
  $effect(() => {
    if (Trailer) return;
    // Трейлер стартует чуть позже кадра: сначала страница, потом видео (и меньше кода при первой загрузке).
    const t = setTimeout(() => void import('./ShowcaseTrailer.svelte').then((m) => (Trailer = m.default)), 1500);
    return () => clearTimeout(t);
  });
  // Сколько показывается один слайд; столько же заполняется полоска в активной точке.
  const SLIDE_MS = 7000;
  let at = $state(0);
  let manual = $state(false);
  let hovering = $state(false);
  let visible = $state(true);
  let root = $state<HTMLElement>();
  let restored = false;
  const selected = $derived(items[Math.min(at, items.length - 1)]);
  // Подробности фильма из ленты: берём из кэша сразу (без пустого кадра при переключении), свежие - из `fetched`.
  let fetched = $state<Record<string, ExtDetail>>({});
  const extId = $derived(selected?.ext?.id);
  const detail = $derived<ExtDetail | null>(extId ? (fetched[extId] ?? peekDetail(extId)) : null);
  const card = $derived(selected?.card ?? detail?.card ?? selected?.ext);
  const href = $derived(selected?.card ? `/title/${selected.card.id}` : `/ext/${selected?.ext?.id}`);
  const backdrop = $derived(selected?.card?.backdrop ?? detail?.backdrop ?? detail?.gallery?.[0] ?? null);
  const overview = $derived(localized(selected?.card?.overview ?? detail?.overview));
  $effect(() => {
    const id = extId;
    if (!id) return;
    let live = true;
    let timer = 0;
    let tries = 0;
    const load = () => getDetail(id).then((r) => { if (!live) return; fetched[id] = r; if (r.refreshing && ++tries < 12) timer = window.setTimeout(load,3000); }, () => {});
    load();
    return () => { live = false; clearTimeout(timer); };
  });

  // Кадр - два слоя друг над другом: новый кадр сначала загружается и раскодируется в скрытом слое,
  // и только потом слои меняются местами (прозрачность - её считает видеокарта). Так в переключении
  // нет ни чёрных вспышек, ни рывков от декодирования большой картинки.
  type Shot = { src: string; set: string | undefined };
  let layers = $state<[Shot | null, Shot | null]>([null, null]);
  let front = $state(0);
  $effect(() => {
    const src = backdrop;
    if (!src) return;
    const set = heroSet(src, selected?.card?.backdropSmall);
    const now = untrack(() => front);
    if (untrack(() => layers[now])?.src === src) return;
    const next = 1 - now;
    layers[next] = { src, set };
    // Следующий кадр уже раскодирован заранее (go()), поэтому слои можно менять сразу.
    front = next;
  });

  const wait = (ms: number) => new Promise<void>((done) => setTimeout(done, ms));
  function decode(src: string, small?: string | null) {
    const img = new Image();
    img.decoding = 'async';
    img.sizes = '100vw';
    const set = heroSet(src, small);
    if (set) img.srcset = set;
    img.src = src;
    return Promise.race([img.decode().catch(() => {}), wait(2500)]);
  }
  /** Следующий слайд готов: описание получено, кадр загружен и раскодирован (или прошло 2,5 с). */
  async function ready(i: number) {
    const it = items[i];
    if (!it) return;
    let src = it.card?.backdrop ?? null;
    if (!src && it.ext) {
      const d = await Promise.race([getDetail(it.ext.id).catch(() => null), wait(2500).then(() => null)]);
      if (d) fetched[it.ext.id] = d;
      src = d?.backdrop ?? d?.gallery?.[0] ?? null;
    }
    if (src) await decode(src, it.card?.backdropSmall);
  }
  let switching = false;
  // Куда едет карусель (+1 вперёд, -1 назад): соседние карточки по бокам выезжают с этой стороны.
  let dir = $state(1);
  /** Переключить слайд: сначала всё готово - потом текст и кадр меняются вместе. */
  async function go(i: number, way = 0) {
    const to = ((i % items.length) + items.length) % items.length;
    if (switching || to === at) return;
    switching = true;
    try {
      await ready(to);
      dir = way || (to > at ? 1 : -1);
      at = to;
    } finally {
      switching = false;
    }
  }
  const prevAt = $derived((at - 1 + items.length) % items.length);
  const nextAt = $derived((at + 1) % items.length);
  /** Кадр соседней карточки: свой, из ленты - если уже получен, иначе постер. */
  function peekArt(i: number): string | null {
    const it = items[i];
    if (!it) return null;
    if (it.card) return it.card.backdropSmall ?? it.card.backdrop ?? it.card.poster ?? null;
    const id = it.ext!.id;
    const d = fetched[id] ?? peekDetail(id);
    return d?.backdrop ?? d?.gallery?.[0] ?? it.ext!.poster ?? null;
  }
  function keys(e: KeyboardEvent) {
    hold();
    if (items.length < 2 || (e.target instanceof HTMLElement && e.target.closest('input,textarea'))) return;
    if (e.key === 'ArrowRight') void go(at + 1, 1);
    else if (e.key === 'ArrowLeft') void go(at - 1, -1);
  }
  $effect(() => {
    if (!root) return;
    const observer = new IntersectionObserver(([entry]) => visible = entry.isIntersecting, { threshold: .2 });
    observer.observe(root); return () => observer.disconnect();
  });
  $effect(() => {
    if (restored || !items.length) return;
    restored = true;
    try { const saved = JSON.parse(sessionStorage.getItem('nova-showcase') ?? 'null'); const i = items.findIndex((c) => String(c.card?.id ?? c.ext?.id) === saved?.id); if (i >= 0) { at = i; manual = !!saved.manual; } } catch { /* пустая сессия */ }
  });
  $effect(() => {
    if (manual || hovering || !visible || items.length < 2 || !motionOn()) return;
    const timer = setInterval(() => { if (document.visibilityState === 'visible') void go(at + 1, 1); }, SLIDE_MS);
    return () => clearInterval(timer);
  });
  // Следующий слайд готовим заранее: описание и кадр - пока показывается этот.
  $effect(() => {
    if (manual || !visible || items.length < 2) return;
    const next = (at + 1) % items.length;
    const timer = setTimeout(() => { void ready(next); }, 1200);
    return () => clearTimeout(timer);
  });
  let touchStart: { x: number; y: number } | null = null;
  function hold(e?: PointerEvent) {
    manual = true;
    if (e?.pointerType === 'touch') touchStart = { x: e.clientX, y: e.clientY };
    save();
  }
  function swipe(e: PointerEvent) {
    if (e.pointerType !== 'touch' || !touchStart || items.length < 2) return;
    const dx = e.clientX - touchStart.x, dy = e.clientY - touchStart.y;
    touchStart = null;
    if (Math.abs(dx) > 50 && Math.abs(dx) > Math.abs(dy) * 1.2) { void go(at + (dx < 0 ? 1 : -1), dx < 0 ? 1 : -1); save(); }
  }
  function save() { try { sessionStorage.setItem('nova-showcase', JSON.stringify({ id: String(selected?.card?.id ?? selected?.ext?.id), manual })); } catch { /* приватный режим */ } }

  // Надпись над названием.
  const ext = $derived(selected?.ext);
  const eyebrow = $derived(
    selected?.card?.resume ? 'Вы смотрели'
      : selected?.card ? 'Новое на Nova'
        : ext?.mark === 'want' ? 'В ваших планах'
          : ext?.mark === 'watching' ? 'Вы смотрите'
            : 'Популярно сейчас',
  );
  const wanted = $derived(selected?.card ? selected.card.status === 'want' : ext?.mark === 'want');
  function want(e: Event) {
    e.stopPropagation();
    if (selected?.card) void toggle(selected.card, 'want');
    else if (selected?.ext) void toggleMark(selected.ext, 'want');
  }
  // «Не интересно» (то же, что «Больше не смотрю», с отменой) - код грузится только по нажатию.
  async function nope(e: Event) {
    e.stopPropagation();
    (await import('./nope')).nope(selected?.card, selected?.ext);
  }
</script>

{#if card}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <section class="showcase" style:--dir={dir} aria-label="Рекомендуем" bind:this={root} onpointerenter={() => hovering = true} onpointerleave={() => hovering = false} onpointerdown={hold} onpointerup={swipe} onpointercancel={() => touchStart = null} onkeydown={keys}>
    <div class="track">
      {#if items.length > 1}
        {#key prevAt}<button class="peek l" tabindex="-1" aria-hidden="true" onclick={() => { manual = true; void go(prevAt, -1); save(); }}>{#if peekArt(prevAt)}<img src={peekArt(prevAt)} alt="" decoding="async" />{/if}</button>{/key}
        {#key nextAt}<button class="peek r" tabindex="-1" aria-hidden="true" onclick={() => { manual = true; void go(nextAt, 1); save(); }}>{#if peekArt(nextAt)}<img src={peekArt(nextAt)} alt="" decoding="async" />{/if}</button>{/key}
      {/if}
      <div class="card">
        <div class="stage" aria-hidden="true">
          {#each layers as shot, i (i)}
            {#if shot}<img class="backdrop" class:on={front === i} src={shot.src} srcset={shot.set} sizes="100vw" alt="" decoding="async" fetchpriority={i === front ? 'high' : 'auto'} />{/if}
          {/each}
        </div>
        {#if Trailer}<Trailer card={selected?.card} ext={selected?.ext} />{/if}
        <div class="shade"></div>
        <div class="copies">{#key card.id}<div class="copy" in:fly|global={{ y: 14, duration: 650, delay: 300, easing: cubicOut }} out:fade|global={{ duration: 240 }}>
          <span class="eyebrow">{eyebrow}</span>
          <h1 use:fitTitle={card.title}>{card.title}</h1>
          <p class="meta">
            {#if card.genres.length}<span>{card.genres.slice(0, 2).map(genreRu).join(', ')}</span>{:else}<span>{card.kind === 'series' ? 'Сериал' : 'Фильм'}</span>{/if}
            {#if card.year}<span>{card.year}</span>{/if}
            {#if card.rating}<b class="rating"><Icon name="star" size={13} /> {card.rating.toFixed(1)}</b>{/if}
          </p>
          <p class="description" class:empty={!overview}>{overview}</p>
          <div class="actions">
            {#if selected.card?.resume}<a class="btn primary" href="/watch/{selected.card.resume.fileId}"><Icon name="play" size={18} /> Продолжить</a>{:else}<a class="btn primary" {href}><Icon name="play" size={18} /> Смотреть</a>{/if}
            <a class="btn glass more" {href}>Подробнее</a>
            <button class="btn glass round" class:on={wanted} aria-pressed={wanted} aria-label={wanted ? 'В планах' : 'Хочу'} title={wanted ? 'В планах' : 'Хочу'} onclick={want}><Icon name={wanted ? 'check' : 'plus'} size={18} /> <span class="lbl">{wanted ? 'В планах' : 'Хочу'}</span></button>
            <button class="btn glass round" aria-label="Не интересно" title="Не интересно" onclick={nope}><Icon name="ban" size={22} /></button>
          </div>
        </div>{/key}</div>
      </div>
    </div>
    {#if items.length > 1}
      <div class="foot">
        <div class="dots" role="tablist" aria-label="Кадры">
          {#each items as it, i (it.card?.id ?? it.ext?.id)}
            <button role="tab" aria-selected={i === at} class:on={i === at} aria-label={it.card?.title ?? it.ext?.title} onclick={() => { manual = true; void go(i); save(); }}>
              {#if i === at && !manual && motionOn()}{#key `${at}:${hovering}`}<i class="fill" class:hold={hovering || !visible} style:--ms="{SLIDE_MS}ms"></i>{/key}{/if}
            </button>
          {/each}
        </div>
        <div class="arrows">
          <button aria-label="Предыдущий кадр" onclick={() => { manual = true; void go(prevAt, -1); save(); }}><Icon name="back" size={18} /></button>
          <button aria-label="Следующий кадр" onclick={() => { manual = true; void go(nextAt, 1); save(); }}><Icon name="right" size={18} /></button>
        </div>
      </div>
    {/if}
  </section>
{/if}

<style>
  /* «Витрина»: большая карточка по центру, соседние видны по бокам (пока листаешь - выезжают с нужной стороны), под ней
     точки с полоской времени и стрелки. Размеры растут вместе с экраном. */
  .showcase { --cw: min(62vw, 1240px); --ch: min(calc(var(--cw) * .5), 66svh); --gap: clamp(14px, 1.8vw, 26px); position: relative; padding: 8px 0 4px; overflow: hidden; touch-action: pan-y; }
  .track { position: relative; width: var(--cw); height: var(--ch); margin: 0 auto; animation: track-in .9s cubic-bezier(.22, 1, .36, 1) both; }
  @keyframes track-in { from { opacity: 0; transform: translateY(14px) scale(.975); } }
  .card { position: absolute; inset: 0; overflow: hidden; border-radius: clamp(22px, 2.2vw, 32px); background: #0e0e10; isolation: isolate; box-shadow: 0 34px 80px -34px #000, 0 0 0 1px #ffffff1c; }
  .stage { position: absolute; inset: 0; z-index: -2; }
  /* Два слоя: прозрачность меняется плавно и без пересчёта вёрстки; новый кадр медленно «оседает». */
  .backdrop { position: absolute; inset: 0; width: 100%; height: 100%; max-width: none; object-fit: cover; object-position: center 30%; opacity: 0; transition: opacity .9s cubic-bezier(.4, 0, .2, 1); will-change: opacity; }
  .backdrop.on { opacity: 1; animation: settle 10s cubic-bezier(.22, 1, .36, 1) both; }
  @keyframes settle { from { transform: scale(1.07); } }
  .shade { position: absolute; inset: 0; z-index: -1; pointer-events: none; background: linear-gradient(0deg, #000 0%, #000e 14%, #000a 34%, #0003 56%, transparent 74%), linear-gradient(180deg, #0005, transparent 24%); }

  /* Соседние карточки: тусклые, видны краем. */
  .peek { position: absolute; top: 0; width: var(--cw); height: 100%; padding: 0; border: 0; overflow: hidden; border-radius: clamp(22px, 2.2vw, 32px); background: #0e0e10; cursor: pointer; opacity: .5; transition: opacity .3s, transform .45s cubic-bezier(.22, 1, .36, 1); animation: peek-in .8s cubic-bezier(.22, 1, .36, 1) both; }
  .peek.l { right: calc(100% + var(--gap)); }
  .peek.r { left: calc(100% + var(--gap)); }
  .peek img { width: 100%; height: 100%; max-width: none; object-fit: cover; object-position: center 30%; filter: brightness(.7); }
  .peek::after { content: ''; position: absolute; inset: 0; background: linear-gradient(0deg, #000a, transparent 60%); }
  @media (hover: hover) { .peek:hover { opacity: .72; } .peek.l:hover { transform: translateX(10px); } .peek.r:hover { transform: translateX(-10px); } }
  @keyframes peek-in { from { opacity: 0; transform: translateX(calc(var(--dir, 1) * 70px)); } }

  .copies { position: absolute; inset: auto 0 0; display: grid; grid-template-columns: minmax(0, 1fr); padding: 0 clamp(20px, 3vw, 48px) clamp(22px, 2.6vw, 40px); }
  .copy { grid-area: 1 / 1; align-self: end; text-align: center; }
  .eyebrow { display: inline-block; padding: 4px 12px; border-radius: 999px; background: #ffffff1f; font-size: 11px; font-weight: 700; letter-spacing: .12em; text-transform: uppercase; color: #ffffffe0; }
  h1 { font-size: clamp(34px, 4.7vw, 76px); line-height: .98; letter-spacing: -.04em; font-weight: 900; text-transform: uppercase; margin: 12px 0 10px; white-space: nowrap; text-overflow: ellipsis; text-shadow: 0 4px 34px #0009; overflow: hidden; padding-bottom: .06em; }
  .meta { display: flex; flex-wrap: wrap; justify-content: center; align-items: center; gap: 6px 10px; font-size: 14px; font-weight: 600; color: #ffffffc8; margin: 0; }
  .meta span + span::before { content: '•'; margin-right: 10px; opacity: .45; }
  .rating { display: inline-flex; align-items: center; gap: 4px; padding: 2px 9px; border-radius: 7px; border: 1px solid #ffffff66; font-weight: 700; font-size: 13px; }
  .description { max-width: 64ch; margin: 12px auto 0; font-size: 15px; line-height: 1.5; color: #ffffffb8; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .description.empty { display: none; }
  .actions { display: flex; justify-content: center; gap: 10px; flex-wrap: wrap; margin-top: 18px; }
  .actions .btn { height: 48px; padding: 0 26px; border-radius: 999px; font-weight: 700; font-size: 15px; gap: 9px; border: 0; transition: transform .25s cubic-bezier(.22, 1, .36, 1), background .2s, box-shadow .25s; }
  .actions .btn:active { transform: scale(.96); }
  .actions .btn.primary { min-width: 150px; background: #fff; color: #0a0a0a; }
  .actions .btn.primary:hover { transform: translateY(-2px); box-shadow: 0 14px 34px -12px #fff9; background: #fff; }
  .actions .btn.round { width: 48px; padding: 0; }
  .actions .btn.glass { background: #ffffff26; color: #fff; box-shadow: inset 0 0 0 1px #ffffff2e; }
  .actions .btn.glass:hover { background: #ffffff38; transform: translateY(-2px); }
  .actions .btn.glass.on { box-shadow: inset 0 0 0 1px #fff; }
  .actions .lbl { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

  .foot { position: relative; width: var(--cw); margin: 16px auto 0; display: flex; justify-content: center; align-items: center; min-height: 40px; }
  .dots { display: flex; align-items: center; gap: 7px; }
  .dots button { position: relative; width: 8px; height: 8px; padding: 0; border: 0; border-radius: 99px; background: #ffffff45; cursor: pointer; overflow: hidden; transition: width .45s cubic-bezier(.22, 1, .36, 1), background .3s; }
  .dots button.on { width: 34px; background: #ffffff45; }
  /* Полоска в активной точке заполняется за то время, что показывается слайд (на наведении - стоит). */
  .fill { position: absolute; inset: 0; border-radius: inherit; background: #fff; transform-origin: 0 50%; animation: fill var(--ms) linear both; }
  .fill.hold { animation-play-state: paused; }
  .dots button.on:not(:has(.fill)) { background: #fff; }
  @keyframes fill { from { transform: scaleX(0); } }
  .arrows { position: absolute; right: 0; top: 50%; display: flex; gap: 10px; transform: translateY(-50%); }
  .arrows button { display: grid; place-items: center; width: 40px; height: 40px; padding: 0; border: 1.5px solid #ffffff70; border-radius: 50%; background: none; color: #fff; cursor: pointer; transition: background .2s, border-color .2s, transform .25s cubic-bezier(.22, 1, .36, 1); }
  .arrows button:hover { background: #ffffff1f; border-color: #fff; }
  .arrows button:active { transform: scale(.9); }
  :global(.no-motion) .backdrop { transition: none; animation: none; }

  /* Планшет и телефон: карточка уже и выше, соседние выглядывают по краям, стрелок нет (листается свайпом). */
  @media (max-width: 1000px) {
    .showcase { --cw: min(84vw, 760px); --ch: clamp(320px, calc(var(--cw) * .66), 520px); }
    .arrows { display: none; }
  }
  @media (max-width: 640px) {
    .showcase { --cw: calc(100vw - 56px); --ch: clamp(420px, calc(var(--cw) * 1.32), 600px); --gap: 10px; padding-top: 4px; }
    .card, .peek { border-radius: 26px; }
    h1 { font-size: clamp(28px, 9.4vw, 40px); white-space: normal; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; text-wrap: balance; margin: 10px 0 8px; }
    .meta { font-size: 12.5px; }
    .description, .more { display: none; }
    .actions { flex-wrap: nowrap; gap: 8px; margin-top: 14px; }
    .actions .btn { height: 46px; }
    .actions .btn.primary { min-width: 138px; padding: 0 22px; font-size: 16px; }
    .actions .btn.round { width: 46px; }
    .foot { margin-top: 12px; min-height: 22px; }
  }
</style>
