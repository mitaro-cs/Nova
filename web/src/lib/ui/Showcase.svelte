<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { heroSet, placeholder } from '$lib/ui/art';
  import { fitTitle } from '$lib/ui/fitTitle';
  import { getDetail, peekDetail } from '$lib/discover/details';
  import { genreRu } from '$lib/genres';
  import { motionOn } from '$lib/motion';
  import { upcomingYear } from '$lib/title/upcoming';
  import { toggle, toggleMark } from '$lib/marks';
  import { localized } from '$lib/localized.svelte';
  import type { Card, ExtCard, ExtDetail } from '$lib/types';
  import type { Component } from 'svelte';
  import { untrack } from 'svelte';
  import Icon from './Icon.svelte';

  type Item = { card?: Card; ext?: ExtCard };
  let { items }: { items: Item[] } = $props();

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
  // Трейлер заиграл: название уходит в левый нижний угол, лишний текст прячется.
  let trailerOn = $state(false);
  $effect(() => {
    void at;
    trailerOn = false;
  });
  let visible = $state(true);
  let root = $state<HTMLElement>();
  let restored = false;
  const n = $derived(items.length);
  const selected = $derived(items[Math.min(at, items.length - 1)]);
  // Подробности фильма из ленты: берём из кэша сразу (без пустого кадра при переключении), свежие - из `fetched`.
  let fetched = $state<Record<string, ExtDetail>>({});
  const extId = $derived(selected?.ext?.id);
  const detail = $derived<ExtDetail | null>(extId ? (fetched[extId] ?? peekDetail(extId)) : null);
  const card = $derived(selected?.card ?? detail?.card ?? selected?.ext);
  const href = $derived(selected?.card ? `/title/${selected.card.id}` : `/ext/${selected?.ext?.id}`);
  const overview = $derived(localized(selected?.card?.overview ?? detail?.overview));
  // Свежие подробности - и у центральной карточки, и у соседей: в кэше страницы может лежать старая версия (до выбора
  // кадра хостом или пока сервер ещё подбирал кадр), и соседи показывали не тот баннер, что потом в центре (25.11).
  const nearIds = $derived([...new Set([extId, ...items.map((it, i) => (it.ext && Math.abs(relOf(i)) <= 2 ? it.ext.id : null))].filter((x): x is string => !!x))]);
  const loading = new Set<string>();
  $effect(() => {
    const ids = nearIds;
    let live = true;
    const timers: number[] = [];
    for (const id of ids) {
      const have = untrack(() => fetched[id]);
      if (have && !have.refreshing) continue;
      if (loading.has(id)) continue;
      loading.add(id);
      let tries = 0;
      const load = () => getDetail(id).then((r) => { if (!live) { loading.delete(id); return; } fetched[id] = r; if (r.refreshing && ++tries < 12) timers.push(window.setTimeout(load, 3000)); else loading.delete(id); }, () => loading.delete(id));
      load();
    }
    return () => { live = false; timers.forEach(clearTimeout); for (const id of ids) loading.delete(id); };
  });

  // Карусель - настоящая лента: у каждой карточки свой кадр (в полном размере), положение - `--rel` (0 - по центру,
  // ±1 - соседи по бокам). Смена слайда меняет только `--rel`, а сдвиг делает CSS-переход: все карточки едут вместе,
  // ничего не подменяется и не «прыгает». Рисуются карточки от -2 до +2 (дальше - за экраном).
  const keyOf = (it: Item) => (it.card ? `c${it.card.id}` : `x${it.ext?.id}`);
  const relOf = (i: number) => {
    let d = (((i - at) % n) + n) % n;
    if (d > n / 2) d -= n;
    return d;
  };
  const slots = $derived(items.map((it, i) => ({ it, i, r: relOf(i), key: keyOf(it) })).filter((s) => Math.abs(s.r) <= 2));
  /** Кадр карточки: свой, из ленты - если уже получен, иначе постер. */
  function artOf(it: Item): { src: string | null; set: string | undefined } {
    if (it.card) {
      const src = it.card.backdrop ?? it.card.backdropSmall ?? it.card.poster ?? null;
      return { src, set: it.card.backdrop ? heroSet(it.card.backdrop, it.card.backdropSmall) : undefined };
    }
    // Пока свежие подробности не пришли - постер, а не старый кадр из кэша (он мог быть «не тем»).
    const d = fetched[it.ext!.id] ?? (peekDetail(it.ext!.id)?.refreshing ? null : peekDetail(it.ext!.id));
    const src = d?.backdrop ?? d?.gallery?.[0] ?? it.ext!.poster ?? null;
    return { src, set: d?.backdrop || d?.gallery?.[0] ? heroSet(src!, null) : undefined };
  }
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
  /** Переключить слайд: сначала кадр готов - потом лента едет. Новая смена в пути просто меняет цель сдвига. */
  async function go(i: number) {
    const to = ((i % n) + n) % n;
    if (switching || to === at) return;
    switching = true;
    try {
      await ready(to);
      at = to;
    } finally {
      switching = false;
    }
  }
  function keys(e: KeyboardEvent) {
    hold();
    if (n < 2 || (e.target instanceof HTMLElement && e.target.closest('input,textarea'))) return;
    if (e.key === 'ArrowRight') void go(at + 1);
    else if (e.key === 'ArrowLeft') void go(at - 1);
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
    if (manual || hovering || !visible || n < 2 || !motionOn()) return;
    const timer = setInterval(() => { if (document.visibilityState === 'visible') void go(at + 1); }, SLIDE_MS);
    return () => clearInterval(timer);
  });
  // Следующий слайд готовим заранее: описание и кадр - пока показывается этот.
  $effect(() => {
    if (manual || !visible || n < 2) return;
    const next = (at + 1) % n;
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
    if (e.pointerType !== 'touch' || !touchStart || n < 2) return;
    const dx = e.clientX - touchStart.x, dy = e.clientY - touchStart.y;
    touchStart = null;
    if (Math.abs(dx) > 50 && Math.abs(dx) > Math.abs(dy) * 1.2) { void go(at + (dx < 0 ? 1 : -1)); save(); }
  }
  function save() { try { sessionStorage.setItem('nova-showcase', JSON.stringify({ id: String(selected?.card?.id ?? selected?.ext?.id), manual })); } catch { /* приватный режим */ } }
  function pick(i: number) {
    manual = true;
    void go(i);
    save();
  }

  // Надпись над названием.
  const ext = $derived(selected?.ext);
  const eyebrow = $derived(
    ext && (ext.soon || upcomingYear(ext.release)) ? 'Скоро'
    : selected?.card?.resume ? 'Вы смотрели'
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
  <section class="showcase" aria-label="Рекомендуем" bind:this={root} onpointerenter={() => hovering = true} onpointerleave={() => hovering = false} onpointerdown={hold} onpointerup={swipe} onpointercancel={() => touchStart = null} onkeydown={keys}>
    <div class="track">
      {#each slots as s (s.key)}
        {@const art = artOf(s.it)}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="slot" class:side={s.r !== 0} style:--rel={s.r} style:z-index={10 - Math.abs(s.r)} style:opacity={Math.abs(s.r) > 1 ? 0 : 1} style:background={placeholder(s.it.card?.title ?? s.it.ext?.title ?? 'Nova')} in:fade|global={{ duration: 450 }} aria-hidden={s.r !== 0 ? 'true' : undefined} onclick={() => s.r !== 0 && pick(s.i)}>
          <div class="art-fallback" aria-hidden="true"><i>{s.it.card?.kind==='series'||s.it.ext?.kind==='series'?'СЕРИАЛ':'ФИЛЬМ'}</i><b>{s.it.card?.title ?? s.it.ext?.title}</b></div>
          {#if art.src}<img class="backdrop" src={art.src} srcset={art.set} sizes="(max-width: 640px) 100vw, 62vw" alt="" decoding="async" fetchpriority={s.r === 0 ? 'high' : 'auto'} onerror={(event) => { const img=event.currentTarget as HTMLImageElement; const fallback=s.it.card?.poster ?? s.it.ext?.poster; if(fallback && img.dataset.fallback !== fallback && img.getAttribute('src') !== fallback){img.dataset.fallback=fallback;img.removeAttribute('srcset');img.src=fallback;}else img.style.visibility='hidden'; }} />{/if}
          {#if Trailer && s.r === 0}<Trailer card={s.it.card} ext={s.it.ext} bind:active={trailerOn} />{/if}
          <div class="shade"></div>
          <div class="dim"></div>
          {#if s.r === 0}
            <div class="copies">{#key `${card.id}:${trailerOn}`}<div class="copy" class:mini={trailerOn} in:fly|global={{ y: 14, duration: trailerOn ? 500 : 650, delay: trailerOn ? 120 : 380, easing: cubicOut }} out:fade|global={{ duration: 120 }}>
              <span class="eyebrow">{eyebrow}</span>
              <h1 use:fitTitle={card.title}>{card.title}</h1>
              <p class="meta">
                {#if card.genres.length}<span>{card.genres.slice(0, 2).map(genreRu).join(', ')}</span>{:else}<span>{card.kind === 'series' ? 'Сериал' : 'Фильм'}</span>{/if}
                {#if card.year}<span>{card.year}</span>{/if}
                {#if card.rating}<b class="rating"><Icon name="star" size={13} /> {card.rating.toFixed(1)}</b>{/if}
              </p>
              <p class="description" class:blank={!overview}>{overview}</p>
              <div class="actions">
                {#if selected.card?.resume}<a class="btn primary" href="/watch/{selected.card.resume.fileId}"><Icon name="play" size={18} /> Продолжить</a>{:else}<a class="btn primary" {href}><Icon name="play" size={18} /> Смотреть</a>{/if}
                <a class="btn glass more" {href}>Подробнее</a>
                <button class="btn glass round" class:on={wanted} aria-pressed={wanted} aria-label={wanted ? 'В планах' : 'Хочу'} title={wanted ? 'В планах' : 'Хочу'} onclick={want}><Icon name={wanted ? 'check' : 'plus'} size={18} /> <span class="lbl">{wanted ? 'В планах' : 'Хочу'}</span></button>
                <button class="btn glass round" aria-label="Не интересно" title="Не интересно" onclick={nope}><Icon name="ban" size={22} /></button>
              </div>
            </div>{/key}</div>
          {/if}
        </div>
      {/each}
      {#if n > 1}
        <button class="arrow l" aria-label="Предыдущий кадр" onclick={() => pick(at - 1)}><Icon name="back" size={24} /></button>
        <button class="arrow r" aria-label="Следующий кадр" onclick={() => pick(at + 1)}><Icon name="right" size={24} /></button>
      {/if}
    </div>
    {#if n > 1}
      <div class="foot">
        <div class="dots" role="tablist" aria-label="Кадры">
          {#each items as it, i (it.card?.id ?? it.ext?.id)}
            <button role="tab" aria-selected={i === at} class:on={i === at} aria-label={it.card?.title ?? it.ext?.title} onclick={() => pick(i)}>
              {#if i === at && !manual && motionOn()}{#key `${at}:${hovering}`}<i class="fill" class:hold={hovering || !visible} style:--ms="{SLIDE_MS}ms"></i>{/key}{/if}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </section>
{/if}

<style>
  /* «Витрина»: большая карточка по центру, соседние видны по бокам. Это настоящая лента: карточки едут вместе, пока
     листаешь. Под ней точки с полоской времени; стрелки - стеклянные круги по краям центральной карточки. */
  .showcase { --pitch: calc(var(--cw) + var(--gap)); --cw: min(62vw, 1240px); --ch: min(calc(var(--cw) * .5), 66svh); --gap: clamp(14px, 1.8vw, 26px); --round: clamp(22px, 2.2vw, 32px); position: relative; padding: 8px 0 4px; overflow: hidden; touch-action: pan-y; }
  .track { position: relative; width: var(--cw); height: var(--ch); margin: 0 auto; animation: track-in .9s cubic-bezier(.22, 1, .36, 1) both; }
  @keyframes track-in { from { opacity: 0; transform: translateY(14px) scale(.975); } }
  .slot { position: absolute; inset: 0; overflow: hidden; border-radius: var(--round); background: #0e0e10; isolation: isolate; box-shadow: 0 34px 80px -34px #000; transform: translateX(calc(var(--rel) * var(--pitch))); transition: transform .95s cubic-bezier(.22, 1, .36, 1), opacity .6s ease; will-change: transform; }
  .art-fallback { position:absolute; inset:0; z-index:-3; display:grid; place-content:center; justify-items:center; gap:14px; padding:10%; text-align:center; background:radial-gradient(ellipse at 50% 35%,#ffffff18,transparent 60%),linear-gradient(0deg,#050506bc,transparent 85%); }
  .art-fallback i { font-size:11px; font-style:normal; font-weight:700; letter-spacing:.24em; color:#ffffffa8; }
  .art-fallback b { max-width:18ch; font-size:clamp(24px,5vw,64px); line-height:1.08; text-wrap:balance; text-shadow:0 4px 24px #000; }
  .slot.side { cursor: pointer; }
  .backdrop { position: absolute; inset: 0; z-index: -2; width: 100%; height: 100%; max-width: none; object-fit: cover; object-position: center 30%; }
  .shade { position: absolute; inset: 0; z-index: -1; pointer-events: none; background: linear-gradient(0deg, #000 0%, #000e 14%, #000a 34%, #0003 56%, transparent 74%), linear-gradient(180deg, #0005, transparent 24%); }
  /* Соседние карточки темнее центральной; светлеют, когда становятся центральной. */
  .dim { position: absolute; inset: 0; z-index: 1; pointer-events: none; background: #000; opacity: 0; transition: opacity .95s cubic-bezier(.22, 1, .36, 1); }
  .slot.side .dim { opacity: .55; }
  @media (hover: hover) { .slot.side:hover .dim { opacity: .38; } }

  .copies { position: absolute; inset: auto 0 0; z-index: 2; display: grid; grid-template-columns: minmax(0, 1fr); padding: 0 clamp(20px, 3vw, 48px) clamp(22px, 2.6vw, 40px); }
  .copy { grid-area: 1 / 1; align-self: end; text-align: center; }
  .eyebrow { display: inline-block; padding: 4px 12px; border-radius: 999px; background: #ffffff1f; font-size: 11px; font-weight: 700; letter-spacing: .12em; text-transform: uppercase; color: #ffffffe0; }
  h1 { font-size: clamp(34px, 4.7vw, 76px); line-height: 1.22; letter-spacing: -.04em; font-weight: 900; text-transform: uppercase; margin: 6px 0 4px; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; padding-block: .09em; }
  .meta { display: flex; flex-wrap: wrap; justify-content: center; align-items: center; gap: 6px 10px; font-size: 14px; font-weight: 600; color: #ffffffc8; margin: 0; }
  .meta span + span::before { content: '•'; margin-right: 10px; opacity: .45; }
  .rating { display: inline-flex; align-items: center; gap: 4px; padding: 2px 9px; border-radius: 7px; background: #ffffff26; font-weight: 700; font-size: 13px; }
  .description { max-width: 64ch; margin: 12px auto 0; font-size: 15px; line-height: 1.5; color: #ffffffb8; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .description.blank { display: none; }
  /* Играет трейлер: название и кнопки - в левом нижнем углу, остального текста нет, кадр открыт. */
  .copy.mini { text-align: left; max-width: min(52%, 560px); }
  .copy.mini .eyebrow, .copy.mini .description { display: none; }
  .copy.mini h1 { font-size: clamp(22px, 2.5vw, 40px); margin: 0 0 8px; }
  .copy.mini .meta { justify-content: flex-start; }
  .copy.mini .actions { justify-content: flex-start; margin-top: 12px; }
  .copy.mini .actions .btn { height: 42px; font-size: 14px; padding: 0 22px; }
  .copy.mini .actions .btn.round { width: 42px; padding: 0; }
  .copy.mini .actions .btn.primary { min-width: 0; }
  .copies:has(.mini) { padding-right: 0; }
  .actions { display: flex; justify-content: center; gap: 10px; flex-wrap: wrap; margin-top: 18px; }
  .actions .btn { height: 48px; padding: 0 26px; border-radius: 999px; font-weight: 700; font-size: 15px; gap: 9px; border: 0; transition: transform .25s cubic-bezier(.22, 1, .36, 1), background .2s, box-shadow .25s; }
  .actions .btn:active { transform: scale(.96); }
  .actions .btn.primary { min-width: 150px; background: #fff; color: #0a0a0a; }
  .actions .btn.primary:hover { transform: translateY(-2px); box-shadow: 0 14px 34px -12px #fff9; background: #fff; }
  .actions .btn.round { width: 48px; padding: 0; }
  .actions .btn.glass { background: #ffffff1a; color: #ffffffd9; }
  .actions .btn.glass:hover { background: #ffffff26; color: #fff; transform: translateY(-2px); }
  .actions .btn.glass.on { background: #fff; color: #0a0a0a; }
  .actions .lbl { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

  /* Стрелки: стеклянные круги на краях центральной карточки, появляются, когда мышь над каруселью. */
  .arrow { position: absolute; top: 50%; z-index: 20; display: grid; place-items: center; width: 46px; height: 46px; margin-top: -23px; padding: 0; border: 0; border-radius: 50%; background: #0000004d; -webkit-backdrop-filter: blur(10px); backdrop-filter: blur(10px); color: #ffffffcc; cursor: pointer; outline: none; opacity: 0; transition: opacity .35s ease, transform .4s cubic-bezier(.22, 1, .36, 1), background .25s, color .25s; }
  .arrow.l { left: 14px; }
  .arrow.r { right: 14px; }
  .showcase:hover .arrow, .arrow:focus-visible { opacity: .85; }
  .arrow:hover { opacity: 1; background: #ffffff33; color: #fff; transform: scale(1.06); }
  .arrow:active { transform: scale(.92); transition-duration: .1s; }

  .foot { position: relative; width: var(--cw); margin: 16px auto 0; display: flex; justify-content: center; align-items: center; min-height: 22px; }
  .dots { display: flex; align-items: center; gap: 7px; }
  .dots button { position: relative; width: 8px; height: 8px; padding: 0; border: 0; border-radius: 99px; background: #ffffff45; cursor: pointer; overflow: hidden; transition: width .45s cubic-bezier(.22, 1, .36, 1), background .3s; }
  .dots button.on { width: 34px; background: #ffffff45; }
  /* Полоска в активной точке заполняется за то время, что показывается слайд (на наведении - стоит). */
  .fill { position: absolute; inset: 0; border-radius: inherit; background: #fff; transform-origin: 0 50%; animation: fill var(--ms) linear both; }
  .fill.hold { animation-play-state: paused; }
  .dots button.on:not(:has(.fill)) { background: #fff; }
  @keyframes fill { from { transform: scaleX(0); } }
  :global(.no-motion) .slot, :global(.no-motion) .dim { transition: none; }

  /* Планшет и телефон: карточка уже и выше, соседние выглядывают по краям, стрелок нет (листается свайпом). */
  @media (max-width: 1000px) {
    .showcase { --cw: min(84vw, 760px); --ch: clamp(320px, calc(var(--cw) * .66), 520px); }
    .arrow { display: none; }
  }
  @media (max-width: 640px) {
    /* Телефон (26.11 «баннеры не очень высокие»): карточка почти квадратная - не больше половины экрана. */
    .showcase { --cw: calc(100vw - 48px); --ch: clamp(340px, min(calc(var(--cw) * 1.12), 52svh), 470px); --gap: 10px; --round: 26px; padding-top: 4px; }
    h1 { font-size: clamp(28px, 9.4vw, 40px); white-space: normal; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; text-wrap: balance; margin: 10px 0 8px; }
    .meta { font-size: 12.5px; }
    .description, .more { display: none; }
    .copy.mini { max-width: 100%; }
    .actions .btn { flex: none; }
    .actions { flex-wrap: nowrap; gap: 8px; margin-top: 14px; }
    .actions .btn { height: 46px; }
    .actions .btn.primary { min-width: 138px; padding: 0 22px; font-size: 16px; }
    .actions .btn.round { width: 46px; }
    .foot { margin-top: 12px; min-height: 22px; }
  }
</style>
