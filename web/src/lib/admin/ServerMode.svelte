<script lang="ts">
  import { api, errorText, put } from '$lib/api';
  import { ask } from '$lib/ui/dialogs.svelte';
  import { toast } from '$lib/ui/toast.svelte';
  import type { Status } from './types';

  // «Центр управления»: оценка здоровья сервера из 100 и три режима. Режим одним выбором меняет регистрацию
  // (закрыта / по приглашениям / открыта); «Только дом» ещё и выключает доступ из интернета.
  let { status, reload }: { status: Status; reload: () => void } = $props();

  type Mode = 'home' | 'family' | 'open';
  const MODES: { id: Mode; name: string; text: string; tags: string[]; reg: 'closed' | 'invite' | 'open' }[] = [
    { id: 'home', name: 'Только дом', text: 'Смотрят устройства из вашей сети. Регистрация закрыта.', tags: ['Без интернета', 'Закрытая регистрация'], reg: 'closed' },
    { id: 'family', name: 'Семья и друзья', text: 'Постоянный адрес, вход по приглашению.', tags: ['Адрес в интернете', 'По приглашениям'], reg: 'invite' },
    { id: 'open', name: 'Открытый', text: 'Любой может зарегистрироваться. Больше лимитов и защиты.', tags: ['Регистрация открыта', 'Строгие лимиты'], reg: 'open' },
  ];

  let reg = $state<'closed' | 'invite' | 'open' | null>(null);
  let busy = $state(false);
  $effect(() => {
    api<{ registration: 'invite' | 'open' | 'closed' }>('/api/admin/settings').then((s) => (reg = s.registration), () => {});
  });
  const mode = $derived(MODES.find((m) => m.reg === reg)?.id ?? null);

  async function choose(m: (typeof MODES)[number]) {
    if (busy || m.reg === reg) return;
    if (m.id === 'home' && status.publicUrl && !(await ask('Выключить доступ из интернета и закрыть регистрацию? Друзья вне вашей сети перестанут заходить.', 'Выключить', true))) return;
    busy = true;
    try {
      await put('/api/admin/settings', { registration: m.reg });
      if (m.id === 'home' && status.publicUrl) await put('/api/admin/access', { mode: 'lan' });
      reg = m.reg;
      toast(`Режим: ${m.name}`);
      reload();
    } catch (e) {
      toast(errorText(e));
    } finally {
      busy = false;
    }
  }

  // Оценка: что мешает работе - вычитаем.
  const checks = $derived.by(() => {
    const low = status.system?.diskFree != null && status.system.diskFree < 10 * 1024 ** 3;
    const list: { w: number; text: string; href: string }[] = [];
    if (!status.ffmpeg || !status.ffprobe) list.push({ w: 25, text: 'Нет ffmpeg: не играют MKV и AVI', href: '?tab=settings' });
    if (low) list.push({ w: 25, text: 'Мало места на диске', href: '?tab=overview' });
    if (status.scan.error) list.push({ w: 10, text: 'Сканирование папки закончилось ошибкой', href: '?tab=folders' });
    if (status.unmatched > 0) list.push({ w: 5, text: `${status.unmatched} фильмов без описания`, href: '?tab=catalog' });
    if (status.update?.state === 'available') list.push({ w: 3, text: 'Есть новая версия', href: '?tab=version' });
    if (!status.files) list.push({ w: 15, text: 'Фильмов пока нет: добавьте папку', href: '?tab=folders' });
    return list;
  });
  const score = $derived(Math.max(0, 100 - checks.reduce((a, c) => a + c.w, 0)));
  const verdict = $derived(score >= 95 ? 'Всё отлично' : score >= 80 ? 'Почти идеально' : score >= 60 ? 'Есть что поправить' : 'Нужно внимание');
</script>

<section class="center" aria-label="Состояние и режим">
  <div class="health">
    <div class="ring" style:--p="{score}%" role="img" aria-label="Оценка {score} из 100"><span><b>{score}</b><small>из 100</small></span></div>
    <div class="ht">
      <h3>{verdict}</h3>
      {#if checks.length}
        <ul>{#each checks.slice(0, 3) as c (c.text)}<li><a href={c.href} data-sveltekit-replacestate data-sveltekit-noscroll>{c.text}</a></li>{/each}</ul>
      {:else}<p>Сервер работает без замечаний</p>{/if}
    </div>
  </div>
  <div class="modes" role="radiogroup" aria-label="Режим сервера">
    {#each MODES as m (m.id)}
      <button type="button" role="radio" aria-checked={mode === m.id} class:on={mode === m.id} disabled={busy || reg === null} onclick={() => choose(m)}>
        <span class="top"><b>{m.name}</b><i></i></span>
        <span class="tx">{m.text}</span>
        <span class="tags">{#each m.tags as t (t)}<em>{t}</em>{/each}</span>
      </button>
    {/each}
  </div>
</section>

<style>
  .center { display: grid; grid-template-columns: minmax(0, 360px) minmax(0, 1fr); gap: 14px; margin-bottom: 14px; }
  .health { display: flex; align-items: center; gap: 20px; padding: 20px 22px; border: 1px solid #ffffff1a; border-radius: 28px; background: #101012; }
  .ring { flex: none; display: grid; place-items: center; width: 104px; height: 104px; border-radius: 50%; background: conic-gradient(#22c55e var(--p), #ffffff1a 0); }
  .ring span { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px; width: 84px; height: 84px; border-radius: 50%; background: #101012; }
  .ring b { font-size: 28px; font-weight: 800; letter-spacing: -0.04em; line-height: 1.1; }
  .ring small { color: var(--muted); font-size: 11px; line-height: 1.2; }
  .ht { min-width: 0; }
  h3 { margin: 0; font-size: 17px; }
  .ht p { margin: 6px 0 0; color: var(--muted); font-size: 13px; }
  ul { margin: 8px 0 0; padding: 0; list-style: none; display: grid; gap: 4px; font-size: 13px; }
  ul a { color: #fcd34d; }
  ul a:hover { text-decoration: underline; }
  .modes { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
  .modes button { display: grid; align-content: start; gap: 8px; padding: 18px; border: 1px solid #ffffff1a; border-radius: 26px; background: #101012; color: inherit; font: inherit; text-align: left; cursor: pointer; transition: transform 0.2s, background 0.2s; }
  .modes button:hover:not(:disabled) { transform: translate3d(0, -2px, 0); background: #17171a; }
  .modes button.on { border: 2px solid #fff; padding: 17px; background: #17171a; }
  .modes button:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }
  .top { display: flex; justify-content: space-between; align-items: center; }
  .top b { font-size: 17px; }
  .top i { width: 20px; height: 20px; border: 2px solid #ffffff40; border-radius: 50%; box-sizing: border-box; }
  .on .top i { border-color: #fff; background: radial-gradient(#fff 0 40%, transparent 45%); }
  .tx { color: #c4c4cc; font-size: 13px; line-height: 1.4; }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .tags em { padding: 4px 10px; border-radius: 999px; background: #ffffff14; font-size: 11px; font-style: normal; font-weight: 600; }
  @media (max-width: 1100px) { .center { grid-template-columns: minmax(0, 1fr); } }
  @media (max-width: 700px) { .modes { grid-template-columns: minmax(0, 1fr); } }
</style>
