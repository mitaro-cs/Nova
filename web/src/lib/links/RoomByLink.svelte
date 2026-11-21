<script lang="ts">
  import { goto } from '$app/navigation';
  import { errorText, post } from '$lib/api';
  import { toast } from '$lib/ui/toast.svelte';

  // «Создать комнату» (владелец, 21.11): сначала комната, видео потом. По событию `nova-room-link` сразу создаём
  // пустую комнату и открываем её; фильмы и ссылки добавляют уже внутри, во вкладке «Очередь».
  let busy = false;
  $effect(() => {
    const create = async () => {
      if (busy) return;
      busy = true;
      try {
        const room = await post<{ id: string }>('/api/rooms', {});
        await goto(`/room/${room.id}`);
      } catch (x) {
        toast(errorText(x));
      } finally {
        busy = false;
      }
    };
    window.addEventListener('nova-room-link', create);
    return () => window.removeEventListener('nova-room-link', create);
  });
</script>
