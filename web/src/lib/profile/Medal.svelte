<script lang="ts">
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';

  // Значок-медаль: «цветок» из девяти кругов и иконка в центре. Три тона: светлый, графитовый, тёмный с обводкой.
  let { icon, tone = 0, label, i = 0 }: { icon: IconName; tone?: 0 | 1 | 2; label: string; i?: number } = $props();
  const FILL = ['#f4f4f6', '#3d3d44', '#101013'];
  const INK = ['#111', '#f4f4f6', '#f4f4f6'];
  const petals = Array.from({ length: 8 }, (_, k) => ({ x: 18 + 10 * Math.cos((k * Math.PI) / 4), y: 18 + 10 * Math.sin((k * Math.PI) / 4) }));
</script>

<span class="medal" style:--i={i} title={label} role="img" aria-label={label}>
  <svg viewBox="0 0 36 36" aria-hidden="true">
    {#each petals as p, k (k)}<circle cx={p.x} cy={p.y} r="6.6" fill={FILL[tone]} stroke={tone === 2 ? '#ffffff66' : 'none'} />{/each}
    <circle cx="18" cy="18" r="11.6" fill={FILL[tone]} />
  </svg>
  <span class="ic" style:color={INK[tone]}><Icon name={icon} size={17} stroke={1.9} /></span>
</span>

<style>
  .medal {
    position: relative;
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    animation: pop 0.6s cubic-bezier(0.2, 0.9, 0.3, 1.4) both;
    animation-delay: calc(var(--i) * 110ms + 600ms);
    transition: transform 0.3s cubic-bezier(0.2, 0.9, 0.3, 1.4);
  }
  .medal:hover {
    transform: rotate(-12deg) scale(1.12);
  }
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .ic {
    position: relative;
    display: grid;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.3);
    }
  }
</style>
