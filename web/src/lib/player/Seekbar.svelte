<script lang="ts">
  import { clock } from '$lib/fmt';
  import type { Trickplay } from '$lib/types';
  import { trickFrame } from './levels';

  // Полоса перемотки: загруженное, зона заставки, кадр под курсором (как у Netflix) или время.
  let {
    time,
    duration,
    bufferedPct,
    intro = null,
    trickplay = null,
    fileId = 0,
    marks = [],
    onseek,
    ondrag,
  }: {
    time: number;
    duration: number;
    bufferedPct: number;
    intro?: [number, number] | null;
    trickplay?: Trickplay | null;
    fileId?: number;
    /** Реакции зала (комната): смайлик над полосой на своей секунде, нажатие - перейти к моменту. */
    marks?: { position: number; emoji: string }[];
    onseek: (t: number) => void;
    ondrag?: (dragging: boolean) => void;
  } = $props();

  let hover = $state<number | null>(null);
  let barWidth = $state(0);
  let dragging = $state(false);

  const playedPct = $derived(duration ? Math.min(100, (time / duration) * 100) : 0);
  const thumb = $derived(hover !== null && trickplay ? trickFrame(trickplay, hover) : null);
  const thumbLeft = $derived.by(() => {
    if (hover === null || !duration) return 0;
    const w = trickplay?.width ?? 0;
    const x = (hover / duration) * barWidth;
    return Math.max(w / 2, Math.min(barWidth - w / 2, x));
  });

  function pos(e: PointerEvent, el: HTMLElement) {
    const r = el.getBoundingClientRect();
    return Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * duration;
  }
  function down(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    dragging = true;
    ondrag?.(true);
    hover = pos(e, el);
  }
  function move(e: PointerEvent) {
    hover = pos(e, e.currentTarget as HTMLElement);
  }
  function up(e: PointerEvent) {
    if (dragging) onseek(pos(e, e.currentTarget as HTMLElement));
    dragging = false;
    ondrag?.(false);
  }
</script>

<div
  class="bar"
  role="slider"
  tabindex="-1"
  aria-label="Перемотка"
  aria-valuemin={0}
  aria-valuemax={Math.round(duration)}
  aria-valuenow={Math.round(time)}
  bind:clientWidth={barWidth}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointerleave={() => !dragging && (hover = null)}
>
  <div class="rail">
    <i class="buf" style:width="{bufferedPct}%"></i>
    {#if intro && duration}<i class="introzone" style:left="{(intro[0] / duration) * 100}%" style:width="{((intro[1] - intro[0]) / duration) * 100}%"></i>{/if}
    <i class="played" style:width="{dragging && hover !== null && duration ? (hover / duration) * 100 : playedPct}%"></i>
  </div>
  {#if marks.length && duration}
    <span class="marks" aria-hidden="true">
      {#each marks.filter((m) => m.position <= duration) as m, i (i)}<span class="mk" style:left="{(m.position / duration) * 100}%" style:--lift="{(i % 3) * 14}px">{m.emoji}</span>{/each}
    </span>
  {/if}
  {#if hover !== null && duration}
    {#if thumb && trickplay}
      <span
        class="thumb"
        style:left="{thumbLeft}px"
        style:width="{trickplay.width}px"
        style:height="{trickplay.height}px"
        style:background-image="url(/api/files/{fileId}/trickplay/{thumb.sheet}.jpg)"
        style:background-position="-{thumb.x}px -{thumb.y}px"
      ><span>{clock(hover)}</span></span>
    {:else}
      <span class="tip" style:left="{(hover / duration) * 100}%">{clock(hover)}</span>
    {/if}
  {/if}
</div>

<style>
  .marks { position: absolute; left: 0; right: 0; bottom: 100%; height: 0; pointer-events: none; }
  .mk { position: absolute; bottom: calc(2px + var(--lift)); font-size: 15px; line-height: 1; transform: translateX(-50%); filter: drop-shadow(0 2px 4px #0009); animation: mk-in 0.4s cubic-bezier(0.2, 0.9, 0.3, 1.4) both; }
  @keyframes mk-in { from { opacity: 0; transform: translate(-50%, 6px) scale(0.6); } }
  .bar {
    position: relative;
    height: 24px;
    display: flex;
    align-items: center;
    cursor: pointer;
    touch-action: none;
  }
  .rail {
    position: relative;
    width: 100%;
    height: 3px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.22);
    transition: height 0.15s;
  }
  .bar:hover .rail {
    height: 5px;
  }
  .rail i {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 3px;
  }
  .buf {
    background: rgba(255, 255, 255, 0.35);
  }
  .introzone {
    background: rgba(255, 255, 255, 0.5);
    border-radius: 0;
  }
  .played {
    background: #fff;
  }
  .played::after {
    content: '';
    position: absolute;
    right: -6px;
    top: 50%;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #fff;
    transform: translateY(-50%) scale(0);
    transition: transform 0.15s;
  }
  .bar:hover .played::after {
    transform: translateY(-50%) scale(1);
  }
  .tip,
  .thumb {
    position: absolute;
    bottom: 28px;
    transform: translateX(-50%);
    pointer-events: none;
  }
  .tip {
    padding: 3px 8px;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.8);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .thumb {
    background-repeat: no-repeat;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.3);
    overflow: hidden;
  }
  .thumb span {
    position: absolute;
    inset: auto 0 4px 0;
    text-align: center;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-shadow: 0 1px 3px #000;
  }
</style>
