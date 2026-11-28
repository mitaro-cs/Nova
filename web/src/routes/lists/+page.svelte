<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  // С 28.11 подборки - раздел «Подборки» (`/my`): отметки и общие подборки на одной странице. Старый адрес ведёт туда.
  onMount(() => goto('/my', { replace: true }));
</script>

<svelte:head><title>Подборки - Nova</title></svelte:head>
