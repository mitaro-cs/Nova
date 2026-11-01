<script lang="ts">
  import ChatList from '$lib/social/ChatList.svelte';
  import Sheet from '$lib/ui/Sheet.svelte';
</script>

<Sheet title="Чаты" subtitle="Переписка с друзьями и фильмы, которыми вы делитесь">
  <div class="rw-block"><ChatList /></div>
</Sheet>
