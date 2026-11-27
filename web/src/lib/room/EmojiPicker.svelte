<script lang="ts">
  import { errorText, put } from '$lib/api';
  import { session } from '$lib/session.svelte';
  import Modal from '$lib/ui/Modal.svelte';

  // Свои смайлики для реакций в комнатах (23.11): до шести, сохраняются в профиле и работают во всех комнатах.
  // Слева выбранные (нажатие - убрать), ниже - все по разделам и готовые наборы под жанр.
  let { open = $bindable(false) }: { open: boolean } = $props();

  const CATS: [string, string[]][] = [
    ['Частые', ['❤️', '😂', '😮', '😢', '🔥', '👏', '😱', '👍', '🍿', '🫶', '🤯', '😭', '💀', '👀', '🙈', '🎉']],
    ['Лица', ['😍', '🥲', '😤', '🫠', '🤔', '😴', '🥶', '🤡', '😬', '😳', '🤩', '😇', '🤨', '😏', '🙄', '🤣']],
    ['Жесты', ['🙏', '👌', '🤌', '✌️', '🫡', '🤝', '💪', '👋', '🙌', '🤞', '👎', '🫣', '🤦', '🤷', '💅', '✍️']],
    ['Кино', ['🎬', '🍿', '⭐', '🎞️', '📽️', '🎭', '🏆', '🎟️', '🔪', '👻', '🩸', '🚀', '👽', '🧟', '🦖', '🕵️']],
    ['Природа', ['🐱', '🐶', '🦊', '🐸', '🐼', '🦄', '🐍', '🦈', '🌧️', '🌈', '⚡', '❄️', '🌙', '🌊', '🌹', '🍀']],
    ['Еда', ['🍕', '🍔', '🍩', '🍫', '🍷', '🍺', '☕', '🍜', '🌮', '🍦', '🥤', '🧃', '🍣', '🥨', '🍉', '🧁']],
  ];
  const SETS: [string, string[]][] = [
    ['Ужасы', ['😱', '🙈', '💀', '👻', '🔪', '🩸']],
    ['Комедия', ['😂', '🤣', '💀', '🤡', '👏', '😹']],
    ['Драма', ['😭', '🥲', '💔', '🫶', '🙏', '😢']],
    ['Киноман', ['🍿', '🎬', '⭐', '🤯', '👀', '🫡']],
  ];
  let cat = $state(0);
  let picked = $state<string[]>([]);
  let busy = $state(false);
  let error = $state('');
  $effect(() => {
    if (open) {
      picked = [...(session.me?.reactions ?? [])];
      error = '';
    }
  });
  function toggle(e: string) {
    if (picked.includes(e)) picked = picked.filter((x) => x !== e);
    else if (picked.length < 6) picked = [...picked, e];
  }
  async function save() {
    busy = true;
    error = '';
    try {
      const r = await put<{ reactions: string[] }>('/api/me/reactions', { emojis: picked });
      if (session.me) session.me.reactions = r.reactions;
      open = false;
    } catch (x) {
      error = errorText(x);
    } finally {
      busy = false;
    }
  }
</script>

<Modal bind:open title="Мои реакции" sheet>
  <div class="ep">
    <div class="slots" aria-label="Выбрано">
      <!-- Шесть мест: занятые - выбранные смайлики (нажатие убирает), свободные - пунктиром. -->
      {#each picked as e (e)}<button type="button" class="slot" onclick={() => toggle(e)} aria-label="Убрать {e}">{e}</button>{/each}
      {#each { length: Math.max(0, 6 - picked.length) } as _, i (i)}<span class="slot free" aria-hidden="true">+</span>{/each}
    </div>
    <div class="cats" role="tablist" aria-label="Разделы">
      {#each CATS as c, i (c[0])}<button type="button" role="tab" aria-selected={cat === i} class:on={cat === i} onclick={() => (cat = i)}>{c[0]}</button>{/each}
    </div>
    <div class="grid">
      {#each CATS[cat][1] as e (e)}
        <button type="button" class:on={picked.includes(e)} aria-pressed={picked.includes(e)} disabled={!picked.includes(e) && picked.length >= 6} onclick={() => toggle(e)}>{e}</button>
      {/each}
    </div>
    <div class="sets">
      <small>Готовые наборы</small>
      {#each SETS as s (s[0])}
        <button type="button" onclick={() => (picked = [...s[1]])}><b>{s[0]}</b><span>{s[1].join(' ')}</span></button>
      {/each}
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="acts">
      <button class="btn ghost" type="button" onclick={() => (picked = [])}>Как у всех</button>
      <button class="btn primary" type="button" disabled={busy} onclick={save}>{busy ? 'Сохраняю…' : 'Сохранить'}</button>
    </div>
    <p class="note">Набор работает во всех комнатах: другие видят ваши смайлики с вашим аватаром.</p>
  </div>
</Modal>

<style>
  .ep { display: grid; gap: 14px; padding: 4px 18px 18px; }
  .slots { display: flex; gap: 8px; }
  .slot { width: 46px; height: 46px; border-radius: 14px; display: grid; place-items: center; font-size: 24px; background: #ffffff14; border: 0; cursor: pointer; }
  .slot.free { background: none; border: 1px dashed #ffffff40; color: #6c6c75; font-size: 18px; cursor: default; }
  .cats { display: flex; gap: 6px; flex-wrap: wrap; }
  .cats button { padding: 6px 12px; border: 0; border-radius: 999px; background: #ffffff12; color: #fff; font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
  .cats button.on { background: #fff; color: #000; }
  .grid { display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); gap: 6px; }
  .grid button { aspect-ratio: 1; border: 0; border-radius: 12px; background: #ffffff0a; font-size: 22px; cursor: pointer; }
  .grid button:hover:not(:disabled) { background: #ffffff1f; }
  .grid button.on { background: #fff; }
  .grid button:disabled { opacity: 0.35; cursor: default; }
  .sets { display: grid; gap: 6px; }
  .sets small { color: #a1a1aa; font-size: 12px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; }
  .sets button { display: flex; align-items: center; gap: 12px; padding: 9px 12px; border: 0; border-radius: 12px; background: #ffffff0a; color: #fff; font: inherit; text-align: left; cursor: pointer; }
  .sets button:hover { background: #ffffff16; }
  .sets b { min-width: 76px; font-size: 13px; }
  .sets span { font-size: 18px; letter-spacing: 3px; }
  .acts { display: flex; justify-content: space-between; gap: 8px; }
  .note { margin: 0; color: #a1a1aa; font-size: 13px; }
  .error { margin: 0; color: #ef6a6a; }
  @media (max-width: 420px) { .grid { grid-template-columns: repeat(6, minmax(0, 1fr)); } }
</style>
