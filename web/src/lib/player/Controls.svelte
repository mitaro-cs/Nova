<script lang="ts">
  import { clock } from '$lib/fmt';
  import type { Trickplay } from '$lib/types';
  import Icon from '$lib/ui/Icon.svelte';
  import Seekbar from './Seekbar.svelte';

  // Органы управления - вариант C: сверху название, по центру крупные «назад 10 с», пауза и «вперёд 10 с»,
  // снизу тонкая полоса перемотки, время слева и значки справа. Всё скрывается без движения.
  let {
    title = '',
    episode = '',
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
  const rateText = $derived(`×${String(rate).replace('.', ',')}`);
</script>

<div class="top" class:hide={!shown} bind:clientHeight={topH}>
  <button class="ibtn" aria-label="Назад" onclick={onback}><Icon name="back" size={24} /></button>
  {#if title}
    <div class="name">
      <b>{title}</b>
      {#if episode}<span>{episode}</span>{/if}
    </div>
  {/if}
  <span class="grow"></span>
  {#if canTogether}
    <button class="ibtn" disabled={roomBusy} aria-label="Смотреть вместе" title="Смотреть вместе с текущего места" onclick={ontogether}><Icon name="party" size={22} /></button>
  {/if}
</div>

{#if !busy}
  <div class="center" class:hide={!shown && !paused} class:covered={!!panel}>
    <button class="cbig" aria-label="Назад на 10 секунд" onclick={() => onskip(-10)}><Icon name="rewind" size={34} /><small>10</small></button>
    <button class="cbig main" aria-label={paused ? 'Смотреть' : 'Пауза'} onclick={ontoggle}>
      {#key paused}<span class="swap"><Icon name={paused ? 'play' : 'pause'} size={40} /></span>{/key}
    </button>
    <button class="cbig" aria-label="Вперёд на 10 секунд" onclick={() => onskip(10)}><Icon name="forward" size={34} /><small>10</small></button>
  </div>
{/if}

<div class="bottom" class:hide={!shown} bind:clientHeight={bottomH} role="group" aria-label="Управление просмотром">
  <Seekbar {time} {duration} {bufferedPct} {intro} {trickplay} {fileId} {onseek} {ondrag} />
  <div class="row">
    <span class="time">{clock(time)} <span class="faint">/ {clock(duration)}</span></span>
    <span class="grow"></span>
    <div class="vol">
      <button class="ibtn" aria-label={muted ? 'Включить звук' : 'Выключить звук'} onclick={() => onvolume(muted ? -1 : 0)}>
        <Icon name={muted || volume === 0 ? 'mute' : 'volume'} size={22} />
      </button>
      <input type="range" min="0" max="1" step="0.02" value={muted ? 0 : volume} oninput={(e) => onvolume(Number((e.target as HTMLInputElement).value))} aria-label="Громкость" />
    </div>
    {#if showEpisodes}<button class="ibtn" class:on={panel === 'episodes'} aria-label="Сезоны и серии" title="Сезоны и серии" onclick={() => onpanel('episodes')}><Icon name="tv" size={22} /></button>{/if}
    {#if showSubs}<button class="ibtn" class:on={panel === 'subs' || subsOn} aria-label="Субтитры" title="Субтитры" onclick={() => onpanel('subs')}><Icon name="captions" size={22} /></button>{/if}
    {#if showQuality}<button class="ibtn q" class:on={panel === 'quality'} aria-label="Качество" title="Качество" onclick={() => onpanel('quality')}>{qualityLabel}</button>{/if}
    <button class="ibtn q" class:on={panel === 'speed' || rate !== 1} aria-label="Скорость" title="Скорость" onclick={() => onpanel('speed')}>{rateText}</button>
    <button class="ibtn" class:on={panel === 'settings'} aria-label="Настройки плеера" title="Настройки" onclick={() => onpanel('settings')}><Icon name="sliders" size={22} /></button>
    {#if canNext}<button class="ibtn" aria-label="Следующая серия" title="Следующая серия (N)" onclick={onnext}><Icon name="next" size={22} /></button>{/if}
    {#if pipOk}<button class="ibtn hide-sm" aria-label="Картинка в картинке" title="Картинка в картинке (P)" onclick={onpip}><Icon name="copy" size={20} /></button>{/if}
    <button class="ibtn hide-sm" aria-label="Горячие клавиши" title="Горячие клавиши (?)" onclick={() => onpanel('keys')}><Icon name="info" size={20} /></button>
    <button class="ibtn" aria-label="Во весь экран" onclick={onfullscreen}><Icon name={full ? 'exitfs' : 'fullscreen'} size={22} /></button>
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
    display: grid;
    line-height: 1.25;
    min-width: 0;
  }
  .name b {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 17px;
    font-weight: 600;
  }
  .name span {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.7);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
  .cbig small {
    position: absolute;
    font-size: 11px;
    font-weight: 700;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -38%);
    pointer-events: none;
  }
  .swap {
    display: grid;
    animation: swap 0.18s ease-out;
  }
  @keyframes swap {
    from {
      opacity: 0;
      transform: scale(0.8);
    }
  }
  @media (max-width: 700px) {
    .hide-sm {
      display: none;
    }
    .vol input {
      display: none;
    }
  }
</style>
