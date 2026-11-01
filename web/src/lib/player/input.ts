// Клавиатура плеера: клавиша -> действие (чистая функция). Что делать с действием, решает оболочка.
// Esc в `+layout` всегда `preventDefault` (иначе окно на Mac выходит из полноэкранного режима) -
// здесь он лишь «меню закрыть, иначе назад к фильму».

export type Action =
  | { type: 'toggle' }
  | { type: 'seek'; delta: number }
  | { type: 'volume'; delta: number }
  | { type: 'fullscreen' }
  | { type: 'mute' }
  | { type: 'skipIntro' }
  | { type: 'next' }
  | { type: 'captions' }
  | { type: 'escape' }
  | { type: 'rate'; delta: number }
  | { type: 'pip' }
  | { type: 'keys' }
  | { type: 'percent'; tenth: number }
  /** Удержание одноразовой клавиши: ничего не делать, но событие погасить. */
  | { type: 'swallow' };

export interface KeyInput {
  code: string;
  key: string;
  shiftKey: boolean;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  repeat: boolean;
}

export interface KeyState {
  pipOk: boolean;
  canNext: boolean;
  hasSubs: boolean;
  inIntro: boolean;
  duration: number;
}

/** Клавиши, которые при удержании срабатывают один раз. */
const ONCE = ['Space', 'KeyK', 'KeyF', 'KeyM', 'KeyN', 'KeyC', 'KeyP'];

export function keyAction(e: KeyInput, s: KeyState): Action | null {
  if (e.metaKey || e.ctrlKey || e.altKey) return null;
  const k = e.code;
  if (e.repeat && ONCE.includes(k)) return { type: 'swallow' };
  if (k === 'Space' || k === 'KeyK') return { type: 'toggle' };
  if (k === 'ArrowRight' || k === 'KeyL') return { type: 'seek', delta: 10 };
  if (k === 'ArrowLeft' || k === 'KeyJ') return { type: 'seek', delta: -10 };
  if (k === 'ArrowUp') return { type: 'volume', delta: 0.1 };
  if (k === 'ArrowDown') return { type: 'volume', delta: -0.1 };
  if (k === 'KeyF') return { type: 'fullscreen' };
  if (k === 'KeyM') return { type: 'mute' };
  if (k === 'KeyS' && s.inIntro) return { type: 'skipIntro' };
  if (k === 'KeyN' && s.canNext) return { type: 'next' };
  if (k === 'KeyC' && s.hasSubs) return { type: 'captions' };
  if (k === 'Escape') return { type: 'escape' };
  if (k === 'Period' && e.shiftKey) return { type: 'rate', delta: 0.25 };
  if (k === 'Comma' && e.shiftKey) return { type: 'rate', delta: -0.25 };
  if (k === 'KeyP' && s.pipOk) return { type: 'pip' };
  if (e.key === '?') return { type: 'keys' };
  if (/^Digit\d$/.test(k) && s.duration) return { type: 'percent', tenth: Number(k.slice(5)) };
  return null;
}
