<script lang="ts">
  import type { Card, ExtCard } from '$lib/types';
  import { artW, placeholder } from './art';
  import { fadeIn } from './fade';

  // «Топ-10 Nova за неделю» (главная «Витрина», 23.11): огромные контурные цифры, как на странице режиссёра, и постер
  // рядом. Лента прокручивается вбок; на широком окне видно пять-шесть мест.
  // Тот же ряд - «Топ-10 мира» в Каталоге (25.11): своё название, без счёта зрителей.
  let { items, title = 'Топ-10 Nova за неделю', id = 'top10-h' }: { items: { card?: Card; ext?: ExtCard; viewers?: number }[]; title?: string; id?: string } = $props();
  const list = $derived(
    items.map((it) => {
      const c = (it.card ?? it.ext)!;
      return { key: it.card ? `t${it.card.id}` : `x${it.ext!.id}`, title: c.title, poster: it.card?.posterLarge ?? c.poster, href: it.card ? `/title/${it.card.id}` : `/ext/${it.ext!.id}`, viewers: it.viewers };
    }),
  );
  const people = (n: number) => `${n} ${n % 10 === 1 && n % 100 !== 11 ? 'зритель' : n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20) ? 'зрителя' : 'зрителей'}`;
</script>

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

<style>
  /* Цифра - своя колонка слева, низом по постеру (не режется краем ленты); под мышью ничего не прыгает - только светлая
     рамка (просьба владельца 25.11 «не должно двигаться»). Лента едет только вбок. */
  .top10 { margin: 8px 0 30px; }
  h2 { margin: 0 0 14px; font-size: clamp(19px, 1.6vw, 24px); font-weight: 700; }
  ol { display: flex; gap: 10px; margin: 0; padding: 2px 0 6px; list-style: none; overflow-x: auto; overflow-y: hidden; overscroll-behavior: contain; scrollbar-width: none; }
  ol::-webkit-scrollbar { display: none; }
  li { flex: none; display: flex; align-items: flex-end; }
  .n { flex: none; width: 0.64em; margin-right: -0.06em; text-align: right; color: #000; font-size: clamp(100px, 10vw, 170px); font-weight: 800; line-height: 0.74; letter-spacing: -0.06em; -webkit-text-stroke: 2px #ffffffb8; paint-order: stroke fill; pointer-events: none; user-select: none; }
  li:nth-child(10) .n { width: 1.1em; letter-spacing: -0.1em; }
  a { position: relative; z-index: 1; flex: none; width: var(--pw, 170px); display: grid; place-items: center; aspect-ratio: 2 / 3; border-radius: 14px; overflow: hidden; color: #fff; font-weight: 700; text-align: center; box-shadow: 0 18px 40px -20px #000; }
  a::after { content: ''; position: absolute; inset: 0; border-radius: inherit; box-shadow: inset 0 0 0 2px #ffffffb0; opacity: 0; transition: opacity 0.2s; pointer-events: none; }
  a:hover::after, a:focus-visible::after { opacity: 1; }
  img { width: 100%; height: 100%; object-fit: cover; }
</style>
