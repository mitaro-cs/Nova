<script lang="ts">
  import { api, patch, post } from '$lib/api';
  import { getDetail } from '$lib/discover/details';
  import StarInput from '$lib/social/StarInput.svelte';
  import type { Card, ExtCard, Review } from '$lib/types';
  import { artW, placeholder } from '$lib/ui/art';
  import Icon from '$lib/ui/Icon.svelte';

  // Конец фильма (выбор владельца, 23.11): сразу оценить звёздами и выбрать, что дальше - похожее на этот фильм.
  // Свой фильм - похожие из каталога Nova, фильм ленты (видео по ссылке) - «похожие» IMDb. Оценка - та же, что на
  // странице фильма: своя уже есть - меняем её, а не пишем второй отзыв.
  let { titleId = 0, linkId = null, title, onreplay, onback }: { titleId?: number; linkId?: number | null; title: string; onreplay: () => void; onback: () => void } = $props();

  type Rec = { key: string; title: string; poster: string | null; href: string };
  let recs = $state<Rec[]>([]);
  let ext = $state<string | null>(null);
  let mine = $state<Review | null>(null);
  let stars = $state<number | null>(null);
  let saved = $state(false);

  const reviewsUrl = $derived(titleId ? `/api/titles/${titleId}/reviews` : ext ? `/api/discover/title/${ext}/reviews` : null);

  $effect(() => {
    let live = true;
    (async () => {
      if (titleId) {
        const t = await api<{ similar?: Card[] }>(`/api/titles/${titleId}`).catch(() => null);
        if (live && t?.similar) recs = t.similar.slice(0, 4).map((c) => ({ key: `t${c.id}`, title: c.title, poster: c.posterLarge ?? c.poster ?? null, href: `/title/${c.id}` }));
      } else if (linkId) {
        const l = await api<{ extId?: string | null }>(`/api/links/${linkId}`).catch(() => null);
        if (!live || !l?.extId) return;
        ext = l.extId;
        const d = await getDetail(l.extId).catch(() => null);
        if (live && d?.similar) recs = d.similar.slice(0, 4).map((c: ExtCard) => ({ key: `x${c.id}`, title: c.title, poster: c.poster ?? null, href: `/ext/${c.id}` }));
      }
    })();
    return () => (live = false);
  });
  $effect(() => {
    const url = reviewsUrl;
    if (!url) return;
    let live = true;
    api<{ items: Review[] }>(url).then((r) => {
      if (!live) return;
      mine = r.items.find((x) => x.mine) ?? null;
      stars = mine?.stars ?? null;
    }, () => {});
    return () => (live = false);
  });

  async function rate(v: number | null) {
    if (!v || !reviewsUrl) return;
    stars = v;
    try {
      if (mine) mine = await patch<Review>(`/api/reviews/${mine.id}`, { body: mine.body, stars: v });
      else mine = await post<Review>(reviewsUrl, { stars: v });
      saved = true;
    } catch {
      saved = false;
    }
  }
</script>

<div class="end" role="dialog" aria-label="Фильм закончился">
  <div class="box">
    <p class="eyebrow">Фильм закончился</p>
    <h2>Как вам «{title}»?</h2>
    {#if reviewsUrl}
      <div class="rate"><StarInput value={stars} onchange={rate} size={34} label="Ваша оценка" /></div>
      <p class="note" aria-live="polite">{saved ? 'Оценка сохранена - Nova учтёт её в советах' : 'Оценка помогает Nova подбирать фильмы под вас'}</p>
    {/if}
    {#if recs.length}
      <p class="eyebrow">Дальше</p>
      <ul class="recs">
        {#each recs as r (r.key)}
          <li><a href={r.href} title={r.title} style:background={placeholder(r.title)}>{#if r.poster}<img src={artW(r.poster, 342)} alt={r.title} loading="lazy" width="120" height="180" />{:else}<span>{r.title}</span>{/if}</a></li>
        {/each}
      </ul>
    {/if}
    <div class="acts">
      <button class="btn primary" type="button" onclick={onreplay}><Icon name="refresh" size={16} /> Смотреть снова</button>
      <button class="btn" type="button" onclick={onback}><Icon name="back" size={16} /> К фильму</button>
    </div>
  </div>
</div>

<style>
  .end { position: absolute; inset: 0; z-index: 8; display: grid; place-items: center; padding: 24px; background: #000000d0; animation: in 0.4s ease both; }
  .box { display: grid; justify-items: center; gap: 14px; max-width: 640px; text-align: center; }
  .eyebrow { margin: 6px 0 0; color: #a1a1aa; font-size: 12px; font-weight: 700; letter-spacing: 0.16em; text-transform: uppercase; }
  h2 { margin: 0; font-size: clamp(24px, 3.4vw, 40px); font-weight: 800; letter-spacing: -0.03em; line-height: 1.1; text-wrap: balance; }
  .rate { padding: 4px 0; }
  .note { margin: 0; color: #c9c9cf; font-size: 13px; }
  .recs { display: flex; gap: 12px; margin: 0; padding: 0; list-style: none; }
  .recs a { display: grid; place-items: center; width: clamp(84px, 11vw, 120px); aspect-ratio: 2 / 3; border-radius: 10px; overflow: hidden; color: #fff; font-size: 12px; font-weight: 700; transition: transform 0.25s var(--ease, ease); }
  .recs a:hover { transform: translateY(-4px); }
  .recs img { width: 100%; height: 100%; object-fit: cover; }
  .acts { display: flex; flex-wrap: wrap; justify-content: center; gap: 10px; margin-top: 6px; }
  @keyframes in { from { opacity: 0; } }
</style>
