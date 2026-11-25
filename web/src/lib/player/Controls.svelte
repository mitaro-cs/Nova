<script lang="ts">
  import { clock } from '$lib/fmt';
  import type { Trickplay } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';
  import Seekbar from './Seekbar.svelte';
  import { playbackHeading } from './title';

  // Органы управления - вариант A «Кинотеатр» (выбор владельца, 23.11): сверху всегда название, снизу тонкая полоса
  // перемотки; под ней слева пауза, ±10 с и время, справа одна строка значков с подписями при наведении (звук, озвучка,
  // субтитры, качество текстом, настройки, картинка в картинке, полный экран). Крупные кнопки по центру кадра - только
  // на сенсорных экранах (палец), с мышью их нет. Всё скрывается без движения, кроме паузы.
  let {
    title = '',
    episode = '',
    season = null,
    episodeNo = null,
    shown,
    paused,
    busy,
    panel,
    time,
    duration,
    bufferedPct,
    intro = null,
    trickplay = null,
    fileId = 0,
    marks = [],
    volume,
    muted,
    qualityLabel = '',
    rate,
    showEpisodes = false,
    showAudio = false,
    showSubs = false,
    subsOn = false,
    showQuality = false,
    canNext = false,
    canTogether = false,
    roomBusy = false,
    pipOk = false,
    full = false,
    onback,
    ontogether,
    ontoggle,
    onskip,
    onseek,
    ondrag,
    onvolume,
    onpanel,
    onnext,
    onpip,
    onmoment,
    onfullscreen,
    onheight,
  }: {
    title?: string;
    episode?: string;
    season?: number | null;
    episodeNo?: number | null;
    shown: boolean;
    paused: boolean;
    /** Идёт загрузка: большую кнопку не показываем, крутится индикатор. */
    busy: boolean;
    panel: string;
    time: number;
    duration: number;
    bufferedPct: number;
    intro?: [number, number] | null;
    trickplay?: Trickplay | null;
    fileId?: number;
    marks?: { position: number; emoji: string }[];
    volume: number;
    muted: boolean;
    qualityLabel?: string;
    rate: number;
    showEpisodes?: boolean;
    showAudio?: boolean;
    showSubs?: boolean;
    subsOn?: boolean;
    showQuality?: boolean;
    canNext?: boolean;
    canTogether?: boolean;
    roomBusy?: boolean;
    pipOk?: boolean;
    full?: boolean;
    onback: () => void;
    ontogether: () => void;
    ontoggle: () => void;
    onskip: (delta: number) => void;
    onseek: (t: number) => void;
    ondrag: (dragging: boolean) => void;
    onvolume: (v: number) => void;
    onpanel: (name: string) => void;
    onnext: () => void;
    onpip: () => void;
    /** «Отметить момент»: ссылка на эту секунду - себе или другу. */
    onmoment?: () => void;
    onfullscreen: () => void;
    onheight: (top: number, bottom: number) => void;
  } = $props();

  let topH = $state(80);
  let bottomH = $state(120);
  $effect(() => onheight(topH, bottomH));
  // Перемотка на паузе: крупное название прячется, пока тянут полосу, и на секунду после прыжка.
  let scrub = $state(false);
  let moved = $state(false);
  let movedTimer: ReturnType<typeof setTimeout> | undefined;
  function nudge() {
    moved = true;
    clearTimeout(movedTimer);
    movedTimer = setTimeout(() => (moved = false), 1200);
  }
  // Прыжок клавишами или кнопками ±10 на паузе - тоже прячем название на секунду.
  let lastT = 0;
  $effect(() => {
    const t = time;
    if (paused && Math.abs(t - lastT) > 1) nudge();
    lastT = t;
  });
  // «Закончится в»: время конца при обычной скорости, пересчитывается с каждым шагом ролика.
  const endsAt = $derived(
    duration > 0 && time >= 0 ? new Date(Date.now() + Math.max(0, duration - time) * 1000).toLocaleTimeString('ru-RU', { hour: '2-digit', minute: '2-digit' }) : '',
  );
</script>

<div class="top" class:hide={!shown} bind:clientHeight={topH}>
  <button class="ibtn" aria-label="Назад" onclick={onback}><Icon name="back" size={24} /></button>
  {#if title}
    <div class="name"><b>{playbackHeading(title, episode, season, episodeNo)}</b></div>
  {/if}
  <span class="grow"></span>
  {#if endsAt}<span class="ends">Закончится в {endsAt}</span>{/if}
  {#if canTogether}
    <button class="ibtn" disabled={roomBusy} aria-label="Смотреть вместе" title="Смотреть вместе с текущего места" onclick={ontogether}><Icon name="party" size={22} /></button>
  {/if}
</div>

{#if !busy}
  <div class="center" class:hide={!shown && !paused} class:covered={!!panel} class:rest={paused && !panel}>
    <button class="cbig" aria-label="Назад на 10 секунд" onclick={() => onskip(-10)}>
      <svg class="skip" viewBox="0 0 24 24" aria-hidden="true"><path d="M8.62 5.25A8 8 0 1 0 15.38 5.25M16.54 8.45 15.38 5.25 18.58 4.09" /><text x="12" y="15.5">10</text></svg>
    </button>
    <button class="cbig main" aria-label={paused ? 'Смотреть' : 'Пауза'} onclick={ontoggle}>
      {#key paused}<span class="swap"><Icon name={paused ? 'play' : 'pause'} size={40} /></span>{/key}
    </button>
    <button class="cbig" aria-label="Вперёд на 10 секунд" onclick={() => onskip(10)}>
      <svg class="skip" viewBox="0 0 24 24" aria-hidden="true"><path d="M15.38 5.25A8 8 0 1 1 8.62 5.25M7.46 8.45 8.62 5.25 5.42 4.09" /><text x="12" y="15.5">10</text></svg>
    </button>
  </div>
{/if}

{#if paused && !busy && !panel}
  <!-- Пока перематывают, крупное название уходит: виден кадр, на который встаёт видео. -->
  <div class="paused" class:away={scrub || moved} aria-label="Пауза">
    <small>Пауза</small>
    <h2>{title}</h2>
    {#if episode}<p>{episode}</p>{/if}
    <div><button class="go" type="button" onclick={ontoggle}><Icon name="play" size={18} /> Продолжить</button>{#if duration > time}<span class="left">Осталось {Math.max(1, Math.round((duration - time) / 60))} мин</span>{/if}</div>
  </div>
{/if}

<div class="bottom" class:hide={!shown} bind:clientHeight={bottomH} role="group" aria-label="Управление просмотром">
  <div class="cap">
  <Seekbar {time} {duration} {bufferedPct} {intro} {trickplay} {fileId} {marks} onseek={(t) => { onseek(t); nudge(); }} ondrag={(d) => { scrub = d; ondrag(d); }} />
  <div class="row">
    <button class="ibtn tip mouse" data-tip={paused ? 'Смотреть (пробел)' : 'Пауза (пробел)'} aria-label={paused ? 'Смотреть' : 'Пауза'} onclick={ontoggle}><Icon name={paused ? 'play' : 'pause'} size={22} /></button>
    <button class="ibtn tip mouse" data-tip="Назад 10 с (←)" aria-label="Назад на 10 секунд" onclick={() => onskip(-10)}>
      <svg class="skip sm" viewBox="0 0 24 24" aria-hidden="true"><path d="M8.62 5.25A8 8 0 1 0 15.38 5.25M16.54 8.45 15.38 5.25 18.58 4.09" /><text x="12" y="15.5">10</text></svg>
    </button>
    <button class="ibtn tip mouse" data-tip="Вперёд 10 с (→)" aria-label="Вперёд на 10 секунд" onclick={() => onskip(10)}>
      <svg class="skip sm" viewBox="0 0 24 24" aria-hidden="true"><path d="M15.38 5.25A8 8 0 1 1 8.62 5.25M7.46 8.45 8.62 5.25 5.42 4.09" /><text x="12" y="15.5">10</text></svg>
    </button>
    <span class="time">{clock(time)} <span class="faint">/ {clock(duration)}</span></span>
    <span class="grow"></span>
    {#if canNext}<button class="ibtn tip" data-tip="Следующая серия (N)" aria-label="Следующая серия" onclick={onnext}><Icon name="next" size={22} /></button>{/if}
    {#if showEpisodes}<button class="ibtn tip extra" class:on={panel === 'episodes'} data-tip="Серии" aria-label="Серии" onclick={() => onpanel('episodes')}><Icon name="tv" size={21} /></button>{/if}
    <div class="vol extra">
      <button class="ibtn tip" data-tip={muted ? 'Включить звук (M)' : 'Звук (M)'} aria-label={muted ? 'Включить звук' : 'Выключить звук'} onclick={() => onvolume(muted ? -1 : 0)}>
        <Icon name={muted || volume === 0 ? 'mute' : 'volume'} size={22} />
      </button>
      <input type="range" min="0" max="2" step="0.02" value={muted ? 0 : volume} oninput={(e) => onvolume(Number((e.target as HTMLInputElement).value))} aria-label="Громкость" aria-valuetext={`${Math.round(volume*100)}%`} />
      <span class="volume-label">{Math.round((muted?0:volume)*100)}%</span>
    </div>
    {#if showAudio}<button class="ibtn tip extra" class:on={panel === 'audio'} data-tip="Озвучка" aria-label="Озвучка" onclick={() => onpanel('audio')}><Icon name="audio" size={22} /></button>{/if}
    {#if showSubs}<button class="ibtn tip extra" class:on={panel === 'subs' || subsOn} data-tip="Субтитры (C)" aria-label="Субтитры" onclick={() => onpanel('subs')}><Icon name="captions" size={22} /></button>{/if}
    <!-- Качество - текстом («1080p», «Авто»), а не вторым значком «ползунки»: раньше их было два одинаковых рядом. -->
    {#if showQuality}<button class="ibtn tip q extra" class:on={panel === 'quality'} data-tip="Качество" aria-label="Качество" onclick={() => onpanel('quality')}><span class="qtext">{qualityLabel || 'Авто'}</span></button>{/if}
    <button class="ibtn tip" class:on={panel === 'settings'} data-tip="Настройки" aria-label="Настройки плеера" onclick={() => onpanel('settings')}><Icon name="sliders" size={22} /><span class="slabel">Настройки</span></button>
    {#if onmoment}<button class="ibtn tip extra" data-tip="Отметить момент" aria-label="Отметить момент" onclick={onmoment}><Icon name="bookmark" size={20} /></button>{/if}
    {#if pipOk}<button class="ibtn tip extra" data-tip="Картинка в картинке (P)" aria-label="Картинка в картинке" onclick={onpip}><Icon name="copy" size={20} /></button>{/if}
    <button class="ibtn tip" data-tip={full ? 'Выйти из полного экрана (F)' : 'Во весь экран (F)'} aria-label="Во весь экран" onclick={onfullscreen}><Icon name={full ? 'exitfs' : 'fullscreen'} size={22} /></button>
  </div>
  </div>
</div>

<style>
  .top,
  .bottom {
    position: absolute;
    left: 0;
    right: 0;
    padding: 18px clamp(14px, 3vw, 36px);
    transition:
      opacity 0.3s,
      transform 0.3s;
  }
  .top {
    top: 0;
    /* Выше экрана ошибки: «Назад» и «Смотреть вместе» доступны, даже если видео не играет. */
    z-index: 7;
    display: flex;
    align-items: center;
    gap: 14px;
    padding-top: calc(18px + env(safe-area-inset-top));
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.75), transparent);
  }
  .bottom {
    bottom: 0;
    /* Над слоем паузы: её затемнение не должно гасить значки управления. */
    z-index: 4;
    padding-top: 28px;
    padding-bottom: calc(14px + env(safe-area-inset-bottom));
    background: linear-gradient(0deg, rgba(0, 0, 0, 0.8), transparent);
  }
  .top.hide {
    opacity: 0;
    transform: translateY(-10px);
    pointer-events: none;
  }
  .bottom.hide {
    opacity: 0;
    transform: translateY(10px);
    pointer-events: none;
  }
  .name {
    display: flex;
    align-items: center;
    min-width: 0;
  }
  .name b {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .grow {
    flex: 1;
  }
  .ibtn {
    display: inline-grid;
    place-items: center;
    min-width: 40px;
    height: 40px;
    padding: 0 8px;
    border: 0;
    border-radius: 999px;
    background: none;
    color: #fff;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
  }
  .ibtn:hover,
  .ibtn.on {
    background: rgba(255, 255, 255, 0.16);
  }
  .ibtn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-top: 2px;
  }
  .time {
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .faint {
    color: rgba(255, 255, 255, 0.6);
  }
  .vol {
    display: flex;
    align-items: center;
  }
  .vol input {
    width: 0;
    opacity: 0;
    accent-color: #fff;
    transition:
      width 0.2s,
      opacity 0.2s;
  }
  .vol:hover input,
  .vol:focus-within input {
    width: 84px;
    opacity: 1;
  }
  /* Центр: крупные кнопки, удобно пальцем. Масштаб - как у остального интерфейса (--rw), но не меньше пальца. */
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: clamp(28px, 6vw, 72px);
    pointer-events: none;
    transition: opacity 0.3s;
  }
  .center.hide,
  .center.covered {
    opacity: 0;
  }
  .center.hide .cbig,
  .center.covered .cbig {
    pointer-events: none;
  }
  .cbig {
    pointer-events: auto;
    position: relative;
    display: grid;
    place-items: center;
    width: max(56px, calc(64 * var(--rw)));
    height: max(56px, calc(64 * var(--rw)));
    border: 0;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.14);
    color: #fff;
    cursor: pointer;
  }
  .cbig.main {
    width: max(72px, calc(84 * var(--rw)));
    height: max(72px, calc(84 * var(--rw)));
    background: rgba(255, 255, 255, 0.2);
  }
  .cbig:hover {
    background: rgba(255, 255, 255, 0.28);
  }
  /* Стрелка и «10» - один рисунок: число стоит ровно в центре дуги, а не поверх чужого значка. */
  .skip {
    width: max(42px, calc(46 * var(--rw)));
    height: max(42px, calc(46 * var(--rw)));
    fill: none;
    stroke: #fff;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    pointer-events: none;
  }
  .skip text {
    fill: #fff;
    stroke: none;
    font-size: 6.6px;
    font-weight: 700;
    font-family: inherit;
    text-anchor: middle;
    font-variant-numeric: tabular-nums;
  }
  .swap {
    display: grid;
    animation: swap 0.18s ease-out;
  }
  .volume-label { min-width:34px;font-size:11px;font-variant-numeric:tabular-nums;color:#ccc; }
  @keyframes swap {
    from {
      opacity: 0;
      transform: scale(0.8);
    }
  }
  .q { min-width: 52px; padding-inline: 10px; border: 1px solid #ffffff38; height: 30px; font-size: 13px; font-weight: 700; letter-spacing: 0.01em; }
  .skip.sm { width: 24px; height: 24px; }
  /* Подписи при наведении: белая капсула над значком (у крайних справа - прижата к правому краю). */
  .tip { position: relative; }
  .tip::after {
    content: attr(data-tip); position: absolute; bottom: calc(100% + 10px); left: 50%; z-index: 4;
    padding: 6px 10px; border-radius: 8px; background: #fff; color: #0a0a0b; font-size: 12px; font-weight: 600; white-space: nowrap;
    opacity: 0; transform: translate(-50%, 4px); transition: opacity 0.15s, transform 0.15s; pointer-events: none;
  }
  .row > :last-child.tip::after, .row > .tip:nth-last-child(2)::after { left: auto; right: 0; transform: translateY(4px); }
  @media (hover: hover) and (pointer: fine) {
    .tip:hover::after { opacity: 1; transform: translate(-50%, 0); }
    .row > :last-child.tip:hover::after, .row > .tip:nth-last-child(2):hover::after { transform: none; }
    /* С мышью крупных кнопок по центру нет: пауза и ±10 - в строке внизу. */
    .center { display: none; }
  }
  /* Пальцем: крупные кнопки по центру, в строке внизу их не дублируем. */
  @media not ((hover: hover) and (pointer: fine)) {
    .mouse { display: none; }
  }
  /* Мини-плеер: только значки и название, цифры и подписи не теснят экран. */
  @media (max-width: 700px) {
    .ibtn{min-width:32px;padding-inline:4px}.row{gap:2px}.volume-label{display:none}.time{font-size:11px}

    .vol input {
      display: none;
    }
  }
  /* Минимализм (плееры «Тишина» и «Пауза»): ни плашек, ни рамок - название, время, тонкая линия и одна строка значков;
     на паузе крупно встаёт название с кнопкой «Продолжить»; кадр при этом не темнеет (просьба владельца, 23.11) -
     текст читается за счёт мягкой тени. */
  .ends { display: none; }
  .paused { display: none; }
  @media (min-width: 701px) {
    .top { background: linear-gradient(180deg, #00000099, #00000040 55%, transparent); padding-inline: clamp(18px, 3vw, 44px); padding-bottom: 48px; }
    .top .name { padding: 0; border: 0; background: none; }
    .top .name b { font-size: 15px; opacity: 0.9; }
    .top .ibtn { width: 40px; height: 40px; border: 0; background: none; }
    .top .ibtn:hover { background: #ffffff1f; }
    .ends { display: inline-block; font-size: 13px; font-weight: 500; opacity: 0.6; white-space: nowrap; }
    .bottom { background: linear-gradient(transparent, #000000a6); padding: 60px clamp(18px, 3vw, 44px) calc(30px + env(safe-area-inset-bottom)); }
    .cap { padding: 0; border: 0; border-radius: 0; background: none; }
    .ibtn { opacity: 0.85; }
    .ibtn:hover, .ibtn.on { opacity: 1; }
    .paused {
      position: absolute; inset: 0; z-index: 3; display: flex; flex-direction: column; justify-content: flex-end;
      padding: 0 clamp(28px, 4.5vw, 72px) 150px; pointer-events: none;
      text-shadow: 0 2px 24px #00000080, 0 1px 3px #00000066;
      /* Пауза (просьба владельца 25.11 «совсем нет затемнения»): мягкая тень снизу-слева под названием и кнопкой,
         середина кадра остаётся светлой - лицо и картинка видны. */
      background:
        linear-gradient(90deg, #000000b8 0%, #00000066 34%, transparent 62%),
        linear-gradient(0deg, #000000b0 0%, #00000040 40%, transparent 65%);
      animation: swap 0.25s ease-out;
    }
    .paused * { pointer-events: auto; }
    .paused { transition: opacity 0.25s; }
    .paused.away { opacity: 0; }
    .paused.away * { pointer-events: none; }
    .paused small { font-size: 13px; font-weight: 600; letter-spacing: 0.14em; text-transform: uppercase; opacity: 0.65; }
    .paused h2 { margin: 10px 0 0; max-width: 18ch; font-size: clamp(34px, 4.4vw, 64px); font-weight: 800; letter-spacing: -0.045em; line-height: 1; overflow-wrap: anywhere; }
    .paused p { margin: 12px 0 0; font-size: 18px; opacity: 0.75; }
    .paused .go { display: inline-flex; align-items: center; gap: 10px; margin-top: 24px; padding: 14px 30px; border: 0; border-radius: 999px; background: #fff; color: #0a0a0b; font: inherit; font-size: 16px; font-weight: 700; cursor: pointer; }
    .paused .left { margin-left: 18px; font-size: 14px; opacity: 0.7; }
    /* Большие кнопки по центру на паузе не нужны: есть «Продолжить». */
    .center.covered, .center.hide { opacity: 0; }
  }

  /* ---------- Телефон и планшет (палец), 26.11 «на телефоне всё налезает» ---------- */
  .slabel { display: none; }
  /* Одно меню на всех устройствах (26.11 «всё в одном едином центре, не считая качества»): в строке - пауза, ±10 и время
     слева; справа следующая серия, качество, «Настройки» (озвучка, субтитры, скорость, громкость, серии, момент, картинка в
     картинке) и полный экран. */
  .extra { display: none !important; }
  .slabel { display: inline; margin-left: 6px; font-size: 13px; font-weight: 600; }
  .row > .ibtn:has(.slabel) { display: inline-flex; padding: 0 14px 0 10px; border: 1px solid #ffffff2e; }
  .row > .ibtn.q { display: inline-grid !important; }
  @media not ((hover: hover) and (pointer: fine)) {
    .row > .ibtn.q { display: none !important; }
    /* На паузе - экран паузы с «Продолжить», крупные кнопки по центру не дублируют его. */
    .center.rest { opacity: 0; visibility: hidden; }
    .center.rest .cbig { pointer-events: none; }
  }
  /* Низкий экран (телефон лёжа): всё компактнее, чтобы кадр было видно. */
  @media (max-height: 500px) {
    .top { padding: calc(8px + env(safe-area-inset-top)) max(14px, env(safe-area-inset-right)) 18px max(14px, env(safe-area-inset-left)); gap: 8px; }
    .top .name b { font-size: 14px; }
    .ends { font-size: 12px !important; }
    .bottom { padding: 26px max(14px, env(safe-area-inset-right)) calc(8px + env(safe-area-inset-bottom)) max(14px, env(safe-area-inset-left)) !important; }
    .ibtn { height: 36px; min-width: 36px; }
    .cbig { width: 50px; height: 50px; }
    .cbig.main { width: 64px; height: 64px; }
    .skip { width: 36px; height: 36px; }
    .center { gap: 40px; }
    .paused { padding: 0 max(20px, env(safe-area-inset-left)) calc(var(--bottom-h, 90px) + 14px) !important; }
    .paused small { display: none; }
    .paused h2 { margin: 0 !important; max-width: 60% !important; font-size: 24px !important; letter-spacing: -0.03em !important; }
    .paused p { display: none; }
    .paused .go { margin-top: 12px !important; padding: 10px 20px !important; font-size: 14px !important; }
    .paused .left { font-size: 12px !important; margin-left: 12px !important; }
  }
  /* Телефон стоя: экран паузы тоже есть (раньше - только с 701 px), компактный. */
  @media (max-width: 700px) and (pointer: coarse) {
    .paused {
      position: absolute; inset: 0; z-index: 3; display: flex; flex-direction: column; justify-content: flex-end;
      padding: 0 16px calc(var(--bottom-h, 90px) + 14px); pointer-events: none;
      background: linear-gradient(0deg, #000000c0 0%, #00000040 45%, transparent 70%);
    }
    .paused * { pointer-events: auto; }
    .paused.away { opacity: 0; }
    .paused small, .paused p { display: none; }
    .paused h2 { margin: 0; font-size: 20px; font-weight: 800; line-height: 1.1; }
    .paused .go { display: inline-flex; align-items: center; gap: 8px; margin-top: 10px; padding: 10px 18px; border: 0; border-radius: 999px; background: #fff; color: #0a0a0b; font: inherit; font-size: 14px; font-weight: 700; }
    .paused .left { margin-left: 10px; font-size: 12px; opacity: 0.7; }
  }
</style>
