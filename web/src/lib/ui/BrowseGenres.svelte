<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IconName } from './icons';
  const genres: { label: string; chart: string; icon: IconName; hue: number }[] = [
    {label:'Комедии',hue:50,chart:'comedy_movies',icon:'sparkle'}, {label:'Фантастика',hue:212,chart:'scifi_movies',icon:'globe'},
    {label:'Триллеры',hue:165,chart:'thriller_movies',icon:'eye'}, {label:'Боевики',hue:8,chart:'action_movies',icon:'play'},
    {label:'Драмы',hue:268,chart:'drama_movies',icon:'film'}, {label:'Анимация',hue:192,chart:'animation_movies',icon:'star'},
  ];
  const franchises = ['Гарри Поттер','Властелин колец','Звёздные войны','Матрица','Мстители'];
</script>
<section class="browse" aria-label="Жанры"><h2>Под настроение</h2><div class="genres">{#each genres as genre}<a href="/discover?chart={genre.chart}" style:background="linear-gradient(160deg, hsl({genre.hue} 34% 22%), hsl({genre.hue + 18} 24% 8%))"><Icon name={genre.icon} size={27} /><b>{genre.label}</b><span>Открыть подборку <Icon name="right" size={13} /></span></a>{/each}</div></section>
<section class="browse" aria-label="Франшизы"><h2>Любимые киновселенные</h2><div class="franchises">{#each franchises as title}<a href="/search?q={encodeURIComponent(title)}"><Icon name="film" size={18} />{title}<Icon name="right" size={14} /></a>{/each}</div></section>
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
    min-height: 176px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    justify-content: space-between;
    padding: 22px 18px;
    border: 1px solid var(--line);
    background: var(--surface-2);
    border-radius: max(18px, calc(24 * var(--rw)));
    transition: transform 0.2s, background 0.2s;
  }
  .genres a:hover {
    transform: translateY(-4px);
    background: var(--surface-3);
  }
  .genres b {
    font-size: clamp(15px, 1.8vw, 23px);
  }
  .genres span {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 11px;
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
      padding: 18px 14px;
      gap: 14px;
    }
    .franchises a {
      padding: 12px 16px;
    }
  }
</style>