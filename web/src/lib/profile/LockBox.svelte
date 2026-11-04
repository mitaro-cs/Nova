<script lang="ts">
  import Icon from '$lib/ui/Icon.svelte';

  // Раздел профиля, который человек закрыл: замок и «Скрыто владельцем». Сервер скрытого не отдаёт вовсе,
  // размытые столбики на фоне - только украшение.
  let { text = 'Скрыто владельцем' }: { text?: string } = $props();
  const GHOST = [30, 62, 44, 80, 52, 70, 38, 66, 48];
</script>

<div class="lock" role="note">
  <div class="ghost" aria-hidden="true">{#each GHOST as h, i (i)}<i style:height="{h}%"></i>{/each}</div>
  <span class="ic"><Icon name="lock" size={30} /></span>
  <b>{text}</b>
</div>

<style>
  .lock {
    position: relative;
    display: grid;
    justify-items: center;
    gap: 8px;
    padding: 38px 16px;
    border: 1px dashed var(--line-strong, #ffffff40);
    border-radius: 18px;
    overflow: hidden;
    text-align: center;
    color: var(--muted);
  }
  .ghost {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-end;
    gap: 10px;
    padding: 16px;
    opacity: 0.16;
    filter: blur(3px);
  }
  .ghost i {
    flex: 1;
    border-radius: 6px;
    background: #fff;
  }
  .lock > :not(.ghost) {
    position: relative;
  }
  .ic {
    color: var(--text);
    animation: shake 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) 0.2s both;
  }
  b {
    color: #dcdcdf;
    font-weight: 600;
  }
  @keyframes shake {
    20% {
      transform: rotate(-12deg);
    }
    45% {
      transform: rotate(10deg);
    }
    70% {
      transform: rotate(-5deg);
    }
  }
</style>
