<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { size } from '$lib/fmt';
  import { forgetSpeed, measureSpeed } from '$lib/player/speed';
  import Icon from '$lib/ui/Icon.svelte';
  import Access from './Access.svelte';
  import Folders from './Folders.svelte';
  import People from './People.svelte';
  import Settings from './Settings.svelte';
  import type { Status } from './types';

  // Мастер настройки (дизайн «Мастер, живой»): пять шагов, любой открывается нажатием или цифрой. Справа - проверки,
  // которые идут сами (папка, ffmpeg, описания, доступ) и замер скорости до этого устройства. Всё хранится сразу,
  // «Сохранить» не нужно: каждый шаг - те же разделы управления.
  let { status, reload, window: inWindow }: { status: Status; reload: () => void; window: boolean } = $props();

  const STEPS = [
    { id: 'folders', n: 'Папка', title: 'Где лежат ваши фильмы?', text: 'Выберите папку или перетащите файлы в окно: Nova следит за папкой сама.' },
    { id: 'meta', n: 'Описания', title: 'Откуда брать постеры и описания?', text: 'По умолчанию - IMDb, без ключей. Названия и описания показываются по-русски.' },
    { id: 'access', n: 'Доступ', title: 'Кто и откуда сможет смотреть?', text: 'Для просмотра не из дома нужен постоянный адрес. Если не уверены, оставьте «Только дома».' },
    { id: 'people', n: 'Люди', title: 'Пригласите зрителей', text: 'Отправьте ссылку-приглашение или откройте регистрацию.' },
    { id: 'done', n: 'Готово', title: 'Всё готово', text: 'Проверьте итог. Любую настройку можно изменить позже в «Управлении».' },
  ] as const;

  const at = $derived(Math.max(0, STEPS.findIndex((s) => s.id === page.url.searchParams.get('step'))));
  const step = $derived(STEPS[at]);

  function go(i: number) {
    const n = Math.min(STEPS.length - 1, Math.max(0, i));
    goto(`?tab=setup&step=${STEPS[n].id}`, { replace: true, reset: false });
  }

  // Состояние каждого шага - по данным сервера, обновляется само (родитель опрашивает `status`).
  type Check = { n: string; d: string; state: 'ok' | 'run' | 'warn' | 'todo'; v: string };
  const checks = $derived.by<Check[]>(() => {
    const list: Check[] = [];
    list.push(
      status.files > 0
        ? { n: 'Папка найдена', d: `${status.files} видеофайлов, ${size(status.size)}`, state: 'ok', v: 'готово' }
        : { n: 'Папка пока пустая', d: 'Добавьте папку или перетащите файлы в окно', state: 'todo', v: 'нужно' },
    );
    list.push(
      status.ffmpeg && status.ffprobe
        ? { n: 'ffmpeg готов', d: 'MKV, AVI и озвучки поддерживаются', state: 'ok', v: 'готово' }
        : { n: 'ffmpeg не найден', d: 'Без него не играют MKV и AVI: укажите путь в настройках', state: 'warn', v: 'нужно' },
    );
    if (status.scan.running) list.push({ n: 'Читаю папку', d: 'Ищу фильмы и серии', state: 'run', v: status.scan.progress != null ? `${Math.round(status.scan.progress * 100)}%` : '…' });
    if (status.meta.running || status.metaPending > 0) {
      list.push({ n: 'Описания и постеры', d: `Готово ${status.meta.done}, осталось ${status.metaPending}`, state: 'run', v: 'идёт' });
    } else if (status.unmatched > 0) {
      list.push({ n: 'Названия и годы', d: `${status.unmatched} фильмов без описания: уточните в «Каталоге»`, state: 'warn', v: `${status.unmatched} вопросов` });
    } else if (status.titles > 0) {
      list.push({ n: 'Описания готовы', d: `${status.titles} фильмов с постерами`, state: 'ok', v: 'готово' });
    }
    list.push(
      status.publicUrl
        ? { n: 'Доступ из интернета', d: status.publicUrl, state: 'ok', v: 'включён' }
        : { n: 'Только дома', d: 'Из интернета не откроется', state: 'todo', v: 'по желанию' },
    );
    return list;
  });
  const doneShare = $derived(Math.round((checks.filter((c) => c.state === 'ok').length / checks.length) * 100));
  const marks = { ok: '✓', run: '…', warn: '!', todo: '·' } as const;

  // Замер скорости: сколько зрителей потянет раздача 1080p (около 8 Мбит/с на зрителя).
  let mbit = $state<number | null>(null);
  let measuring = $state(false);
  let measured = $state(false);
  async function measure(again = false) {
    if (measuring) return;
    measuring = true;
    if (again) forgetSpeed();
    mbit = await measureSpeed();
    measured = true;
    measuring = false;
  }
  $effect(() => {
    void measure();
  });
  const viewers = $derived(mbit ? Math.max(1, Math.floor(mbit / 8)) : 0);

  function onkey(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    if (t && (t.closest('input, textarea, select, [contenteditable]') || e.metaKey || e.ctrlKey || e.altKey)) return;
    if (/^[1-5]$/.test(e.key)) go(Number(e.key) - 1);
    else if (e.key === 'ArrowRight') go(at + 1);
    else if (e.key === 'ArrowLeft') go(at - 1);
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="wz">
  <ol class="steps" aria-label="Шаги настройки">
    {#each STEPS as s, i (s.id)}
      <li>
        <button type="button" class:on={i === at} class:past={i < at} aria-current={i === at ? 'step' : undefined} onclick={() => go(i)}>
          <span class="no">{i < at ? '✓' : i + 1}</span>
          <span class="tx"><b>{s.n}</b><small>{i < at ? 'пройден' : i === at ? 'сейчас' : 'дальше'}</small></span>
        </button>
      </li>
    {/each}
  </ol>

  <div class="grid">
    <section class="card" aria-label="Шаг {at + 1} из {STEPS.length}">
      <p class="kick">Шаг {at + 1} из {STEPS.length}</p>
      <h2>{step.title}</h2>
      <p class="lead">{step.text}</p>

      {#key step.id}
        <div class="body">
          {#if step.id === 'folders'}<Folders {status} window={inWindow} {reload} />
          {:else if step.id === 'meta'}<Settings sourcesOnly />
          {:else if step.id === 'access'}<Access />
          {:else if step.id === 'people'}<People />
          {:else}
            <ul class="final">
              {#each checks as c (c.n)}
                <li><span class="m {c.state}">{marks[c.state]}</span><div><b>{c.n}</b><small>{c.d}</small></div></li>
              {/each}
            </ul>
            <a class="btn primary" href="/">Открыть Nova</a>
          {/if}
        </div>
      {/key}

      <div class="nav">
        <button class="btn" type="button" disabled={at === 0} onclick={() => go(at - 1)}>Назад</button>
        {#if at < STEPS.length - 1}<button class="btn primary" type="button" onclick={() => go(at + 1)}>Дальше: {STEPS[at + 1].n.toLowerCase()}</button>{/if}
      </div>
    </section>

    <aside class="side" aria-label="Проверки">
      <section class="tile">
        <div class="hd"><b>Проверки идут сами</b><span class="pct">{doneShare}%</span></div>
        <div class="bar" role="progressbar" aria-valuenow={doneShare} aria-valuemin="0" aria-valuemax="100"><i style:width="{doneShare}%"></i></div>
        <ul class="chk">
          {#each checks as c (c.n)}
            <li><span class="m {c.state}">{marks[c.state]}</span><div><b>{c.n}</b><small>{c.d}</small></div><em class={c.state}>{c.v}</em></li>
          {/each}
        </ul>
      </section>

      <section class="tile">
        <b>Скорость до этого устройства</b>
        {#if measuring && !measured}
          <p class="big"><span class="pulse">измеряю…</span></p>
        {:else if mbit}
          <p class="big">{mbit}<small> Мбит/с</small></p>
          <p class="note">Хватит примерно на <b>{viewers}</b> {viewers === 1 ? 'зрителя' : 'зрителей'} в 1080p одновременно</p>
        {:else}
          <p class="note">Замер не вышел. Попробуйте ещё раз</p>
        {/if}
        <button class="btn sm" type="button" disabled={measuring} onclick={() => measure(true)}><Icon name="refresh" size={14} /> Проверить ещё раз</button>
      </section>

      <p class="keys"><b>Клавиши</b> Цифры 1-5: к шагу · ← →: назад и дальше</p>
    </aside>
  </div>
</div>

<style>
  .wz { display: grid; gap: 20px; }
  .steps { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 10px; margin: 0; padding: 0; list-style: none; }
  .steps button { display: flex; align-items: center; gap: 12px; width: 100%; padding: 13px 16px; border: 1px solid #ffffff1a; border-radius: 22px; background: #0c0c0e; color: inherit; font: inherit; text-align: left; cursor: pointer; transition: transform 0.2s, background 0.2s, border-color 0.2s; }
  .steps button:hover { transform: translate3d(0, -2px, 0); background: #17171a; }
  .steps button:focus-visible { outline: 2px solid #fff; outline-offset: 2px; }
  .steps button.on { border-color: #ffffff66; background: #17171a; }
  .no { display: grid; place-items: center; flex: none; width: 30px; height: 30px; border-radius: 50%; background: #ffffff1a; font-size: 13px; font-weight: 800; }
  .on .no, .past .no { background: #fff; color: #0a0a0b; }
  .tx { display: grid; min-width: 0; }
  .tx b { font-size: 14px; }
  .tx small { color: var(--muted); font-size: 12px; }
  .grid { display: grid; grid-template-columns: minmax(0, 1.5fr) minmax(0, 1fr); gap: 20px; align-items: start; }
  .card { min-width: 0; padding: 30px; border: 1px solid #ffffff1a; border-radius: 34px; background: #0c0c0e; }
  .kick { margin: 0; color: var(--muted); font-size: 12px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; }
  h2 { margin: 8px 0 0; font-size: clamp(26px, 3vw, 36px); font-weight: 800; letter-spacing: -0.035em; line-height: 1.1; }
  .lead { margin: 10px 0 22px; color: #c4c4cc; font-size: 15px; line-height: 1.5; }
  .body { animation: in 0.4s cubic-bezier(0.2, 0.8, 0.2, 1) both; min-width: 0; }
  .nav { display: flex; justify-content: space-between; gap: 10px; margin-top: 26px; }
  .nav .btn { border-radius: 999px; min-height: 46px; padding-inline: 26px; }
  .side { display: grid; gap: 16px; min-width: 0; }
  .tile { padding: 22px; border: 1px solid #ffffff1a; border-radius: 28px; background: #101012; }
  .hd { display: flex; justify-content: space-between; align-items: baseline; }
  .pct { font-size: 22px; font-weight: 800; font-variant-numeric: tabular-nums; }
  .bar { height: 8px; margin: 12px 0 6px; border-radius: 999px; background: #ffffff14; overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 999px; background: #fff; transition: width 0.6s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .chk, .final { display: grid; gap: 8px; margin: 12px 0 0; padding: 0; list-style: none; }
  .chk li, .final li { display: flex; align-items: center; gap: 12px; padding: 11px 14px; border-radius: 18px; background: #0c0c0e; border: 1px solid #ffffff14; }
  .chk li > div, .final li > div { flex: 1; min-width: 0; }
  .chk b, .final b { display: block; font-size: 14px; }
  .chk small, .final small { display: block; overflow-wrap: anywhere; color: var(--muted); font-size: 12px; }
  .chk em { font-size: 12px; font-style: normal; font-weight: 700; color: var(--muted); white-space: nowrap; }
  .chk em.ok { color: #86efac; }
  .chk em.warn { color: #fcd34d; }
  .m { display: grid; place-items: center; flex: none; width: 24px; height: 24px; border-radius: 50%; background: #ffffff1a; font-size: 12px; font-weight: 800; }
  .m.ok { background: #22c55e; color: #04210f; }
  .m.warn { background: #fcd34d; color: #251a02; }
  .m.run { background: #fff; color: #0a0a0b; animation: pulse 1.4s ease-in-out infinite; }
  .big { margin: 10px 0 0; font-size: 48px; font-weight: 800; letter-spacing: -0.05em; line-height: 1; font-variant-numeric: tabular-nums; }
  .big small { color: var(--muted); font-size: 14px; font-weight: 500; letter-spacing: 0; }
  .note { margin: 10px 0 12px; color: #c4c4cc; font-size: 14px; }
  .note b { color: #fff; }
  .pulse { animation: pulse 1.4s ease-in-out infinite; font-size: 22px; }
  .keys { margin: 0; padding: 16px 20px; border: 1px dashed #ffffff33; border-radius: 24px; color: #c4c4cc; font-size: 13px; line-height: 1.5; }
  .keys b { display: block; color: #fff; font-size: 14px; }
  .final + .btn { margin-top: 18px; border-radius: 999px; }
  @keyframes in { from { opacity: 0; transform: translate3d(0, 12px, 0); } }
  @keyframes pulse { 50% { opacity: 0.45; } }
  @media (max-width: 1000px) {
    .grid { grid-template-columns: minmax(0, 1fr); }
    .steps { grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 6px; }
    .steps button { flex-direction: column; padding: 10px 4px; text-align: center; }
    .tx small { display: none; }
    .card { padding: 20px; border-radius: 26px; }
  }
</style>
