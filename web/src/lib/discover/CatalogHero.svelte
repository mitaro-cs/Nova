<script lang="ts">
  import { getDetail, peekDetail } from '$lib/discover/details';
  import { localized } from '$lib/localized.svelte';
  import { toggleMark } from '$lib/marks';
  import { runtime } from '$lib/fmt';
  import type { ExtCard, ExtDetail } from '$lib/types';
  import { artSet, artW, placeholder } from '$lib/ui/art';
  import { fadeIn } from '$lib/ui/fade';
  import { fitTitle } from '$lib/ui/fitTitle';
  import Icon from '$lib/ui/Icon.svelte';
  import type { Snippet } from 'svelte';

  // Каталог «Афиша мира» (выбор владельца 25.11): самое популярное на IMDb сейчас - кадром на всю ширину, по очереди
  // пять первых мест (8 с, на наведении стоит, точки - выбор). Кадр, описание и длительность - из подробностей фильма
  // (`getDetail`, сервер качает кадр к себе); пока кадра нет - постер справа на цвете названия. Поиск IMDb - `tools`.
  let { items, label, tools }: { items: ExtCard[]; label: string; tools?: Snippet } = $props();
  const SLIDE_MS = 8000;
  let at = $state(0);
  let hold = $state(false);
  let details = $state<Record<string, ExtDetail>>({});
  const list = $derived(items.slice(0, 5));
  const cur = $derived(list[Math.min(at, list.length - 1)]);
  const d = $derived(cur ? (details[cur.id] ?? peekDetail(cur.id)) : null);
  const card = $derived(d?.card ?? cur);

  $effect(() => {
    let live = true;
    for (const c of list) {
      if (details[c.id]) continue;
      getDetail(c.id).then((r) => live && (details[c.id] = r), () => {});
    }
    return () => (live = false);
  });
  $effect(() => {
    if (list.length < 2 || hold || document.documentElement.classList.contains('no-motion')) return;
    void at;
    const t = setTimeout(() => (at = (at + 1) % list.length), SLIDE_MS);
    return () => clearTimeout(t);
  });
  const backdrop = (c: ExtCard) => (details[c.id] ?? peekDetail(c.id))?.backdrop ?? null;
  const meta = $derived(
    [card?.year, card?.kind === 'series' ? 'Сериал' : null, (card?.genres ?? []).slice(0, 2).join(', '), d?.runtime ? runtime(d.runtime) : null, card?.rating ? `IMDb ${card.rating.toFixed(1).replace('.', ',')}` : null]
      .filter(Boolean)
      .join(' · '),
  );
  // Своя отметка «Хочу» поверх карточки: карточки - данные ленты, их не трогаем.
  let own = $state<Record<string, ExtCard['mark']>>({});
  const mark = $derived(cur ? (cur.id in own ? own[cur.id] : card?.mark) : null);
  async function want() {
    if (!cur) return;
    const id = cur.id;
    const tmp = { id, mark: mark ?? null };
    own[id] = tmp.mark === 'want' ? null : 'want';
    await toggleMark(tmp, 'want');
    own[id] = tmp.mark;
  }
</script>

{#if cur}
  <section class="ch" aria-label={label} onmouseenter={() => (hold = true)} onmouseleave={() => (hold = false)}>
    <div class="art" aria-hidden="true">
      {#each list as c, i (c.id)}
        {@const b = backdrop(c)}
        <div class="slide" class:on={i === at} style:background={placeholder(c.title)}>
          {#if b}<img use:fadeIn src={artW(b, 1920)} srcset={artSet(b)} sizes="100vw" alt="" decoding="async" fetchpriority={i === 0 ? 'high' : 'auto'} />
          {:else if c.poster}<img class="po" use:fadeIn src={artW(c.poster, 640)} alt="" decoding="async" />{/if}
        </div>
      {/each}
    </div>
    <div class="veil" aria-hidden="true"></div>
    {#if tools}<div class="tools">{@render tools()}</div>{/if}
    <div class="text">
      <span class="eye">№{at + 1} на IMDb сейчас</span>
      <h1 use:fitTitle={card?.title ?? ''}>{card?.title}</h1>
      {#if meta}<p class="meta">{meta}</p>{/if}
      {#if d?.overview}<p class="ov">{localized(d.overview)}</p>{/if}
      <div class="btns">
        <a class="btn primary" href="/ext/{cur.id}"><Icon name="play" size={18} /> Смотреть</a>
        <button class="btn glass" aria-pressed={mark === 'want'} onclick={want}><Icon name={mark === 'want' ? 'bookmark' : 'plus'} size={18} /> {mark === 'want' ? 'В планах' : 'Хочу'}</button>
      </div>
    </div>
    {#if list.length > 1}
      <div class="dots" role="tablist" aria-label="Первые места">
        {#each list as c, i (c.id)}
          <button role="tab" aria-selected={i === at} aria-label="{i + 1}. {c.title}" class:on={i === at} onclick={() => (at = i)}><i style:animation-duration="{SLIDE_MS}ms" class:run={i === at && !hold}></i></button>
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  /* Кадр от края до края окна (поля `rw-page` гасим отрицательным отступом), высота - по экрану, но не выше 760. */
  .ch {
    position: relative;
    isolation: isolate;
    height: clamp(420px, min(74svh, 52vw), 760px);
    margin: 0 calc(-1 * var(--gutter)) 18px;
    overflow: hidden;
    background: #000;
  }
  .art,
  .slide,
  .veil {
    position: absolute;
    inset: 0;
  }
  .slide {
    opacity: 0;
    transition: opacity 0.9s ease;
  }
  .slide.on {
    opacity: 1;
  }
  .slide img {
    width: 100%;
    max-width: none;
    height: 100%;
    object-fit: cover;
    object-position: 50% 28%;
  }
  .slide.on img {
    animation: drift 9s linear both;
  }
  /* Нет кадра - постер справа, как обложка афиши. */
  .slide img.po {
    position: absolute;
    right: 8%;
    top: 12%;
    width: auto;
    height: 76%;
    aspect-ratio: 2 / 3;
    border-radius: 18px;
    box-shadow: 0 30px 80px -20px #000;
    animation: none;
  }
  @keyframes drift {
    from { transform: scale(1.06); }
    to { transform: scale(1); }
  }
  :global(.no-motion) .slide.on img { animation: none; }
  .veil {
    background:
      linear-gradient(90deg, #000000eb 0%, #00000099 34%, #00000000 68%),
      linear-gradient(0deg, var(--bg, #000) 0%, #00000000 42%);
  }
  .tools {
    position: absolute;
    z-index: 2;
    top: 18px;
    right: var(--gutter);
    left: var(--gutter);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .text {
    position: absolute;
    z-index: 2;
    left: var(--gutter);
    bottom: clamp(48px, 7vh, 84px);
    width: min(600px, calc(100% - 2 * var(--gutter)));
    display: grid;
    gap: 12px;
    justify-items: start;
  }
  .eye {
    padding: 6px 12px;
    border-radius: 999px;
    background: #ffffff21;
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }
  h1 {
    margin: 0;
    max-width: 100%;
    font-size: clamp(38px, 5.4vw, 76px);
    font-weight: 800;
    line-height: 1.02;
    letter-spacing: -0.045em;
    text-wrap: balance;
  }
  .meta {
    margin: 0;
    color: #ffffffcc;
    font-size: 15px;
  }
  .ov {
    margin: 0;
    color: #ffffffb3;
    font-size: 15px;
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .btns {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 6px;
  }
  .btn.glass {
    background: #ffffff1f;
    border-color: transparent;
    color: #fff;
  }
  .btn.glass[aria-pressed='true'] {
    background: #ffffff38;
  }
  .dots {
    position: absolute;
    z-index: 2;
    right: var(--gutter);
    bottom: clamp(48px, 7vh, 84px);
    display: flex;
    gap: 8px;
  }
  .dots button {
    width: 34px;
    height: 20px;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
    display: grid;
    align-items: center;
  }
  .dots button i {
    display: block;
    height: 4px;
    border-radius: 4px;
    background: linear-gradient(90deg, #fff 50%, #ffffff40 50%) right / 200% 100% no-repeat;
  }
  .dots button.on i {
    background-position: right;
  }
  .dots button i.run {
    animation: fill linear both;
  }
  .dots button.on i:not(.run) {
    background-position: left;
  }
  @keyframes fill {
    from { background-position: right; }
    to { background-position: left; }
  }
  @media (max-width: 640px) {
    .ch {
      height: clamp(460px, 78svh, 640px);
    }
    .veil {
      background: linear-gradient(0deg, var(--bg, #000) 4%, #000000b8 46%, #00000033 100%);
    }
    .slide img.po {
      right: 50%;
      transform: translateX(50%);
      top: 9%;
      height: 46%;
    }
    .tools {
      top: 12px;
    }
    .dots {
      left: var(--gutter);
      right: auto;
      bottom: 18px;
    }
    .text {
      bottom: 50px;
    }
  }
</style>
