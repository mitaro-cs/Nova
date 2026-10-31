// Ошибки интерфейса уходят в журнал приложения (`POST /api/diag/client`): так «вылетает» можно
// потом увидеть в «Управление → Обзор». Не больше 5 разных сообщений за страницу, без повторов,
// без адресов страницы (только путь).
export type DiagKind = 'error' | 'rejection' | 'player';
type Body = { kind: string; message: string; where: string };

const MAX_MESSAGE = 500;
const MAX_PER_PAGE = 5;

export function createReporter(send: (b: Body) => void, where: () => string) {
  const seen = new Set<string>();
  return (kind: DiagKind, message: string) => {
    const text = message.slice(0, MAX_MESSAGE);
    const key = `${kind}:${text}`;
    if (!text || seen.has(key) || seen.size >= MAX_PER_PAGE) return;
    seen.add(key);
    try {
      send({ kind, message: text, where: where() });
    } catch {
      /* журнал не должен ломать страницу */
    }
  };
}

function post(b: Body) {
  void fetch('/api/diag/client', {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-fsfy': '1' },
    body: JSON.stringify(b),
    keepalive: true,
    credentials: 'same-origin',
  }).catch(() => {});
}

let reporter: ReturnType<typeof createReporter> | null = null;

/** Отправить сообщение в журнал (плеер и другие модули). */
export function report(kind: DiagKind, message: string) {
  reporter ??= createReporter(post, () => location.pathname);
  reporter(kind, message);
}

let installed = false;

/** Ловить необработанные ошибки и отклонённые обещания страницы. */
export function installDiag() {
  if (installed || typeof window === 'undefined') return;
  installed = true;
  window.addEventListener('error', (e) => {
    const message = e.message || 'ошибка';
    if (message.includes('ResizeObserver')) return;
    report('error', message);
  });
  window.addEventListener('unhandledrejection', (e) => {
    const r = e.reason;
    report('rejection', String(r?.message ?? r ?? 'отклонено'));
  });
}
