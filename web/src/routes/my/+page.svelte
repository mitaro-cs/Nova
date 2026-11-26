<script lang="ts">
  import { api } from '$lib/api';
  import { extSheet } from '$lib/discover/sheet.svelte';
  import type { Marks } from '$lib/types';
  import PosterRow from '$lib/discover/PosterRow.svelte';
  import Icon from '$lib/ui/Icon.svelte';

  // «Коллекция»: крупный заголовок и ряды отмеченного - «Буду смотреть» с оценкой на
  // постере, «Смотрю», «Просмотрено»; ниже - вход в общие подборки. Загрузок на устройство нет.
  let marks = $state<Marks | null>(null);
  let failed = $state(false);
  let retry = $state(0);
  // Не дольше 20 с: на медленной связи (телефон через туннель) лучше «Повторить», чем вечная заглушка.
  $effect(() => {
    void extSheet.version;
    void extSheet.marks;
    void retry;
    let live = true;
    failed = false;
    const ctl = new AbortController();
    const t = setTimeout(() => ctl.abort(), 20000);
    api<Marks>('/api/me/marks', { signal: ctl.signal }).then(
      (m) => live && ((marks = m), (failed = false)),
      () => live && (failed = true),
    ).finally(() => clearTimeout(t));
    return () => {
      live = false;
      clearTimeout(t);
    };
  });
  const empty = $derived(marks !== null && !marks.want.length && !marks.watching.length && !marks.watched.length);
</script>

<svelte:head><title>Моё - Nova</title></svelte:head>

<main class="my">
  <h1>Моё</h1>
  {#if failed && !marks}
    <p class="note">Не удалось загрузить. Проверьте связь.</p>
    <button class="btn again" onclick={() => retry++}>Повторить</button>
  {:else if marks === null}
    <div class="skeleton" aria-busy="true"></div>
  {:else}
    <!-- Ошибка в одном ряду не оставляет страницу пустой: ряд заменяется строкой с «Повторить». -->
    <svelte:boundary>
      <div class="rows">
        {#if marks.want.length}<PosterRow title="Буду смотреть" href="/lists" items={marks.want} />{/if}
        {#if marks.watching.length}<PosterRow title="Смотрю" items={marks.watching} />{/if}
        {#if marks.watched.length}<PosterRow title="Просмотрено" items={marks.watched} />{/if}
      </div>
      {#snippet failed(_e, reset)}
        <p class="note">Не удалось показать отметки. <button class="btn sm" onclick={reset}>Повторить</button></p>
      {/snippet}
    </svelte:boundary>
    {#if empty}
      <p class="note">Здесь будет то, что вы отметите: «Хочу», «Смотрю» и «Просмотрено». Начните с <a href="/discover">каталога</a>.</p>
    {/if}
    <a class="lists" href="/lists">
      <Icon name="list" size={24} />
      <span><b>Подборки</b><small>Общие с друзьями</small></span>
      <Icon name="right" size={20} />
    </a>
  {/if}
</main>

<style>
  .my {
    min-height: 100dvh;
    padding: calc(var(--header) + 24px) 0 calc(48px + var(--tabbar));
  }
  h1 {
    margin: 0 var(--gutter) 24px;
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }
  .note {
    margin: 0 var(--gutter) 28px;
    color: var(--muted);
    font-size: 16px;
    line-height: 1.4;
  }
  .note a {
    color: #fff;
    text-decoration: underline;
  }
  .rows {
    padding: 0 var(--gutter);
  }
  .again {
    margin: 0 var(--gutter);
  }
  .skeleton {
    height: 300px;
    margin: 0 var(--gutter);
    border-radius: 4px;
    background: #141414;
  }
  .lists {
    display: flex;
    align-items: center;
    gap: 14px;
    margin: 0 var(--gutter);
    padding: 16px;
    border-radius: 12px;
    background: #1a1a1a;
    color: #fff;
  }
  .lists span {
    flex: 1;
    display: grid;
    gap: 2px;
  }
  .lists small {
    color: var(--muted);
    font-size: 13px;
  }
  /* Телефон: шапки нет - заголовок ниже строки состояния, как у референса. */
  @media (max-width: 640px) {
    .my {
      padding-top: calc(env(safe-area-inset-top) + 56px);
    }
  }
</style>
