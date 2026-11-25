<script lang="ts">
  import { page } from '$app/state';
  import { replaceState } from '$app/navigation';
  import { api, errorText, patch } from '$lib/api';
  import LockBox from '$lib/profile/LockBox.svelte';
  import Kassa from '$lib/profile/Kassa.svelte';
  import ProfileHead from '$lib/profile/ProfileHead.svelte';
  import ReviewsTab from '$lib/profile/ReviewsTab.svelte';
  import StatWidgets from '$lib/profile/StatWidgets.svelte';
  import { TABS, type ProfilePage, type ProfileTab } from '$lib/profile/types';
  import VisPick from '$lib/profile/VisPick.svelte';
  import WatchedTab from '$lib/profile/WatchedTab.svelte';
  import type { Card, ExtCard, VisLevel } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';
  import { toast } from '$lib/ui/toast.svelte';

  // Страница человека (макет владельца в Figma 21.11): слева липкий «Пульт» (на телефоне - карточка сверху), справа
  // «Моя киновселенная» и вкладки «Обзор», «Статистика», «Просмотрено», «Отзывы». Что человек закрыл (Статистика, Просмотрено, Отзывы),
  // сервер не отдаёт: вместо раздела замок «Скрыто владельцем». Своя страница - с переключателями «кто видит».
  let p = $state<ProfilePage | null>(null);
  let error = $state('');
  const id = $derived(Number(page.params.id));

  $effect(() => {
    const want = id;
    p = null;
    error = '';
    const tz = -new Date().getTimezoneOffset();
    api<ProfilePage>(`/api/members/${want}?tz=${tz}`).then((r) => want === id && (p = r), (e) => (error = errorText(e)));
  });

  // ---------- вкладки ----------
  const fromUrl = (): ProfileTab => {
    const q = page.url.searchParams.get('tab');
    return (TABS as string[]).includes(q ?? '') ? (q as ProfileTab) : 'home';
  };
  let tab = $state<ProfileTab>(fromUrl()); // выбранная
  let shown = $state<ProfileTab>(fromUrl()); // показанная: сменяется после того, как прежняя уехала
  let dir = $state<'r' | 'l' | 'n'>('n');
  let leaving = $state<'r' | 'l' | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function choose(t: ProfileTab) {
    if (t === tab) return;
    const d = TABS.indexOf(t) > TABS.indexOf(tab) ? 'r' : 'l';
    tab = t;
    try {
      replaceState(t === 'home' ? page.url.pathname : `?tab=${t}`, {});
    } catch {
      /* адрес не обновился - вкладка всё равно переключена */
    }
    clearTimeout(timer);
    leaving = d;
    timer = setTimeout(() => {
      dir = d;
      shown = t;
      leaving = null;
    }, 200);
  }

  const hidden = $derived(p?.hidden ?? { stats: false, watched: false, reviews: false });
  let externalWatched = $state<ExtCard[]>([]);
  // Своё просмотренное - тоже полным списком с сервера (в данных профиля только последние 24: 26.11 на вкладке было 408, а в списке меньше).
  let localWatched = $state<Card[] | null>(null);
  let watchedLoading = $state(false);
  let watchedError = $state(false);
  $effect(() => {
    const who = id;
    if (!p || hidden.watched || shown !== 'watched') return;
    const controller = new AbortController();
    externalWatched = [];
    localWatched = null;
    watchedLoading = true;
    watchedError = false;
    Promise.all(['movie', 'series'].map(kind =>
      api<{ local: Card[]; ext: ExtCard[] }>(`/api/people/${who}/watched?kind=${kind}`, { signal: controller.signal })
    )).then(results => {
      if (controller.signal.aborted) return;
      externalWatched = results.flatMap(result => result.ext);
      localWatched = results.flatMap(result => result.local);
    }).catch(() => {
      if (!controller.signal.aborted) watchedError = true;
    }).finally(() => {
      if (!controller.signal.aborted) watchedLoading = false;
    });
    return () => controller.abort();
  });
  const isHidden = (t: ProfileTab) => t !== 'home' && (t === 'stats' ? hidden.stats : t === 'watched' ? hidden.watched : hidden.reviews);
  const defs = $derived<{ id: ProfileTab; label: string; n: number | null }[]>([
    { id: 'home', label: 'Профиль', n: null },
    { id: 'stats', label: 'Статистика', n: null },
    { id: 'watched', label: 'Просмотрено', n: hidden.watched ? null : (p?.stats?.watched ?? p?.watched.length ?? null) },
    { id: 'reviews', label: 'Отзывы', n: hidden.reviews ? null : (p?.stats?.reviews ?? p?.reviews.length ?? null) },
  ]);

  // Чёрта под выбранной вкладкой плавно переезжает.
  let btns = $state<Record<string, HTMLButtonElement | undefined>>({});
  let ink = $state({ x: 0, w: 0 });
  function measure() {
    const b = btns[tab];
    if (b) ink = { x: b.offsetLeft, w: b.offsetWidth };
  }
  $effect(() => {
    void tab;
    void p;
    void defs;
    requestAnimationFrame(measure);
  });
  $effect(() => {
    const on = () => measure();
    window.addEventListener('resize', on);
    return () => window.removeEventListener('resize', on);
  });

  // ---------- приватность (только у себя) ----------
  const KEY = { stats: 'visStats', watched: 'visWatched', reviews: 'visReviews' } as const;
  const level = $derived<VisLevel>(p?.decor.visibility?.[tab as 'stats' | 'watched' | 'reviews'] ?? 'all');
  async function setVis(v: VisLevel) {
    if (!p || tab === 'home') return;
    const key = tab;
    const before = p.decor.visibility;
    if (before) p.decor.visibility = { ...before, [key]: v };
    try {
      await patch('/api/me/profile', { [KEY[key]]: v });
    } catch (x) {
      if (p) p.decor.visibility = before;
      toast(errorText(x));
    }
  }

  // Сводка «кто видит» в шапке: всё открыто - «виден всем».
  const visText = $derived.by(() => {
    const v = p?.decor.visibility;
    if (!v) return 'Профиль виден всем';
    const all = [v.stats, v.watched, v.reviews];
    return all.every((x) => x === 'all') ? 'Профиль виден всем' : all.every((x) => x === 'me') ? 'Разделы видны только вам' : 'Часть разделов скрыта';
  });
</script>

{#if error}
  <div class="page empty"><Icon name="info" size={36} /> {error}</div>
{:else if p}
  <div class="pf">
    <ProfileHead bind:p />

    <div class="body">
      <div class="tabbar">
        <div class="tabs" role="tablist" aria-label="Разделы профиля">
          {#each defs as t (t.id)}
            <button
              type="button"
              role="tab"
              id="tab-{t.id}"
              bind:this={btns[t.id]}
              aria-selected={tab === t.id}
              aria-controls="pane"
              onclick={() => choose(t.id)}
            >{t.label}{#if t.n != null}<sup>{t.n}</sup>{/if}{#if isHidden(t.id)}<Icon name="lock" size={12} />{/if}</button>
          {/each}
          <i class="ink" aria-hidden="true" style:transform="translate3d({ink.x}px, 0, 0)" style:width="{ink.w}px"></i>
        </div>
        {#if p.me}<a class="vis" href="/profile?tab=account">{visText}</a>{/if}
      </div>

      {#if p.me && tab !== 'home'}<div class="vp"><VisPick level={level} pick={setVis} /></div>{/if}

      <div id="pane" class="pf-pane" class:out-r={leaving === 'r'} class:out-l={leaving === 'l'} role="tabpanel" aria-labelledby="tab-{shown}">
        {#key shown}
          <div class="in in-{dir}">
            {#if shown === 'home'}
              <Kassa bind:p toReviews={() => choose('reviews')} />
            {:else if shown === 'stats'}
              {#if hidden.stats || !p.stats}<LockBox />{:else}<StatWidgets stats={p.stats} />{/if}
            {:else if shown === 'watched'}
              {#if hidden.watched}
                <LockBox />
              {:else}
                <WatchedTab local={localWatched ?? p.watched} ext={externalWatched} loading={watchedLoading} failed={watchedError} me={p.me} personId={p.person.id} />
              {/if}
            {:else if hidden.reviews}
              <LockBox />
            {:else}
              <ReviewsTab reviews={p.reviews} me={p.me} toWatched={() => choose('watched')} />
            {/if}
          </div>
        {/key}
      </div>
    </div>
  </div>
{:else}
  <div class="sk-hero skeleton" aria-busy="true"></div>
{/if}

<style>
  /* Профиль (шапка «Дневника», вкладка «Касса кинотеатра» с 23.11): шапка на кадре, ниже вкладки текстом; занимает всю ширину окна. */
  /* «Широкий экран» A1 (выбор владельца 25.11): на всю ширину окна, поля - только отступ окна, без колонки по центру. */
  .pf { --pf-side: var(--gutter); padding-bottom: calc(48px + var(--tabbar)); background: #0b0b0d; }
  .body { padding-inline: var(--pf-side); }
  .tabbar { position: relative; display: flex; align-items: center; gap: 20px; margin-top: 30px; border-block: 1px solid #ffffff1a; }
  .tabs { position: relative; display: flex; gap: 28px; min-width: 0; overflow-x: auto; scrollbar-width: none; }
  .tabs::-webkit-scrollbar { display: none; }
  .tabs button { display: flex; align-items: center; gap: 4px; padding: 14px 0; border: 0; background: none; color: #8c8c96; font: inherit; font-size: 12px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; white-space: nowrap; cursor: pointer; transition: color 0.25s; }
  .tabs button:hover { color: #dcdcdf; }
  .tabs button[aria-selected='true'] { color: #fff; }
  .tabs button:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }
  .tabs sup { color: #8c8c96; font-size: 10px; letter-spacing: 0; }
  .ink { position: absolute; left: 0; bottom: 0; height: 2px; background: #fff; transition: transform 0.45s cubic-bezier(0.34, 1.3, 0.64, 1), width 0.45s cubic-bezier(0.34, 1.3, 0.64, 1); }
  .vis { flex: none; margin-left: auto; color: #8c8c96; font-size: 11px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
  .vis:hover { color: #fff; }
  .vp { margin-top: 16px; }

  /* Раздел уезжает и въезжает с той стороны, куда переключили; обрезка по ширине - страница не растёт. */
  .pf-pane { padding-top: 34px; overflow-x: clip; }
  .pf-pane.out-r { animation: out-r 0.2s ease-in both; }
  .pf-pane.out-l { animation: out-l 0.2s ease-in both; }
  .in-r { animation: in-r 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) both; }
  .in-l { animation: in-l 0.6s cubic-bezier(0.2, 0.8, 0.2, 1) both; }
  @keyframes in-r { from { opacity: 0; transform: translate3d(60px, 0, 0); } }
  @keyframes in-l { from { opacity: 0; transform: translate3d(-60px, 0, 0); } }
  @keyframes out-r { to { opacity: 0; transform: translate3d(-40px, 0, 0); } }
  @keyframes out-l { to { opacity: 0; transform: translate3d(40px, 0, 0); } }
  .sk-hero { height: 70vh; }

  @media (max-width: 640px) {
    .tabbar { margin-top: 22px; }
    .tabs { gap: 20px; }
    .vis { display: none; }
  }
</style>
