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
  import Icon from './Icon.svelte';

  let { items }: { items: { card?: Card; ext?: ExtCard }[] } = $props();
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
  /** Переключить слайд: сначала всё готово - потом текст и кадр меняются вместе. */
  async function go(i: number) {
    const to = ((i % items.length) + items.length) % items.length;
    if (switching || to === at) return;
    switching = true;
    try {
      await ready(to);
      at = to;
    } finally {
      switching = false;
    }
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
    const timer = setInterval(() => { if (document.visibilityState === 'visible') void go(at + 1); }, 7000);
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
    if (Math.abs(dx) > 50 && Math.abs(dx) > Math.abs(dy) * 1.2) { void go(at + (dx < 0 ? 1 : -1)); save(); }
  }
  function save() { try { sessionStorage.setItem('nova-showcase', JSON.stringify({ id: String(selected?.card?.id ?? selected?.ext?.id), manual })); } catch { /* приватный режим */ } }

  // «Кинозал+»: надпись над названием и цвет кадра - по фильму (у своих - цвет постера).
  const accent = '#ffffff';
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
  $effect(() => {
    document.documentElement.style.setProperty('--hero-tint', accent);
    return () => document.documentElement.style.removeProperty('--hero-tint');
  });
</script>

{#if card}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <section class="showcase" aria-label="Рекомендуем" style:--acc={accent} bind:this={root} onpointerenter={() => hovering = true} onpointerleave={() => hovering = false} onpointerdown={hold} onpointerup={swipe} onpointercancel={() => touchStart = null} onkeydown={() => hold()}>
    <div class="stage" aria-hidden="true">
      {#each layers as shot, i (i)}
        {#if shot}<img class="backdrop" class:on={front === i} src={shot.src} srcset={shot.set} sizes="100vw" alt="" decoding="async" fetchpriority={i === front ? 'high' : 'auto'} />{/if}
      {/each}
    </div>
    <div class="shade"></div>
    <div class="copies">{#key card.id}<div class="copy" in:fly|global={{ y: 12, duration: 600, delay: 320, easing: cubicOut }} out:fade|global={{ duration: 260 }}>
      <span class="eyebrow">{eyebrow}</span>
      <h1 use:fitTitle={card.title}>{card.title}</h1>
      <p class="meta">
        {#if card.rating}<span class="rating"><Icon name="star" size={14} /> {card.rating.toFixed(1)}</span>{/if}
        {#if card.year}<span>{card.year}</span>{/if}
        {#if card.genres.length}<span>{card.genres.slice(0, 2).map(genreRu).join(', ')}</span>{:else}<span>{card.kind === 'series' ? 'Сериал' : 'Фильм'}</span>{/if}
      </p>
      <p class="description" class:empty={!overview}>{overview}</p>
      <div class="actions">
        {#if selected.card?.resume}<a class="btn primary" href="/watch/{selected.card.resume.fileId}"><Icon name="play" size={18} /> Продолжить</a>{:else}<a class="btn primary" {href}><Icon name="play" size={18} /> Смотреть</a>{/if}
        <button class="btn glass" class:on={wanted} aria-pressed={wanted} onclick={want}><Icon name={wanted ? 'check' : 'plus'} size={18} /> {wanted ? 'В планах' : 'Хочу'}</button>
      </div>
    </div>{/key}</div>
    {#if items.length > 1}
      <div class="dots" role="tablist" aria-label="Кадры">
        {#each items as it, i (it.card?.id ?? it.ext?.id)}
          <button role="tab" aria-selected={i === at} class:on={i === at} aria-label={it.card?.title ?? it.ext?.title} onclick={() => { manual = true; void go(i); save(); }}></button>
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  /* «Кинозал+» (alpha 0.5): кадр почти на весь экран, слева - надпись цветом фильма, крупное название,
     оценка, описание и две капсулы; справа внизу - точки. Затемнение только слева и снизу. */
  .showcase { position: relative; min-height: clamp(520px, 78svh, 860px); isolation: isolate; display: flex; align-items: flex-end; padding: calc(var(--header) + 36px) var(--gutter) clamp(48px, 9vh, 100px); overflow: hidden; }
  /* Кадр - на всю ширину, слева его плавно гасит затемнение (края кадра не видно). */
  .stage { position: absolute; inset: 0; z-index: -2; }
  /* Два слоя: прозрачность меняется плавно и без пересчёта вёрстки. */
  .backdrop { position: absolute; inset: 0; width: 100%; height: 100%; max-width: none; object-fit: cover; object-position: center 30%; opacity: 0; transition: opacity .9s cubic-bezier(.4, 0, .2, 1); will-change: opacity; }
  .backdrop.on { opacity: 1; }
  .shade { position: absolute; inset: 0; z-index: -1; pointer-events: none; }
  .shade { background: linear-gradient(90deg, var(--tone-dark) 0%, color-mix(in srgb, var(--tone-dark) 92%, transparent) 14%, color-mix(in srgb, var(--tone-dark) 70%, transparent) 30%, color-mix(in srgb, var(--tone-dark) 30%, transparent) 50%, transparent 72%), linear-gradient(0deg, var(--tone-dark) 0%, color-mix(in srgb, var(--tone-dark) 50%, transparent) 22%, transparent 50%); }
  .copies { display: grid; max-width: min(46vw, 620px); }
  .copy { grid-area: 1 / 1; align-self: end; }
  .eyebrow { display: inline-flex; align-items: center; gap: 10px; font-size: 12px; font-weight: 800; letter-spacing: .14em; text-transform: uppercase; color: color-mix(in srgb, var(--acc) 65%, #fff); }
  .eyebrow::before { content: ''; width: 22px; height: 2px; background: currentColor; }
  h1 { font-size: clamp(38px, 4.6vw, 72px); line-height: 1.02; letter-spacing: -.035em; font-weight: 800; margin: 14px 0; white-space: nowrap; text-overflow: ellipsis; text-shadow: 0 2px 30px #0006; overflow: hidden; padding-bottom: .08em; }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 16px; font-size: 14px; font-weight: 500; color: #c9c4bb; margin: 0; }
  .rating { display: inline-flex; align-items: center; gap: 5px; color: #f5c518; font-weight: 800; }
  /* Место под три строки занято всегда: когда описание приходит позже, ничего не прыгает. */
  .description { max-width: 60ch; min-height: 4.8em; font-size: 16px; line-height: 1.6; color: #ddd8d0; display: -webkit-box; -webkit-line-clamp: 3; line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden; margin: 16px 0 0; }
  .actions { display: flex; gap: 12px; flex-wrap: wrap; margin-top: 26px; }
  .actions .btn { height: 52px; padding: 0 26px; border-radius: 999px; font-weight: 700; font-size: 15px; gap: 10px; border: 0; transition: transform .15s ease, background .2s; }
  .actions .btn:active { transform: scale(.96); }
  .actions .btn.primary { background: #fff; color: #0a0a0a; }
  /* Стекло - только на маленькой капсуле. */
  .actions .btn.glass { background: #ffffff1c; color: #fff; box-shadow: inset 0 0 0 1px #ffffff2e; backdrop-filter: blur(14px); }
  .actions .btn.glass:hover { background: #ffffff2a; }
  .actions .btn.glass.on { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--acc) 70%, #fff); }
  .dots { position: absolute; right: var(--gutter); bottom: clamp(80px, 12vh, 140px); display: flex; gap: 6px; }
  .dots button { width: 8px; height: 8px; padding: 0; border: 0; border-radius: 99px; background: #ffffff55; cursor: pointer; transition: width .3s cubic-bezier(.2,.8,.2,1), background .3s; }
  .dots button.on { width: 28px; background: #fff; }
  :global(.no-motion) .backdrop { transition: none; }
  /* Телефон: кадр сверху почти целиком, всё по центру под ним. */
  @media (max-width: 640px) {
    .showcase { min-height: 0; touch-action: pan-y; padding: calc(75vw - 40px) var(--gutter) 50px; justify-content: center; text-align: center; }
    .stage { inset: 0 0 auto 0; width: 100%; aspect-ratio: 4 / 3; -webkit-mask-image: linear-gradient(#000 60%, transparent); mask-image: linear-gradient(#000 60%, transparent); }
    .shade { background: linear-gradient(0deg, var(--tone-dark) 30%, #0000 70%), linear-gradient(180deg, #0006, #0000 30%); }
    .copies { max-width: 100%; }
    .eyebrow { justify-content: center; font-size: 11px; }
    h1 { font-size: clamp(32px, 10vw, 44px); margin: 8px 0 10px; }
    .meta { justify-content: center; font-size: 13px; }
    .description { display: none; }
    .actions { justify-content: center; margin-top: 20px; }
    .actions .btn { height: 48px; padding: 0 22px; }
    .dots { left: 0; right: 0; bottom: 18px; justify-content: center; }
  }
</style>
