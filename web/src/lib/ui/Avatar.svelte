<script lang="ts">
  import { characterOf } from './characters';

  // Аватар: своё фото (круг) или портрет Nova «Чернила» (36 портретов тушью, номер - по кругу).
  let {
    avatar = 0,
    size = 40,
    photo = null,
  }: { avatar?: number; size?: number; /** Детский режим больше не включается - значок убран, свойство оставлено для старых вызовов. */ kids?: boolean; photo?: string | null } = $props();
  const hero = $derived(characterOf(avatar));
  const uid = $props.id();
</script>

{#if photo}
  <img src={photo} alt="" width={size} height={size} class="ava photo" loading="lazy" decoding="async" />
{:else}
<svg width={size} height={size} viewBox="0 0 128 128" aria-hidden="true" class="ava">
  <defs><clipPath id="c{uid}"><circle cx="64" cy="64" r="64" /></clipPath></defs>
  <circle cx="64" cy="64" r="64" fill={hero.bg} />
  <g clip-path="url(#c{uid})">{@html hero.art}</g>
</svg>
{/if}

<style>
  .ava {
    display: block;
    flex: none;
    border-radius: 50%;
  }
  .photo {
    object-fit: cover;
    background: var(--surface-2);
  }
</style>
