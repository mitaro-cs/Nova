<script lang="ts">
  import { page } from '$app/state';
  import { fade } from 'svelte/transition';
  import { api } from '$lib/api';
  import { getDetail } from '$lib/discover/details';
  import { artSet, artW, placeholder } from '$lib/ui/art';
  import { localized } from '$lib/localized.svelte';
  import { upcomingYear } from '$lib/title/upcoming';
  import Icon from '$lib/ui/Icon.svelte';
  import Loading from '$lib/ui/Loading.svelte';
  import type { ExtCard, ExtDetail } from '$lib/types';

  // Серия фильмов: слева кадр выбранного фильма (фон страницы чёрный, кадр плавно растворяется в него), под ним
  // постер, год, оценка и описание; справа - все фильмы серии. Наведёшь на фильм - слева он же.
  type Group = { name: string; items: ExtCard[] };
  const id = $derived(page.params.id ?? '');
  let groups = $state<Group[] | null>(null), error = $state(''), chosen = $state('');
  let detail = $state<ExtDetail | null>(null);
  const group = $derived(groups?.[Math.max(0, Math.min(groups.length - 1, Number(page.url.searchParams.get('group')) || 0))] ?? null);
  const current = $derived(group?.items.find(x => x.id === chosen) ?? group?.items[0] ?? null);
  const href = (f: ExtCard) => (f.local ? `/title/${f.local}` : `/ext/${f.id}`);
  $effect(() => {
    const want = id; groups = null; error = ''; chosen = want;
    api<Group[]>(`/api/franchise?imdb=${encodeURIComponent(want)}`).then(g => { if (id === want) groups = g.filter(x => x.items.length > 1); }, () => { if (id === want) error = 'Не удалось загрузить подборку.'; });
  });
  $effect(() => {
    const want = current?.id; detail = null; if (!want) return;
    let live = true, timer = 0, attempts = 0;
    const load = () => getDetail(want).then(d => { if (!live) return; detail = d; if (d.refreshing && attempts++ < 8) timer = window.setTimeout(load,2500); }, () => {});
    load(); return () => { live = false; clearTimeout(timer); };
  });
</script>
{#if error}<div class="message"><p>{error}</p><a class="btn" href="/ext/{id}">Вернуться к фильму</a></div>
{:else if groups === null}<div class="message"><Loading label="Собираем фильмы серии" /></div>
{:else if !group || !current}<div class="message"><p>Подборка для этого фильма пока не найдена.</p><a class="btn" href="/ext/{id}">Вернуться к фильму</a></div>
{:else}
  <main class="collection">
    <div class="stage" aria-hidden="true">
      <div class="frame" style:background={placeholder(current.title)}>
        {#key detail?.backdrop}{#if detail?.backdrop}<img in:fade|global={{ duration: 600 }} out:fade|global={{ duration: 600 }} src={artW(detail.backdrop, 1920)} srcset={artSet(detail.backdrop)} sizes="54vw" alt="" decoding="async" />{/if}{/key}
      </div>
      <div class="veil"></div>
      {#key current.id}
        <div class="film" in:fade|global={{ duration: 450 }}>
          <span class="poster" style:background={placeholder(current.title)}>{#if current.poster}<img src={artW(detail?.posterLarge ?? current.poster, 640)} alt="" decoding="async" />{/if}</span>
          <div class="text">
            <b class="name">{current.title}</b>
            <span class="meta">{[upcomingYear(current.release) ? 'Скоро' : '', current.year, current.rating ? `★ ${current.rating.toFixed(1)}` : ''].filter(Boolean).join('  ·  ')}</span>
            {#if detail?.overview}<p class="ov">{localized(detail.overview, '')}</p>{/if}
          </div>
        </div>
      {/key}
    </div>
    <section class="content" aria-label="Фильмы серии">
      <a class="back" href="/ext/{id}"><Icon name="back" size={16} /> К фильму</a>
      <h1>{group.name}</h1><p class="sub">Все фильмы серии</p>
      <ol>
        {#each group.items as film, i (film.id)}
          <li><a href={href(film)} class:active={film.id === current.id} onmouseenter={() => chosen = film.id} onfocus={() => chosen = film.id}>
            <span class="art" style:background={placeholder(film.title)}>{#if film.poster}<img src={artW(film.poster,640)} alt="" loading="lazy" />{/if}</span>
            <span class="label"><b>{film.title}</b><small>{upcomingYear(film.release) ? `Скоро · ${film.year ?? ''}` : (film.year ?? '')}</small></span><span class="num">{String(i+1).padStart(2,'0')}</span>
          </a></li>
        {/each}
      </ol>
    </section>
  </main>
{/if}
<style>
  /* Серия фильмов: чёрный фон, большой кадр слева мягко растворяется в него (без резких краёв), справа - список. */
  .message{min-height:60vh;display:grid;place-content:center;text-align:center;gap:20px;padding:24px}
  .collection{position:relative;display:grid;grid-template-columns:60% minmax(0,1fr);min-height:100svh;color:#fff;background:#050506;isolation:isolate}
  .stage{position:sticky;top:0;height:100svh;overflow:hidden}
  /* Кадр занимает почти весь экран; снизу и справа растворяется длинным плавным переходом. */
  .frame{position:absolute;inset:0 0 14% 0;-webkit-mask-image:linear-gradient(to bottom,#000 0%,#000 52%,#000c 66%,#0008 78%,#0004 90%,transparent 100%);mask-image:linear-gradient(to bottom,#000 0%,#000 52%,#000c 66%,#0008 78%,#0004 90%,transparent 100%)}
  .frame img{position:absolute;inset:0;width:100%;height:100%;max-width:none;object-fit:cover;object-position:center 25%}
  .veil{position:absolute;inset:0;background:linear-gradient(270deg,#050506 0%,#050506e6 6%,#050506b8 14%,#05050680 24%,#05050640 36%,#05050618 48%,transparent 62%),linear-gradient(180deg,#0000008c,transparent 20%)}
  .film{position:absolute;left:0;right:0;bottom:0;display:flex;align-items:flex-end;gap:clamp(20px,2.2vw,34px);padding:0 clamp(24px,3vw,56px) clamp(28px,5vh,60px)}
  .poster{flex:none;width:clamp(130px,12vw,210px);aspect-ratio:2/3;border-radius:16px;overflow:hidden;box-shadow:0 28px 60px -20px #000e}
  .poster img{width:100%;height:100%;object-fit:cover}
  .text{display:grid;gap:10px;min-width:0}
  .name{font-size:clamp(30px,3.6vw,58px);line-height:1.05;font-weight:800;letter-spacing:-.03em;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;padding-bottom:.08em}
  .meta{color:#e6e9ea;font-size:clamp(15px,1.15vw,19px);font-weight:600}
  .ov{margin:0;color:#d9dddd;font-size:clamp(15px,1.15vw,18px);line-height:1.6;display:-webkit-box;-webkit-line-clamp:4;line-clamp:4;-webkit-box-orient:vertical;overflow:hidden;max-width:64ch}
  .content{min-width:0;min-height:100svh;padding:calc(var(--header) + 30px) clamp(20px,3vw,50px) 35px 0}
  .back{display:inline-flex;align-items:center;gap:6px;color:#d4d9d9;font-size:14px}
  h1{font-size:clamp(30px,3.2vw,50px);margin:20px 0 4px;line-height:1.1;letter-spacing:-.03em}
  .sub{color:#c0caca;margin:0 0 20px;font-size:14px}
  ol{list-style:none;margin:0;padding:0;display:grid;grid-template-columns:minmax(0,1fr);gap:10px}
  li{min-width:0}
  li a{display:grid;grid-template-columns:60px minmax(0,1fr) 30px;gap:14px;align-items:center;min-width:0;padding:9px 12px 9px 9px;background:#ffffff0d;border-radius:16px;transition:background .25s,transform .3s var(--ease)}
  li a:hover,li a:focus-visible,li a.active{background:#ffffff1f;transform:translateX(-3px)}
  .art{aspect-ratio:2/3;overflow:hidden;border-radius:9px}
  .art img{width:100%;height:100%;object-fit:cover}
  .label{display:grid;gap:4px;min-width:0}
  .label b{font-size:15px;line-height:1.3;overflow-wrap:anywhere}
  .label small{font-size:13px;color:#cbd3d3}
  .num{font-size:13px;color:#aab2b2;text-align:right}
  @media(max-width:760px){.collection{display:block}.stage{position:relative;height:60svh}.frame{inset:0 0 24% 0}.film{padding:0 16px 16px}.poster{width:92px}.ov{display:none}.content{padding:8px 16px 90px}ol{gap:8px}li a{grid-template-columns:48px minmax(0,1fr);gap:10px}.num{display:none}.label b{font-size:14px}}
</style>
