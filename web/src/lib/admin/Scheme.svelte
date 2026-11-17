<script lang="ts">
  import { size } from '$lib/fmt';
  import Icon from '$lib/ui/Icon.svelte';
  import type { IconName } from '$lib/ui/icons';
  import type { Status } from './types';

  // Схема работы Nova: файлы → ffmpeg → сервер → доступ → зрители. Каждый узел горит зелёным, если работает,
  // красным, если сломан, и серым, если не используется (или ещё ждёт). По связям между узлами бегут точки, пока
  // обе стороны в порядке; если рядом поломка - линия красная и пунктирная, точки стоят. Так видно, где именно
  // рвётся путь от фильма на диске до зрителя. Двигаются только `transform` и `opacity`.
  let { status }: { status: Status } = $props();

  type State = 'ok' | 'bad' | 'idle';
  type Node = { key: string; icon: IconName; title: string; value: string; note: string; state: State; href?: string };

  const GB = 1024 ** 3;
  const low = $derived(status.system?.diskFree != null && status.system.diskFree < 10 * GB);
  const ffmpegOk = $derived(status.ffmpeg && status.ffprobe);
  const host = $derived.by(() => {
    if (!status.publicUrl) return '';
    try {
      return new URL(status.publicUrl).host;
    } catch {
      return status.publicUrl;
    }
  });

  const nodes = $derived<Node[]>([
    {
      key: 'files',
      icon: 'folder',
      title: 'Файлы',
      value: String(status.titles),
      note: status.scan.error ? status.scan.error : low ? 'Мало места на диске' : status.titles ? `${status.files} файлов · ${size(status.size)}` : 'Добавьте папку с фильмами',
      state: status.scan.error || low ? 'bad' : status.titles ? 'ok' : 'idle',
      href: '?tab=folders',
    },
    {
      key: 'ffmpeg',
      icon: 'film',
      title: 'ffmpeg',
      value: ffmpegOk ? 'Готов' : 'Нет',
      note: ffmpegOk ? 'MKV, озвучки, качество' : 'MKV и AVI не играют',
      state: ffmpegOk ? 'ok' : 'bad',
      href: ffmpegOk ? undefined : '?tab=settings',
    },
    {
      key: 'server',
      icon: 'pulse',
      title: 'Сервер',
      value: `Nova ${status.version}`,
      note: status.system?.cpu != null ? `Процессор ${Math.round(status.system.cpu)}%` : 'раздаёт фильмы',
      state: 'ok',
    },
    {
      key: 'access',
      icon: 'globe',
      title: 'Доступ',
      value: status.publicUrl ? 'Интернет' : 'Только дом',
      note: status.publicUrl ? host : 'из интернета не видно',
      state: status.publicUrl ? 'ok' : 'idle',
      href: '?tab=access',
    },
    {
      key: 'viewers',
      icon: 'users',
      title: 'Зрители',
      value: String(status.watching.length),
      note: status.watching.length ? 'смотрят сейчас' : 'сейчас никто не смотрит',
      state: status.watching.length ? 'ok' : 'idle',
    },
  ]);
  const links = $derived(
    nodes.slice(0, -1).map((n, i): State => {
      const b = nodes[i + 1];
      return n.state === 'bad' || b.state === 'bad' ? 'bad' : n.state === 'ok' && b.state === 'ok' ? 'ok' : 'idle';
    }),
  );
</script>

<section class="scheme" aria-label="Схема работы Nova">
  <h3><Icon name="pulse" size={18} /> Как это работает сейчас</h3>
  <p class="muted small">Путь фильма от диска до зрителя. Зелёное - работает, красное - сломано и надо исправить, серое - не используется.</p>
  <ol>
    {#each nodes as n, i (n.key)}
      <li class="node {n.state}">
        <svelte:element this={n.href ? 'a' : 'div'} class="box" href={n.href} data-sveltekit-replacestate={n.href ? '' : undefined} data-sveltekit-noscroll={n.href ? '' : undefined}>
          <span class="ic"><Icon name={n.icon} size={22} /></span>
          <span class="txt">
            <small>{n.title}</small>
            <b>{n.value}</b>
            <em>{n.note}</em>
          </span>
          <i class="lamp" aria-label={n.state === 'ok' ? 'работает' : n.state === 'bad' ? 'ошибка' : 'не используется'}></i>
        </svelte:element>
      </li>
      {#if i < links.length}
        <li class="link {links[i]}" aria-hidden="true"><i></i><i></i><i></i></li>
      {/if}
    {/each}
  </ol>
</section>

<style>
  .scheme {
    margin-bottom: 18px;
    padding: 18px 20px 22px;
    border-radius: 28px;
    background: #101012;
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 4px;
    font-size: 16px;
  }
  .small {
    margin: 0 0 18px;
    font-size: 13px;
  }
  ol {
    display: flex;
    align-items: stretch;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .node {
    flex: 1 1 0;
    min-width: 0;
    display: grid;
  }
  .box {
    position: relative;
    display: grid;
    justify-items: center;
    align-content: start;
    gap: 10px;
    min-width: 0;
    padding: 18px 12px 16px;
    border: 1px solid var(--line);
    border-radius: 22px;
    background: #ffffff08;
    color: inherit;
    text-align: center;
    transition:
      transform 0.3s var(--ease),
      background 0.25s;
  }
  a.box:hover {
    background: #ffffff12;
    transform: translateY(-2px);
  }
  .txt {
    display: grid;
    gap: 2px;
    min-width: 0;
    max-width: 100%;
  }
  .txt small {
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .txt b {
    font-size: clamp(17px, 1.6vw, 22px);
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }
  .txt em {
    color: var(--faint);
    font-size: 12px;
    font-style: normal;
    overflow-wrap: anywhere;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: #ffffff14;
    color: #d4d4d8;
  }
  .lamp {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: #71717a;
  }
  .node.ok .ic {
    background: color-mix(in srgb, var(--ok) 20%, transparent);
    color: var(--ok);
  }
  .node.ok .lamp {
    background: var(--ok);
  }
  .node.bad .box {
    border-color: color-mix(in srgb, var(--danger) 55%, transparent);
    background: color-mix(in srgb, var(--danger) 8%, transparent);
  }
  .node.bad .ic {
    background: color-mix(in srgb, var(--danger) 22%, transparent);
    color: var(--danger);
  }
  .node.bad .lamp {
    background: var(--danger);
    animation: blink 1.6s ease-in-out infinite;
  }
  .node.bad .txt em {
    color: var(--danger);
  }

  /* Связь: тонкая линия, по которой в сторону зрителя бегут три точки. */
  .link {
    position: relative;
    flex: 0 0 clamp(26px, 4vw, 64px);
    align-self: center;
    height: 8px;
    overflow: hidden;
    --line-bg: #ffffff26;
  }
  .link::before {
    content: '';
    position: absolute;
    inset: 3px 0;
    background: var(--line-bg);
  }
  .link i {
    position: absolute;
    inset: 1px 0 auto;
    height: 6px;
    animation: flow 2.1s linear infinite;
    animation-delay: calc(var(--k, 0) * 0.7s);
    opacity: 0;
  }
  .link i::after {
    content: '';
    position: absolute;
    right: 0;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }
  .link i:nth-child(2) {
    --k: 1;
  }
  .link i:nth-child(3) {
    --k: 2;
  }
  .link.ok {
    --line-bg: color-mix(in srgb, var(--ok) 35%, transparent);
  }
  .link:not(.ok) i {
    display: none;
  }
  .link.bad {
    --line-bg: repeating-linear-gradient(90deg, var(--danger) 0 5px, transparent 5px 9px);
  }
  @keyframes flow {
    from {
      transform: translateX(-100%);
      opacity: 0;
    }
    15%,
    85% {
      opacity: 1;
    }
    to {
      transform: translateX(0);
      opacity: 0;
    }
  }
  @keyframes flow-down {
    from {
      transform: translateY(-100%);
      opacity: 0;
    }
    15%,
    85% {
      opacity: 1;
    }
    to {
      transform: translateY(0);
      opacity: 0;
    }
  }
  @keyframes blink {
    50% {
      opacity: 0.4;
    }
  }
  :global(.no-motion) .link i {
    animation: none;
    opacity: 1;
  }
  @media (max-width: 860px) {
    ol {
      flex-direction: column;
      align-items: stretch;
    }
    .box {
      grid-template-columns: auto minmax(0, 1fr);
      justify-items: start;
      align-items: center;
      gap: 14px;
      padding: 12px 36px 12px 14px;
      text-align: left;
    }
    .link {
      flex: none;
      align-self: flex-start;
      width: 8px;
      height: 26px;
      margin-left: 34px;
    }
    .link::before {
      inset: 0 3px;
    }
    .link i {
      inset: 0 auto 0 1px;
      width: 6px;
      height: auto;
      animation-name: flow-down;
    }
    .link i::after {
      right: auto;
      bottom: 0;
    }
    .link.bad {
      --line-bg: repeating-linear-gradient(180deg, var(--danger) 0 5px, transparent 5px 9px);
    }
  }
</style>
