<script lang="ts">
  import { getDetail, peekDetail } from '$lib/discover/details';
  import type { ExtCard } from '$lib/types';
  import { artW, placeholder } from '$lib/ui/art';
  import { fadeIn } from '$lib/ui/fade';

  // Жанры плитками (Каталог «Афиша мира», 25.11): кадр самого популярного фильма жанра, название крупно; нажатие
  // открывает всю подборку жанра. Пока кадра нет - три постера жанра рядом (размер плитки тот же, ничего не прыгает).
  type Row = { key: string; title: string; items: ExtCard[] };
  let { rows }: { rows: Row[] } = $props();
  let frames = $state<Record<string, string | null>>({});
  const lead = (r: Row) => r.items.find((c) => c.poster) ?? r.items[0];

  $effect(() => {
    let live = true;
    for (const r of rows) {
      const c = lead(r);
      if (!c || r.key in frames) continue;
      const have = peekDetail(c.id)?.backdrop;
      if (have) {
        frames[r.key] = have;
        continue;
      }
      getDetail(c.id).then((x) => live && (frames[r.key] = x.backdrop ?? null), () => {});
    }
    return () => (live = false);
  });
  const name = (t: string) => t.replace(/ сериалы$/, '');
</script>

{#if rows.length}
  <section class="gt" aria-labelledby="gt-h">
    <div class="hd"><h2 id="gt-h">Жанры</h2></div>
    <ul>
      {#each rows as r (r.key)}
        {@const c = lead(r)}
        <li>
          <a href="/discover?chart={r.key}" style:background={placeholder(r.title)}>
            {#if frames[r.key]}<img class="bd" use:fadeIn src={artW(frames[r.key], 640)} alt="" loading="lazy" decoding="async" />
            {:else}<span class="three" aria-hidden="true">{#each r.items.filter((x) => x.poster).slice(0, 3) as p (p.id)}<img src={artW(p.poster, 185)} alt="" loading="lazy" decoding="async" />{/each}</span>{/if}
            <b>{name(r.title)}</b>
            {#if c}<small>{c.title}</small>{/if}
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .gt { margin: 8px 0 30px; }
  .hd h2 { margin: 0 0 14px; font-size: clamp(19px, 1.6vw, 24px); font-weight: 700; }
  ul { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 260px), 1fr)); gap: 14px; margin: 0; padding: 0; list-style: none; }
  a { position: relative; isolation: isolate; display: block; height: clamp(120px, 10vw, 160px); border-radius: 18px; overflow: hidden; color: #fff; text-decoration: none; }
  a::after { content: ''; position: absolute; inset: 0; z-index: 0; background: linear-gradient(0deg, #000000d9 0%, #00000033 70%); }
  a::before { content: ''; position: absolute; inset: 0; z-index: 2; border-radius: inherit; box-shadow: inset 0 0 0 2px #ffffffb0; opacity: 0; transition: opacity 0.2s; }
  a:hover::before, a:focus-visible::before { opacity: 1; }
  .bd { position: absolute; inset: 0; width: 100%; max-width: none; height: 100%; object-fit: cover; transition: transform 0.5s var(--ease, ease); }
  a:hover .bd { transform: scale(1.04); }
  .three { position: absolute; inset: 0; display: grid; grid-template-columns: repeat(3, 1fr); }
  .three img { width: 100%; max-width: none; height: 100%; object-fit: cover; }
  b { position: absolute; z-index: 1; left: 18px; bottom: 30px; font-size: clamp(19px, 1.7vw, 24px); font-weight: 800; letter-spacing: -0.02em; }
  small { position: absolute; z-index: 1; left: 18px; right: 18px; bottom: 12px; color: #ffffffa8; font-size: 12.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
