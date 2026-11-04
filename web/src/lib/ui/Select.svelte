<script lang="ts">
  import Icon from './Icon.svelte';

  // Свой выпадающий список вместо системного: браузер рисует системный по-своему (серый, с рамкой),
  // а в окне приложения и на телефоне он выглядит чужим. Панель открывается поверх всего (popover -
  // верхний слой, её не обрезают ни плита, ни окно <dialog>) и ведёт себя как привычный select:
  // стрелки, Enter, Esc, выбранное отмечено галочкой.
  type Option = { value: string; label: string };

  let {
    value = $bindable(''),
    options,
    label = '',
    variant = 'box',
    fit = false,
    disabled = false,
    class: klass = '',
    onchange,
  }: {
    value?: string;
    options: Option[];
    /** Подпись для чтения с экрана (если рядом нет <label>). */
    label?: string;
    /** box - как поле ввода, sm - то же, но ниже, inline - плашка внутри строки (фильтры). Не `field`/`chip`: такие классы уже есть в app.css. */
    variant?: 'box' | 'sm' | 'inline';
    /** По ширине содержимого, а не на всю ширину. */
    fit?: boolean;
    disabled?: boolean;
    class?: string;
    onchange?: (value: string) => void;
  } = $props();

  const uid = `sel-${Math.random().toString(36).slice(2, 9)}`;
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();
  let open = $state(false);
  let active = $state(0);
  let pos = $state({ left: 0, top: 0, bottom: 0, up: false, minWidth: 0, maxHeight: 320 });
  let closedAt = 0;
  let openedAt = 0;

  const index = $derived(Math.max(0, options.findIndex((o) => o.value === value)));
  const current = $derived(options.find((o) => o.value === value) ?? options[0]);

  async function show() {
    if (disabled || !trigger || !options.length) return;
    const r = trigger.getBoundingClientRect();
    const below = window.innerHeight - r.bottom - 10;
    const above = r.top - 10;
    const up = below < 200 && above > below;
    pos = {
      left: r.left,
      top: r.bottom + 6,
      bottom: window.innerHeight - r.top + 6,
      up,
      minWidth: r.width,
      maxHeight: Math.max(120, Math.min(340, (up ? above : below) - 6)),
    };
    active = index;
    open = true;
    openedAt = performance.now();
    try {
      menu?.showPopover?.();
    } catch {
      /* старый браузер без popover: панель и так показана по классу */
    }
    await Promise.resolve();
    // Не выходим за правый край экрана.
    if (menu) {
      const w = menu.getBoundingClientRect().width;
      pos.left = Math.max(8, Math.min(r.left, window.innerWidth - w - 8));
    }
    reveal();
  }

  function hide() {
    if (!open) return;
    open = false;
    closedAt = performance.now();
    try {
      menu?.hidePopover?.();
    } catch {
      /* уже закрыта */
    }
  }

  function pick(i: number) {
    const o = options[i];
    hide();
    if (!o) return;
    if (o.value !== value) {
      value = o.value;
      onchange?.(o.value);
    }
  }

  function reveal() {
    menu?.querySelector<HTMLElement>(`[data-i="${active}"]`)?.scrollIntoView({ block: 'nearest' });
  }

  function toggle(e: MouseEvent) {
    // Space/Enter при открытом списке выбирают вариант и закрывают его; браузер потом присылает «щелчок».
    if (e.detail === 0 && performance.now() - closedAt < 300) return;
    open ? hide() : show();
  }

  function triggerKey(e: KeyboardEvent) {
    if (open || disabled) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      show();
    }
  }

  // Пока список открыт, слушаем окно: в Safari кнопка при щелчке не получает фокус, и клавиши до неё не дошли бы.
  $effect(() => {
    if (!open) return;
    const key = (e: KeyboardEvent) => {
      const last = options.length - 1;
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        hide();
        trigger?.focus({ preventScroll: true });
      } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Home' || e.key === 'End') {
        e.preventDefault();
        active = e.key === 'Home' ? 0 : e.key === 'End' ? last : Math.min(last, Math.max(0, active + (e.key === 'ArrowDown' ? 1 : -1)));
        reveal();
      } else if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        pick(active);
      } else if (e.key === 'Tab') {
        hide();
      }
    };
    const outside = (e: PointerEvent) => {
      const t = e.target as Node;
      // Щелчок по подписи рядом (<label>) сам переключит список - закрывать его заранее не нужно.
      if (!menu?.contains(t) && !trigger?.closest('label')?.contains(t) && !trigger?.contains(t)) hide();
    };
    const scroll = (e: Event) => {
      // Страница могла только что доскроллиться к кнопке (само открытие) - это не повод закрывать.
      if (performance.now() - openedAt < 250) return;
      if (!menu?.contains(e.target as Node)) hide();
    };
    window.addEventListener('keydown', key, true);
    window.addEventListener('pointerdown', outside, true);
    window.addEventListener('scroll', scroll, true);
    window.addEventListener('resize', hide);
    return () => {
      window.removeEventListener('keydown', key, true);
      window.removeEventListener('pointerdown', outside, true);
      window.removeEventListener('scroll', scroll, true);
      window.removeEventListener('resize', hide);
    };
  });
</script>

<span class="select {variant} {klass}" class:fit class:open>
  <button
    type="button"
    class="sel"
    bind:this={trigger}
    role="combobox"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={uid}
    aria-label={label || undefined}
    {disabled}
    onclick={toggle}
    onkeydown={triggerKey}
  >
    <span class="t">{current?.label ?? ''}</span>
    <span class="chev"><Icon name="down" size={16} /></span>
  </button>
  <div
    id={uid}
    class="menu"
    class:shown={open}
    class:up={pos.up}
    popover="manual"
    role="listbox"
    aria-label={label || undefined}
    bind:this={menu}
    style:left="{pos.left}px"
    style:top={pos.up ? 'auto' : `${pos.top}px`}
    style:bottom={pos.up ? `${pos.bottom}px` : 'auto'}
    style:min-width="{pos.minWidth}px"
    style:max-height="{pos.maxHeight}px"
  >
    {#each options as o, i (o.value)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="opt"
        class:on={o.value === value}
        class:hot={i === active}
        role="option"
        aria-selected={o.value === value}
        data-i={i}
        tabindex="-1"
        onmousedown={(e) => e.preventDefault()}
        onpointermove={() => (active = i)}
        onclick={(e) => {
          // Список может лежать внутри <label>: без этого щелчок по варианту открыл бы его снова.
          e.preventDefault();
          e.stopPropagation();
          pick(i);
        }}
      >
        <span>{o.label}</span>
        {#if o.value === value}<Icon name="check" size={16} />{/if}
      </div>
    {/each}
  </div>
</span>

<style>
  .select {
    position: relative;
    display: block;
    width: 100%;
    min-width: 0;
    max-width: 100%;
  }
  .select.fit,
  .select.inline {
    display: inline-block;
    width: auto;
  }
  .sel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    min-width: 0;
    cursor: pointer;
    text-align: left;
    font: inherit;
    font-weight: 500;
    color: var(--text);
    outline: none;
    transition:
      border-color 0.15s,
      background 0.15s;
  }
  .sel:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .box .sel,
  .sm .sel {
    height: 44px;
    padding: 0 12px 0 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--line);
    background: rgba(0, 0, 0, 0.25);
  }
  .sm .sel {
    height: 34px;
    font-size: 13px;
  }
  .inline .sel {
    height: 30px;
    padding: 0 8px 0 10px;
    border-radius: 10px;
    border: 0;
    background: var(--surface-3);
    font-size: inherit;
  }
  .sel:hover:not(:disabled) {
    border-color: rgba(255, 255, 255, 0.28);
  }
  .inline .sel:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.12);
  }
  .open .sel,
  .sel:focus-visible {
    border-color: var(--accent-2);
  }
  .sel:focus-visible {
    outline: 2px solid #fff;
    outline-offset: 2px;
  }
  .t {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chev {
    display: grid;
    flex: none;
    color: var(--muted);
    transition: transform 0.2s var(--ease);
  }
  .open .chev {
    transform: rotate(180deg);
  }

  /* Панель - в верхнем слое (popover), поэтому позиция считается от окна, а не от родителя. */
  .menu {
    display: none;
    position: fixed;
    inset: auto;
    margin: 0;
    padding: 6px;
    width: max-content;
    max-width: min(440px, calc(100vw - 16px));
    box-sizing: border-box;
    overflow-y: auto;
    overscroll-behavior: contain;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 16px;
    background: #1c1c1f;
    color: var(--text);
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.6);
    font-size: 14px;
    z-index: 1000;
    transform-origin: top left;
  }
  .menu.up {
    transform-origin: bottom left;
  }
  .menu.shown {
    display: block;
    animation: menu-in 0.16s var(--ease);
  }
  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
  .menu.up.shown {
    animation-name: menu-in-up;
  }
  @keyframes menu-in-up {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.98);
    }
  }
  :global(.no-motion) .menu.shown {
    animation: none;
  }
  .opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 9px 10px;
    border-radius: 10px;
    cursor: pointer;
    line-height: 1.3;
  }
  .opt.hot {
    background: rgba(255, 255, 255, 0.09);
  }
  .opt.on {
    font-weight: 600;
  }
  .opt :global(svg) {
    flex: none;
    color: #fff;
  }
</style>
