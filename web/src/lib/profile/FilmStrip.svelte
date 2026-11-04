<script lang="ts">
  import { count } from '$lib/fmt';
  import type { Card } from '$lib/types';
  import { artW, placeholder } from '$lib/ui/art';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import LockBox from './LockBox.svelte';

  // Лента просмотренного: плёнка с кадрами фильмов, сама едет влево, под курсором стоит. Кадры - ссылки на фильмы.
  // Ширина набора кратна 16 px (период перфорации), чтобы начало ленты сходилось без шва.
  let { items, personId, total, hidden = false }: { items: Card[]; personId: number; total: number; hidden?: boolean } = $props();

  const STEP = 88; // кадр 80 px + промежуток 8 px
  // Кадров в наборе - чётное число, не меньше 12: набор повторяется, пока не заполнит ширину.
  const set = $derived.by(() => {
    if (!items.length) return [] as Card[];
    const n = Math.max(12, Math.ceil(items.length / 2) * 2);
    return Array.from({ length: n }, (_, i) => items[i % items.length]);
  });
  const width = $derived(set.length * STEP);
</script>

<section class="reel" aria-labelledby="reel-h">
  <div class="head">
    <h2 id="reel-h">Лента просмотренного</h2>
    {#if !hidden && items.length}<a href="/people/{personId}/watched?kind=movie">{count(total, 'фильм', 'фильма', 'фильмов')} · все <Icon name="right" size={14} /></a>{/if}
  </div>
  {#if hidden}
    <LockBox />
  {:else if items.length}
    <div class="strip">
      <div class="vp">
        <div class="roll" style:--w="{width}px" style:--d="{set.length * 3}s">
          {#each [0, 1] as copy (copy)}
            <div class="set" style:width="{width}px" aria-hidden={copy === 1}>
              <div class="perf"></div>
              <div class="frames">
                {#each set as c, i (i)}
                  <a class="fm" href="/title/{c.id}" title={c.title} tabindex={copy === 1 ? -1 : undefined} style:background={c.backdropSmall || c.poster ? undefined : placeholder(c.title)}>
                    {#if c.backdropSmall || c.poster}<img use:fadeIn src={artW((c.backdropSmall ?? c.poster)!, 160)} alt={copy === 1 ? '' : c.title} loading="lazy" decoding="async" width="80" height="54" />{/if}
                    {#if c.year}<small>{c.year}</small>{/if}
                  </a>
                {/each}
              </div>
              <div class="perf"></div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</section>

<style>
  .reel {
    margin-top: 30px;
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px 12px;
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .head a {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    font-size: 13px;
    transition: color 0.2s;
  }
  .head a:hover {
    color: var(--text);
  }
  .strip {
    padding: 16px 0;
    border: 1px solid var(--line);
    border-radius: 16px;
    background: #050506;
    overflow: hidden;
  }
  .vp {
    overflow: hidden;
    -webkit-mask-image: linear-gradient(90deg, transparent, #000 8%, #000 92%, transparent);
    mask-image: linear-gradient(90deg, transparent, #000 8%, #000 92%, transparent);
  }
  .roll {
    display: flex;
    width: max-content;
    animation: roll var(--d) linear infinite;
    will-change: transform;
  }
  .strip:hover .roll,
  .strip:focus-within .roll {
    animation-play-state: paused;
  }
  .set {
    flex: none;
  }
  .perf {
    height: 7px;
    background: repeating-linear-gradient(90deg, #ffffff30 0 10px, transparent 10px 16px);
  }
  .frames {
    display: flex;
    padding: 7px 0;
  }
  .fm {
    position: relative;
    flex: none;
    width: 80px;
    height: 56px;
    margin-right: 8px;
    border-radius: 5px;
    overflow: hidden;
    background: #26262c;
    transition: transform 0.3s cubic-bezier(0.2, 0.9, 0.3, 1.4);
  }
  .fm:hover,
  .fm:focus-visible {
    z-index: 2;
    transform: translateY(-4px) scale(1.08);
  }
  .fm img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .fm small {
    position: absolute;
    left: 6px;
    bottom: 4px;
    color: #f4f4f6;
    font-size: 11px;
    font-weight: 700;
    text-shadow: 0 1px 6px #000;
  }
  @keyframes roll {
    to {
      transform: translate3d(calc(var(--w) * -1), 0, 0);
    }
  }
</style>
