<script lang="ts">
  // Полоса раздела «Каталог» (разделы S1, выбор владельца 23.11): «Всё» - весь мир кино (чарты IMDb, поиск, франшизы),
  // «Фильмы», «Сериалы», «Аниме» - на Nova и в мире по типу. Одна и та же на всех страницах каталога.
  let { on }: { on: 'all' | 'movies' | 'series' | 'anime' } = $props();
  const ITEMS: [typeof on, string, string][] = [
    ['all', '/discover', 'Всё'],
    ['movies', '/?tab=movies', 'Фильмы'],
    ['series', '/?tab=series', 'Сериалы'],
    ['anime', '/?tab=anime', 'Аниме'],
  ];
</script>

<nav class="cstrip" aria-label="Каталог">
  {#each ITEMS as [id, to, label] (id)}
    <a href={to} class:on={on === id} aria-current={on === id ? 'page' : undefined}>{label}</a>
  {/each}
</nav>

<style>
  /* Как прежняя полоса разделов главной: текстом, выбранный подчёркнут, липнет под шапкой. */
  .cstrip {
    position: sticky;
    top: calc(var(--header) + env(safe-area-inset-top));
    z-index: 45;
    display: flex;
    align-items: center;
    gap: clamp(17px, 2vw, 28px);
    height: 46px;
    padding: 0 var(--gutter);
    overflow-x: auto;
    scrollbar-width: none;
    background: #000;
  }
  .cstrip::-webkit-scrollbar { display: none; }
  .cstrip a { flex: none; padding: 8px 0 6px; border-bottom: 3px solid transparent; color: #8e8e93; font-size: 20px; font-weight: 700; letter-spacing: -0.01em; white-space: nowrap; text-decoration: none; transition: color 0.15s; }
  .cstrip a:hover { color: #fff; }
  .cstrip a.on { color: #fff; border-bottom-color: #fff; }
</style>
