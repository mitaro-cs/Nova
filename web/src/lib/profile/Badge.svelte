<script lang="ts">
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';

  // Значок-«цветок» (как раньше): восемь лепестков и круг с иконкой в центре, три тона по очереди. Ещё не полученный -
  // пунктирный контур и кольцо: сколько уже пройдено до него.
  let { icon, tone = 0, got = true, progress = 0, size = 64 }: { icon: IconName; tone?: number; got?: boolean; progress?: number; size?: number } = $props();
  const FILL = ['#f4f4f6', '#3d3d44', '#101013'];
  const INK = ['#111', '#f4f4f6', '#f4f4f6'];
  const petals = Array.from({ length: 8 }, (_, k) => ({ x: 18 + 10 * Math.cos((k * Math.PI) / 4), y: 18 + 10 * Math.sin((k * Math.PI) / 4) }));
  const t = $derived(tone % 3);
  const C = 2 * Math.PI * 17;
</script>

<span class="badge" style:width="{size}px" style:height="{size}px">
  <svg viewBox="-1 -1 38 38" aria-hidden="true">
    {#if !got}
      <circle cx="18" cy="18" r="17" fill="none" stroke="#ffffff1f" stroke-width="1.4" />
      <circle cx="18" cy="18" r="17" fill="none" stroke="#fff" stroke-width="1.4" stroke-linecap="round" stroke-dasharray={C} stroke-dashoffset={C * (1 - Math.max(0, Math.min(1, progress)))} transform="rotate(-90 18 18)" />
    {/if}
    <g fill={got ? FILL[t] : 'none'} stroke={got ? (t === 2 ? '#ffffff66' : 'none') : '#ffffff55'} stroke-width="0.8" stroke-dasharray={got ? undefined : '2 2.4'}>
      {#each petals as p, k (k)}<circle cx={p.x} cy={p.y} r="6.6" />{/each}
      <circle cx="18" cy="18" r="11.6" />
    </g>
  </svg>
  <span class="ic" style:color={got ? INK[t] : '#ffffff66'}><Icon name={icon} size={Math.round(size * 0.3)} stroke={1.9} /></span>
</span>

<style>
  .badge { position: relative; display: grid; place-items: center; flex: none; transition: transform 0.3s cubic-bezier(0.2, 0.9, 0.3, 1.4); }
  svg { position: absolute; inset: 0; width: 100%; height: 100%; }
  .ic { position: relative; display: grid; }
</style>
