<script lang="ts">
  import { clock } from '$lib/fmt';
  import type { Trickplay } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';
  import Seekbar from './Seekbar.svelte';
  import { playbackHeading } from './title';

  // Органы управления - вариант C: сверху название, по центру крупные «назад 10 с», пауза и «вперёд 10 с»,
  // снизу тонкая полоса перемотки, время слева и значки справа. Всё скрывается без движения.
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
    onfullscreen: () => void;
    onheight: (top: number, bottom: number) => void;
  } = $props();

  let topH = $state(80);
  let bottomH = $state(120);
  $effect(() => onheight(topH, bottomH));
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
  <div class="center" class:hide={!shown && !paused} class:covered={!!panel}>
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

<div class="bottom" class:hide={!shown} bind:clientHeight={bottomH} role="group" aria-label="Управление просмотром">
  <div class="cap">
  <Seekbar {time} {duration} {bufferedPct} {intro} {trickplay} {fileId} {onseek} {ondrag} />
  <div class="row">
    <span class="time">{clock(time)} <span class="faint">/ {clock(duration)}</span></span>
    <span class="grow"></span>
    <div class="vol">
      <button class="ibtn" aria-label={muted ? 'Включить звук' : 'Выключить звук'} onclick={() => onvolume(muted ? -1 : 0)}>
        <Icon name={muted || volume === 0 ? 'mute' : 'volume'} size={22} />
      </button>
      <input type="range" min="0" max="2" step="0.02" value={muted ? 0 : volume} oninput={(e) => onvolume(Number((e.target as HTMLInputElement).value))} aria-label="Громкость" aria-valuetext={`${Math.round(volume*100)}%`} />
      <span class="volume-label">{Math.round((muted?0:volume)*100)}%</span>
    </div>
    {#if showAudio}<button class="ibtn" class:on={panel === 'audio'} aria-label="Озвучка" title="Озвучка" onclick={() => onpanel('audio')}><Icon name="audio" size={22} /></button>{/if}
    {#if showQuality}<button class="ibtn q" class:on={panel === 'quality'} aria-label="Качество" title="Качество" onclick={() => onpanel('quality')}><span class="qicon"><Icon name="sliders" size={20} /></span><span class="qtext">{qualityLabel}</span></button>{/if}
    <button class="ibtn" class:on={panel === 'settings'} aria-label="Настройки плеера" title="Настройки" onclick={() => onpanel('settings')}><Icon name="sliders" size={22} /></button>
    {#if pipOk}<button class="ibtn" aria-label="Картинка в картинке" title="Картинка в картинке (P)" onclick={onpip}><Icon name="copy" size={20} /></button>{/if}
    <button class="ibtn" aria-label="Во весь экран" onclick={onfullscreen}><Icon name={full ? 'exitfs' : 'fullscreen'} size={22} /></button>
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
  .q { gap: 6px; }
  .qicon { display: none; }
  /* Мини-плеер: только значки и название, цифры и подписи не теснят экран. */
  @media (max-width: 700px) {
    .ibtn{min-width:32px;padding-inline:4px}.row{gap:2px}.volume-label{display:none}.time{font-size:11px}
    .skip text{display:none}
    .q .qtext{display:none}
    .q .qicon{display:inline-grid}
    .vol input {
      display: none;
    }
  }
  @media (min-width: 1000px) {
    .center { inset: auto 35% 13px; height: 44px; gap: 8px; z-index: 2; }
    .cbig, .cbig.main { width: 42px; height: 42px; }
    .cbig:not(.main) { background: transparent; }
    .cbig.main :global(svg) { width: 22px; height: 22px; }
    .skip { width: 24px; height: 24px; }
  }
  /* Дизайн «Стекло» (плеер D): название и время парят плашками, управление - одна капсула снизу. */
  .ends { display: none; }
  @media (min-width: 701px) {
    .top { background: none; padding-inline: clamp(18px, 3vw, 40px); }
    .top .name { padding: 8px 20px; border: 1px solid #ffffff26; border-radius: 999px; background: #0007; }
    .top .ibtn { width: 46px; height: 46px; border: 1px solid #ffffff26; background: #0007; }
    .ends { display: inline-block; padding: 10px 18px; border: 1px solid #ffffff26; border-radius: 999px; background: #0007; font-size: 14px; font-weight: 600; white-space: nowrap; }
    .bottom { background: none; padding: 0 clamp(18px, 3vw, 40px) calc(28px + env(safe-area-inset-bottom)); }
    .cap { padding: 16px 26px 10px; border: 1px solid #ffffff26; border-radius: 32px; background: #0c0c0ed9; }
  }
  @media (min-width: 1000px) {
    .center { inset: auto 35% 38px; }
  }
</style>
