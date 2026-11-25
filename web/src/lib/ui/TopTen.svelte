<script lang="ts">
  import type { Card, ExtCard } from '$lib/types';
  import { artW, placeholder } from './art';
  import { fadeIn } from './fade';

  // «Топ-10 Nova за неделю» (главная «Витрина», 23.11): огромные контурные цифры, как на странице режиссёра, и постер
  // рядом. Лента прокручивается вбок; на широком окне видно пять-шесть мест.
  // Тот же ряд - «Топ-10 мира» в Каталоге (25.11): своё название, без счёта зрителей.
  // `chart` (26.11 «одинаковый дизайн - не круто»): второй топ в Каталоге - чартом, списком в две колонки: место, постер, год,
  // жанр и оценка IMDb - как хит-парад, а не вторая такая же лента постеров.
  let { items, title = 'Топ-10 Nova за неделю', id = 'top10-h', chart = false }: { items: { card?: Card; ext?: ExtCard; viewers?: number }[]; title?: string; id?: string; chart?: boolean } = $props();
  const list = $derived(
    items.map((it) => {
      const c = (it.card ?? it.ext)!;
      return { key: it.card ? `t${it.card.id}` : `x${it.ext!.id}`, title: c.title, poster: it.card?.posterLarge ?? c.poster, href: it.card ? `/title/${it.card.id}` : `/ext/${it.ext!.id}`, viewers: it.viewers, year: c.year, rating: c.rating, genre: c.genres?.[0] ?? null };
    }),
  );
  const people = (n: number) => `${n} ${n % 10 === 1 && n % 100 !== 11 ? 'зритель' : n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20) ? 'зрителя' : 'зрителей'}`;
</script>

{#if chart}
<section class="chart" aria-labelledby={id}>
  <h2 {id}>{title}</h2>
  <ol>
    {#each list as it, i (it.key)}
      <li>
        <a href={it.href}>
          <span class="cn" aria-hidden="true">{i + 1}</span>
          <span class="cp" style:background={placeholder(it.title)}>{#if it.poster}<img use:fadeIn src={artW(it.poster, 185)} alt="" loading="lazy" decoding="async" width="54" height="81" />{/if}</span>
          <span class="ct"><b>{it.title}</b><small>{[it.year, it.genre].filter(Boolean).join(' · ')}</small></span>
          {#if it.rating}<span class="cr">★ {it.rating.toFixed(1).replace('.', ',')}</span>{/if}
        </a>
      </li>
    {/each}
  </ol>
</section>
{:else}
<section class="top10" aria-labelledby={id}>
  <h2 {id}>{title}</h2>
  <ol>
    {#each list as it, i (it.key)}
      <li style:--i={i}>
        <span class="n" aria-hidden="true">{i + 1}</span>
        <a href={it.href} title={it.viewers ? `${it.title} · ${people(it.viewers)}` : it.title} style:background={placeholder(it.title)}>
          {#if it.poster}<img use:fadeIn src={artW(it.poster, 342)} alt={it.title} loading="lazy" decoding="async" width="228" height="342" />{:else}<span>{it.title}</span>{/if}
        </a>
      </li>
    {/each}
  </ol>
</section>
{/if}

<style>
  /* Чарт: две колонки по пять мест (порядок сверху вниз), на узком - одна колонка. */
  .chart { margin: 8px 0 30px; }
  .chart h2 { margin: 0 0 14px; font-size: clamp(19px, 1.6vw, 24px); font-weight: 700; }
  .chart ol { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); grid-template-rows: repeat(5, auto); grid-auto-flow: column; gap: 8px 28px; margin: 0; padding: 0; list-style: none; }
  .chart a { display: grid; grid-template-columns: 52px 54px minmax(0, 1fr) auto; align-items: center; gap: 14px; padding: 8px 14px 8px 4px; border-radius: 16px; color: inherit; text-decoration: none; transition: background 0.2s; }
  .chart a:hover, .chart a:focus-visible { background: #ffffff0d; }
  .chart li + li a { box-shadow: none; }
  .cn { color: #000; font-size: 44px; font-weight: 800; line-height: 1; letter-spacing: -0.05em; text-align: center; -webkit-text-stroke: 1.5px #ffffffb8; paint-order: stroke fill; font-variant-numeric: tabular-nums; }
  .cp { width: 54px; aspect-ratio: 2 / 3; border-radius: 9px; overflow: hidden; }
  .cp img { width: 100%; height: 100%; object-fit: cover; }
  .ct { display: grid; gap: 3px; min-width: 0; }
  .ct b { overflow: hidden; font-size: 15px; font-weight: 700; text-overflow: ellipsis; white-space: nowrap; }
  .ct small { overflow: hidden; color: #8c8c96; font-size: 12.5px; text-overflow: ellipsis; white-space: nowrap; }
  .cr { padding: 4px 10px; border-radius: 999px; background: #ffffff12; font-size: 12.5px; font-weight: 700; font-variant-numeric: tabular-nums; }
  @media (max-width: 760px) {
    .chart ol { grid-template-columns: minmax(0, 1fr); grid-template-rows: none; grid-auto-flow: row; gap: 4px; }
    .chart a { grid-template-columns: 40px 46px minmax(0, 1fr) auto; gap: 10px; }
    .cn { font-size: 34px; }
    .cp { width: 46px; }
  }
  /* Цифра - своя колонка слева, низом по постеру (не режется краем ленты); под мышью ничего не прыгает - только светлая
     рамка (просьба владельца 25.11 «не должно двигаться»). Лента едет только вбок. */
  .top10 { margin: 8px 0 30px; }
  .top10 h2 { margin: 0 0 14px; font-size: clamp(19px, 1.6vw, 24px); font-weight: 700; }
  .top10 ol { display: flex; gap: 10px; margin: 0; padding: 2px 0 6px; list-style: none; overflow-x: auto; overflow-y: hidden; overscroll-behavior: contain; scrollbar-width: none; }
  .top10 ol::-webkit-scrollbar { display: none; }
  .top10 li { flex: none; display: flex; align-items: flex-end; }
  .top10 .n { flex: none; width: 0.64em; margin-right: -0.06em; text-align: right; color: #000; font-size: clamp(100px, 10vw, 170px); font-weight: 800; line-height: 0.74; letter-spacing: -0.06em; -webkit-text-stroke: 2px #ffffffb8; paint-order: stroke fill; pointer-events: none; user-select: none; }
  .top10 li:nth-child(10) .n { width: 1.1em; letter-spacing: -0.1em; }
  .top10 a { position: relative; z-index: 1; flex: none; width: var(--pw, 170px); display: grid; place-items: center; aspect-ratio: 2 / 3; border-radius: 14px; overflow: hidden; color: #fff; font-weight: 700; text-align: center; box-shadow: 0 18px 40px -20px #000; }
  .top10 a::after { content: ''; position: absolute; inset: 0; border-radius: inherit; box-shadow: inset 0 0 0 2px #ffffffb0; opacity: 0; transition: opacity 0.2s; pointer-events: none; }
  .top10 a:hover::after, .top10 a:focus-visible::after { opacity: 1; }
  .top10 img { width: 100%; height: 100%; object-fit: cover; }
</style>
