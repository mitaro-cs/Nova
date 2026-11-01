<script lang="ts">
  import { page } from '$app/state';
  import PlayerShell from '$lib/player/PlayerShell.svelte';

  const id = $derived(Number(page.params.id));
</script>

{#key id}
  <PlayerShell fileId={id} />
{/key}
