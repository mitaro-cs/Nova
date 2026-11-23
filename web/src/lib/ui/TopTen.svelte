<script lang="ts">
  import type { Card, ExtCard } from '$lib/types';
  import { artW, placeholder } from './art';
  import { fadeIn } from './fade';

  // «Топ-10 Nova за неделю» (главная «Витрина», 23.11): огромные контурные цифры, как на странице режиссёра, и постер
  // рядом. Лента прокручивается вбок; на широком окне видно пять-шесть мест.
  let { items }: { items: { card?: Card; ext?: ExtCard; viewers: number }[] } = $props();
  const list = $derived(
    items.map((it) => {
      const c = (it.card ?? it.ext)!;
      return { key: it.card ? `t${it.card.id}` : `x${it.ext!.id}`, title: c.title, poster: it.card?.posterLarge ?? c.poster, href: it.card ? `/title/${it.card.id}` : `/ext/${it.ext!.id}`, viewers: it.viewers };
    }),
  );
  const people = (n: number) => `${n} ${n % 10 === 1 && n % 100 !== 11 ? 'зритель' : n % 10 >= 2 && n % 10 <= 4 && (n % 100 < 10 || n % 100 >= 20) ? 'зрителя' : 'зрителей'}`;
</script>

<section class="top10" aria-labelledby="top10-h">
  <h2 id="top10-h">Топ-10 Nova за неделю</h2>
  <ol>
    {#each list as it, i (it.key)}
      <li style:--i={i}>
        <span class="n" aria-hidden="true">{i + 1}</span>
        <a href={it.href} title="{it.title} · {people(it.viewers)}" style:background={placeholder(it.title)}>
          {#if it.poster}<img use:fadeIn src={artW(it.poster, 342)} alt={it.title} loading="lazy" decoding="async" width="228" height="342" />{:else}<span>{it.title}</span>{/if}
        </a>
      </li>
    {/each}
  </ol>
</section>

<style>
  .top10 { margin: 8px 0 30px; }
  h2 { margin: 0 0 14px; font-size: clamp(19px, 1.6vw, 24px); font-weight: 700; }
  ol { display: flex; gap: 14px; margin: 0; padding: 4px 0 8px; list-style: none; overflow-x: auto; scrollbar-width: none; }
  ol::-webkit-scrollbar { display: none; }
  li { position: relative; flex: none; width: calc(var(--pw, 170px) + clamp(44px, 4.4vw, 70px)); padding-left: clamp(44px, 4.4vw, 70px); }
  .n { position: absolute; left: -2px; bottom: 6px; z-index: 0; color: #000; font-size: clamp(110px, 11vw, 190px); font-weight: 800; line-height: 0.78; letter-spacing: -0.08em; -webkit-text-stroke: 2px #ffffffb0; pointer-events: none; }
  li:nth-child(10) .n { letter-spacing: -0.12em; }
  a { position: relative; z-index: 1; display: grid; place-items: center; aspect-ratio: 2 / 3; border-radius: 14px; overflow: hidden; color: #fff; font-weight: 700; text-align: center; box-shadow: 0 18px 40px -20px #000; transition: transform 0.3s var(--ease, ease); }
  a:hover { transform: translateY(-4px); }
  img { width: 100%; height: 100%; object-fit: cover; }
</style>
