<script lang="ts">
  import { artW } from './art';
  import Icon from './Icon.svelte';

  // «Жанры» (25.11, просьба владельца: было «Под настроение» с цветными плитками и значками). Плитка жанра - тёмная, название
  // крупно, справа веером три постера лучших фильмов этого жанра из того, что есть у вас и в ленте IMDb.
  type Film = { poster: string | null; genres: string[]; rating: number | null };
  let { pool = [] }: { pool?: Film[] } = $props();
  const genres: { label: string; chart: string; stems: string[] }[] = [
    { label: 'Комедии', chart: 'comedy_movies', stems: ['комед'] },
    { label: 'Фантастика', chart: 'scifi_movies', stems: ['фантаст'] },
    { label: 'Триллеры', chart: 'thriller_movies', stems: ['триллер'] },
    { label: 'Боевики', chart: 'action_movies', stems: ['боевик'] },
    { label: 'Драмы', chart: 'drama_movies', stems: ['драм'] },
    { label: 'Анимация', chart: 'animation_movies', stems: ['мульт', 'анимац'] },
  ];
  const fans = $derived(
    genres.map((g) => {
      const seen = new Set<string>();
      const posters = pool
        .filter((f) => f.poster && f.genres.some((x) => g.stems.some((s) => x.toLowerCase().includes(s))))
        .sort((a, b) => (b.rating ?? 0) - (a.rating ?? 0))
        .map((f) => f.poster!)
        .filter((p) => !seen.has(p) && !!seen.add(p))
        .slice(0, 3);
      return { ...g, posters };
    }),
  );
  // Киновселенные - своей страницей порядка просмотра (`/franchise/<id IMDb первого фильма>`), а не поиском.
  const franchises = [
    { title: 'Гарри Поттер', imdb: 'tt0241527' }, { title: 'Властелин колец', imdb: 'tt0120737' },
    { title: 'Звёздные войны', imdb: 'tt0076759' }, { title: 'Матрица', imdb: 'tt0133093' }, { title: 'Мстители', imdb: 'tt0848228' },
  ];
</script>
<section class="browse" aria-label="Жанры"><h2>Жанры</h2><div class="genres">{#each fans as g (g.chart)}<a href="/discover?chart={g.chart}"><span class="fan" aria-hidden="true">{#each g.posters as p, i (p)}<img src={artW(p, 185)} alt="" loading="lazy" decoding="async" style:--i={i - (g.posters.length - 1) / 2} />{/each}</span><b>{g.label}</b><span class="go">Открыть <Icon name="right" size={14} /></span></a>{/each}</div></section>
<section class="browse" aria-label="Франшизы"><h2>Любимые киновселенные</h2><div class="franchises">{#each franchises as f (f.imdb)}<a href="/franchise/{f.imdb}"><Icon name="film" size={18} />{f.title}<Icon name="right" size={14} /></a>{/each}</div></section>
<style>
  .browse {
    margin: 30px 0 38px;
  }
  h2 {
    margin: 0 0 max(14px, calc(20 * var(--rw)));
    font-size: max(17px, calc(20 * var(--rw)));
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .genres {
    display: grid;
    grid-template-columns: repeat(6, minmax(0, 1fr));
    gap: 12px;
  }
  /* Плитки и капсулы - как чипы на плите Rewind: фон --surface-2, тонкая линия, без толстой рамки. */
  .genres a {
    min-width: 0;
    min-height: 228px;
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 4px;
    justify-content: flex-end;
    padding: 20px;
    border: 1px solid #ffffff14;
    background: linear-gradient(180deg, #17171a, #0d0d0f);
    border-radius: max(22px, calc(26 * var(--rw)));
    transition: border-color 0.25s;
  }
  .genres a:hover { border-color: #ffffff47; }
  /* Веер постеров: средний прямо, крайние наклонены; под мышью веер раскрывается (только transform). */
  .fan { position: absolute; top: 16px; left: 50%; width: 0; height: 120px; }
  .fan img {
    position: absolute;
    left: -36px;
    top: 0;
    width: 72px;
    max-width: none;
    aspect-ratio: 2 / 3;
    object-fit: cover;
    border-radius: 9px;
    box-shadow: 0 12px 26px -10px #000;
    transform: translateX(calc(var(--i) * 40px)) translateY(calc(var(--i) * var(--i) * 8px)) rotate(calc(var(--i) * 9deg));
    transition: transform 0.35s var(--ease, ease);
  }
  .fan img:nth-child(2) { z-index: 1; }
  .genres a:hover .fan img { transform: translateX(calc(var(--i) * 52px)) translateY(calc(var(--i) * var(--i) * 6px - 4px)) rotate(calc(var(--i) * 12deg)); }
  .genres b {
    position: relative;
    z-index: 2;
    font-size: clamp(18px, 1.7vw, 24px);
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .go {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 13px;
  }
  .franchises {
    display: flex;
    gap: 10px;
    overflow-x: auto;
    padding-bottom: 8px;
  }
  .franchises a {
    display: flex;
    align-items: center;
    gap: 9px;
    flex: none;
    padding: 12px 20px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--surface-2);
    font-size: 14px;
    font-weight: 600;
    transition: background 0.2s;
  }
  .franchises a:hover {
    background: var(--surface-3);
  }
  @media (max-width: 1000px) {
    .genres {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  @media (max-width: 600px) {
    .genres {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 10px;
    }
    .genres a {
      min-height: 196px;
      padding: 16px 14px;
    }
    .fan img { width: 64px; left: -32px; }
    .franchises a {
      padding: 12px 16px;
    }
  }
</style>