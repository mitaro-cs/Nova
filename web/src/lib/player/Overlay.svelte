<script lang="ts">
  import { clock } from '$lib/fmt';
  import { artW } from '$lib/ui/art';
  import Icon from '$lib/ui/Icon.svelte';
  import type { SubLook } from './tracks';

  // Всё, что лежит поверх видео и не является органами управления: ±10 с по двойному касанию,
  // субтитры, индикатор загрузки, подсказка, предупреждения, ошибка, пропуск заставки и «Далее».
  let {
    shown,
    jump = null,
    subtitle = '',
    subLook,
    spinner = false,
    toast = '',
    mismatch = null,
    elsewhere = null,
    mutedByBrowser = false,
    error = '',
    roomError = false,
    failedFile = 0,
    failedLink = false,
    skipTo = null,
    skipCredits = false,
    onskipcredits,
    next = null,
    onback,
    onretry,
    onfail,
    onwrong,
    ondismissmismatch,
    ongoelsewhere,
    onstay,
    onunmute,
    onskipintro,
    onnext,
    onstaycredits,
  }: {
    shown: boolean;
    jump?: { side: 'left' | 'right'; n: number; key: number } | null;
    subtitle?: string;
    subLook: SubLook;
    spinner?: boolean;
    toast?: string;
    mismatch?: { duration: number; expect: number; canReport: boolean } | null;
    elsewhere?: number | null;
    mutedByBrowser?: boolean;
    error?: string;
    roomError?: boolean;
    failedFile?: number;
    failedLink?: boolean;
    skipTo?: number | null;
    /** Начались титры фильма (дальше серии нет) - «Пропустить титры» ведёт к экрану конца. */
    skipCredits?: boolean;
    onskipcredits?: () => void;
    next?: { title: string; still: string | null; countdown: number | null; inCredits: boolean } | null;
    onback: () => void;
    onretry: () => void;
    onfail?: () => void;
    onwrong?: () => void;
    ondismissmismatch: () => void;
    ongoelsewhere: () => void;
    onstay: () => void;
    onunmute: () => void;
    onskipintro: (to: number) => void;
    onnext: () => void;
    onstaycredits: () => void;
  } = $props();
</script>

{#if jump}
  {#key jump.key}
    <div class="jump {jump.side}" aria-hidden="true">
      <Icon name={jump.side === 'right' ? 'forward' : 'rewind'} size={30} />
      <b>{jump.side === 'right' ? '+' : '−'}{jump.n} с</b>
    </div>
  {/key}
{/if}

{#if subtitle}
  <div class="subs {subLook.look} size-{subLook.size}" class:up={shown} class:high={subLook.high}>{subtitle}</div>
{/if}

{#if spinner && !error}
  <div class="spinner" aria-label="Загрузка"></div>
{/if}

{#if toast}<div class="toast" role="status">{toast}</div>{/if}

{#if mismatch}
  <div class="banner" role="status">
    <span>Похоже, это другое видео: <b>{clock(mismatch.duration)}</b> вместо <b>{clock(mismatch.expect)}</b></span>
    {#if mismatch.canReport}<button class="go" onclick={onwrong}>Не тот фильм</button>{/if}
    <button class="go" aria-label="Продолжить просмотр" onclick={ondismissmismatch}>Продолжить</button>
  </div>
{/if}
{#if elsewhere !== null}
  <div class="banner" role="status">
    <span>На другом устройстве вы остановились на <b>{clock(elsewhere)}</b></span>
    <button class="go" onclick={ongoelsewhere}>Перейти</button>
    <button class="x" aria-label="Остаться здесь" onclick={onstay}><Icon name="close" size={16} /></button>
  </div>
{/if}
{#if mutedByBrowser}
  <button class="unmute" onclick={onunmute}><Icon name="audio" size={18} /> Браузер выключил звук - включить</button>
{/if}

{#if error && roomError}
  <div class="room-error" role="status">{error}</div>
{:else if error}
  <div class="err" role="alert">
    <Icon name="info" size={40} />
    <p>{error}</p>
    <div class="row">
      <button class="btn" onclick={onback}><Icon name="back" size={16} /> Назад</button>
      <button class="btn" onclick={onretry}><Icon name="refresh" size={16} /> Повторить</button>
      {#if failedLink}
        {#if onfail}<button class="btn white" onclick={onfail}><Icon name="globe" size={16} /> Плеер сайта</button>{/if}
      {:else if failedFile}
        <a class="btn white" href="/api/files/{failedFile}/download" download><Icon name="download" size={16} /> Скачать</a>
      {/if}
    </div>
  </div>
{/if}

{#if skipTo !== null}
  <button class="skip" class:low={!shown} onclick={() => onskipintro(skipTo)}>Пропустить заставку</button>
{:else if skipCredits && onskipcredits}
  <button class="skip" class:low={!shown} onclick={onskipcredits}>Пропустить титры</button>
{/if}

{#if next}
  <div class="nextcard" class:low={!shown}>
    {#if next.still}<img src={artW(next.still, 640)} alt="" />{/if}
    <div class="nc">
      <span class="muted">Следующая серия</span>
      <b>{next.title}</b>
      <div class="ncb">
        <button class="btn sm" onclick={onstaycredits}>{next.inCredits ? 'Смотреть титры' : 'Остаться'}</button>
        <button class="btn sm white go" onclick={onnext} style:--p="{next.countdown === null ? 0 : ((10 - next.countdown) / 10) * 100}%">
          <Icon name="play" size={13} /> Смотреть{next.countdown !== null ? ` · ${next.countdown}` : ''}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .jump {
    position: absolute;
    top: 50%;
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 18px 24px;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.5);
    transform: translateY(-50%);
    animation: pop 0.65s ease-out forwards;
    pointer-events: none;
  }
  .jump.left {
    left: 14%;
  }
  .jump.right {
    right: 14%;
  }
  @keyframes pop {
    0% {
      opacity: 0;
      transform: translateY(-50%) scale(0.8);
    }
    20% {
      opacity: 1;
      transform: translateY(-50%) scale(1);
    }
    100% {
      opacity: 0;
    }
  }
  .subs {
    position: absolute;
    left: 50%;
    bottom: 6%;
    width: min(86%, 900px);
    transform: translateX(-50%);
    text-align: center;
    white-space: pre-line;
    line-height: 1.35;
    pointer-events: none;
    transition: bottom 0.3s;
    font-weight: 500;
  }
  .subs.up {
    bottom: 130px;
  }
  .subs.high {
    bottom: 18%;
  }
  .subs.size-s {
    font-size: clamp(14px, 1.6vw, 20px);
  }
  .subs.size-m {
    font-size: clamp(17px, 2.1vw, 28px);
  }
  .subs.size-l {
    font-size: clamp(21px, 2.8vw, 38px);
  }
  .subs.box {
    color: #fff;
    background: rgba(0, 0, 0, 0.62);
    padding: 4px 12px;
    border-radius: 8px;
    width: fit-content;
    max-width: 86%;
  }
  .subs.shadow {
    color: #fff;
    text-shadow: 0 1px 4px #000, 0 0 8px #000;
  }
  .subs.yellow {
    color: #ffe14d;
    text-shadow: 0 1px 4px #000;
  }
  .subs.cinema {
    color: #fff;
    -webkit-text-stroke: 1px #000;
    paint-order: stroke fill;
    text-shadow: 0 2px 6px #000;
  }
  .spinner {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 48px;
    height: 48px;
    margin: -24px 0 0 -24px;
    border: 3px solid rgba(255, 255, 255, 0.25);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
    pointer-events: none;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .toast {
    position: absolute;
    top: 14%;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 16px;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.7);
    font-size: 14px;
    pointer-events: none;
  }
  .banner {
    position: absolute;
    top: 84px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: calc(100% - 32px);
    padding: 10px 14px;
    border-radius: 14px;
    background: rgba(24, 24, 24, 0.92);
    font-size: 14px;
    z-index: 4;
  }
  .banner .go,
  .banner .x,
  .unmute {
    border: 0;
    cursor: pointer;
    color: #000;
    background: #fff;
    border-radius: 999px;
    padding: 6px 12px;
    font: inherit;
    font-weight: 600;
  }
  .banner .x {
    display: grid;
    place-items: center;
    padding: 6px;
    background: rgba(255, 255, 255, 0.16);
    color: #fff;
  }
  .unmute {
    position: absolute;
    top: 84px;
    right: 16px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    z-index: 4;
  }
  .room-error {
    position: absolute;
    top: 82px;
    left: 16px;
    right: 64px;
    max-height: 88px;
    overflow: auto;
    padding: 10px 12px;
    border-radius: 12px;
    background: #24191a;
    color: #ffd6d6;
    font-size: 12px;
    z-index: 5;
  }
  .err {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 12px;
    padding: 24px;
    text-align: center;
    background: rgba(0, 0, 0, 0.86);
    z-index: 6;
  }
  .err p {
    max-width: 420px;
    margin: 0;
    font-size: 16px;
  }
  .err .row {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 10px;
  }
  .skip {
    position: absolute;
    right: clamp(14px, 3vw, 36px);
    bottom: 140px;
    padding: 10px 20px;
    border: 1px solid rgba(255, 255, 255, 0.5);
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    transition: bottom 0.3s;
  }
  .skip.low {
    bottom: 40px;
  }
  .nextcard {
    position: absolute;
    right: clamp(14px, 3vw, 36px);
    bottom: 140px;
    display: flex;
    gap: 12px;
    width: min(420px, calc(100% - 28px));
    padding: 10px;
    border-radius: 16px;
    background: rgba(20, 20, 20, 0.92);
    transition: bottom 0.3s;
    z-index: 3;
  }
  .nextcard.low {
    bottom: 40px;
  }
  .nextcard img {
    width: 130px;
    aspect-ratio: 16 / 9;
    object-fit: cover;
    border-radius: 8px;
  }
  .nc {
    display: grid;
    gap: 6px;
    min-width: 0;
    align-content: center;
  }
  .nc b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ncb {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .go {
    background-image: linear-gradient(90deg, rgba(0, 0, 0, 0.14) var(--p, 0%), transparent var(--p, 0%));
  }
  .muted {
    color: rgba(255, 255, 255, 0.65);
    font-size: 12px;
  }
</style>
