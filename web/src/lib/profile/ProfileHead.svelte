<script lang="ts">
  import { getDetail, peekDetail } from '$lib/discover/details';
  import { plural } from '$lib/fmt';
  import FriendButton from '$lib/social/FriendButton.svelte';
  import PosterWall from '$lib/social/PosterWall.svelte';
  import { artW } from '$lib/ui/art';
  import Avatar from '$lib/ui/Avatar.svelte';
  import { countUp } from '$lib/ui/countUp';
  import { fadeIn } from '$lib/ui/fade';
  import Icon from '$lib/ui/Icon.svelte';
  import { levelOf, rankOf } from './level';
  import type { ProfilePage } from './types';

  // Шапка профиля «Дневник» (макет Y, выбор владельца 21.11, по мотивам Letterboxd): сверху кадр любимого фильма,
  // под ним портрет, имя, уровень, «О себе», цифры через тонкие черты и кнопки.
  let { p = $bindable() }: { p: ProfilePage } = $props();

  const s = $derived(p.stats);
  const lv = $derived(levelOf(s));

  // Кадр: своя обложка, иначе кадр первого любимого фильма (у фильма из ленты - из подробностей IMDb).
  const fav = $derived(p.decor.favorites[0] ?? null);
  const favTitle = $derived(fav?.card?.title ?? fav?.ext?.title ?? '');
  let extFrame = $state<string | null>(null);
  $effect(() => {
    const id = fav && !fav.card?.backdrop ? fav.ext?.id : null;
    extFrame = id ? (peekDetail(id)?.backdrop ?? null) : null;
    if (!id || extFrame) return;
    let live = true;
    getDetail(id).then((d) => live && (extFrame = d.backdrop ?? null), () => {});
    return () => (live = false);
  });
  const ownCover = $derived(p.decor.coverKind === 'image' ? p.decor.cover : null);
  const art = $derived(ownCover ?? fav?.card?.backdrop ?? (extFrame ? artW(extFrame, 1920) : null));
  const caption = $derived(!ownCover && art && favTitle ? `Кадр: ${favTitle} · любимый фильм` : '');

  const nums = $derived(
    s
      ? [
          { v: s.movies, l: plural(s.movies, 'фильм', 'фильма', 'фильмов') },
          { v: s.episodes, l: plural(s.episodes, 'серия', 'серии', 'серий') },
          { v: s.reviews, l: plural(s.reviews, 'отзыв', 'отзыва', 'отзывов') },
          { v: s.friends, l: plural(s.friends, 'друг', 'друга', 'друзей') },
        ]
      : [],
  );

  let shared = $state(false);
  async function share() {
    try {
      await navigator.clipboard.writeText(location.origin + `/people/${p.person.id}`);
      shared = true;
      setTimeout(() => (shared = false), 1800);
    } catch {
      /* без буфера обмена - ничего */
    }
  }
</script>

<header class="ph">
  <div class="art" aria-hidden="true">
    <!-- Сверху всегда картинка: своя обложка, кадр любимого фильма или стена постеров из просмотренного. -->
    {#if art}<img use:fadeIn src={art} alt="" decoding="async" />{:else}<div class="wall"><PosterWall posters={p.decor.ribbon} accent={p.decor.accent} prominent /></div>{/if}
    {#if caption}<span class="cap">{caption}</span>{/if}
  </div>

  <div class="id">
    <span class="face"><Avatar avatar={p.person.avatar} photo={p.person.photo} size={96} /></span>
    <div class="who">
      <div class="nm"><h1>{p.person.name}</h1>{#if s}<span class="lvl">Уровень {lv.level} · {rankOf(lv.level)}</span>{/if}</div>
      {#if p.decor.bio}<p class="bio selectable">{p.decor.bio}</p>{/if}
    </div>
    {#if nums.length}
      <ul class="nums" aria-label="Цифры">
        {#each nums as n (n.l)}<li><b use:countUp={{ value: n.v }}>0</b><small>{n.l}</small></li>{/each}
      </ul>
    {:else}
      <p class="locked"><Icon name="lock" size={14} /> Цифры скрыты</p>
    {/if}
    <div class="acts">
      {#if p.me}
        <a class="btn primary" href="/profile"><Icon name="pencil" size={14} /> Оформить</a>
        <button class="btn" type="button" onclick={share}><Icon name={shared ? 'check' : 'share'} size={14} /> {shared ? 'Ссылка скопирована' : 'Поделиться'}</button>
      {:else if p.relation}
        <FriendButton id={p.person.id} bind:relation={p.relation} />
        {#if p.relation === 'friend'}<a class="btn" href="/chats/{p.person.id}"><Icon name="mail" size={14} /> Написать</a>{/if}
      {/if}
    </div>
  </div>
</header>

<style>
  .ph { position: relative; padding-top: clamp(220px, 24vw, 330px); }
  .art { position: absolute; inset: 0 0 auto; height: clamp(320px, 33vw, 470px); overflow: hidden; }
  .wall { position: absolute; inset: 0; }
  .wall :global(.wall) { position: absolute; inset: 0; }
  .art img { width: 100%; height: 100%; object-fit: cover; object-position: 50% 35%; }
  .art::after { content: ''; position: absolute; inset: 0; background: linear-gradient(180deg, #0b0b0d66 0%, #0b0b0d00 30%, #0b0b0d 97%), linear-gradient(90deg, #0b0b0d 0%, #0b0b0d00 18%, #0b0b0d00 82%, #0b0b0d 100%); }
  .cap { position: absolute; z-index: 1; right: var(--pf-side); top: calc(var(--header) + 18px); color: #ffffffa6; font-size: 11px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; }

  .id { position: relative; display: flex; flex-wrap: wrap; align-items: flex-end; gap: 18px 22px; padding-inline: var(--pf-side); }
  /* Портрет без обводки, как раньше (просьба владельца, 23.11). */
  .face { flex: none; border-radius: 50%; }
  .face :global(img), .face :global(.avatar) { border-radius: 50%; }
  .who { flex: 1 1 280px; min-width: 0; padding-bottom: 4px; }
  .nm { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 12px; }
  h1 { margin: 0; font-size: clamp(26px, 2.4vw, 32px); font-weight: 800; letter-spacing: -0.03em; line-height: 1.1; overflow-wrap: anywhere; }
  .lvl { padding: 6px 12px; border-radius: 999px; background: #ffffff14; font-size: 11.5px; font-weight: 700; }
  .bio { margin: 6px 0 0; max-width: 560px; color: #b4b4bc; font-size: 14px; line-height: 1.5; overflow-wrap: anywhere; }
  .nums { display: flex; margin: 0; padding: 0 0 4px; list-style: none; }
  .nums li { display: grid; justify-items: center; padding: 0 18px; border-left: 1px solid #ffffff1f; }
  .nums b { font-size: 22px; font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .nums small { color: #8c8c96; font-size: 10px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
  .locked { display: flex; align-items: center; gap: 8px; margin: 0; padding-bottom: 8px; color: #8c8c96; font-size: 13px; }
  .acts { display: flex; flex-wrap: wrap; gap: 8px; padding-bottom: 6px; }
  .acts .btn, .acts :global(.btn) { min-height: 40px; }

  @media (max-width: 900px) {
    .id { align-items: flex-start; }
    .nums { order: 3; width: 100%; justify-content: space-between; padding-top: 6px; border-top: 1px solid #ffffff14; }
    .nums li { flex: 1; padding: 10px 0 0; border-left: 0; }
    .acts { order: 4; width: 100%; }
    .acts .btn, .acts :global(.btn) { flex: 1; justify-content: center; }
  }
  @media (max-width: 640px) {
    .ph { padding-top: 170px; }
    .art { height: 260px; }
    .cap { display: none; }
    .face :global(*) { width: 76px !important; height: 76px !important; }
  }
</style>
