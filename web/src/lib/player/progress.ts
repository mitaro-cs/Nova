// Место просмотра: сохраняется раз в 5 секунд, при паузе и уходе со страницы (keepalive доживает до
// закрытия вкладки); «смотрели на другом устройстве» - предложить перейти туда, где остановились.

export interface ProgressOpts {
  /** Куда писать место (файл хоста или видео по ссылке); null - не сохраняем. */
  url: () => string | null;
  position: () => number;
  duration: () => number;
  /** Как идёт просмотр (скорость, качество, остановки) - хосту в «Управление». */
  net?: () => unknown;
  /** Перед отправкой (метка «это своё сохранение» для синхронизации между устройствами). */
  onSave?: () => void;
  /** Сервер ответил: досмотрено (`done`), фильм только что стал «Просмотрено» (`watched`). */
  onResult?: (r: { done: boolean; watched: boolean }) => void;
}

const MIN_GAP_MS = 5000;

export function createProgress(o: ProgressOpts) {
  let lastSaved = 0;
  let chain: Promise<unknown> = Promise.resolve();

  function save(force = false): Promise<unknown> {
    const time = o.position();
    const duration = o.duration();
    if (!duration || time < 1) return chain;
    const url = o.url();
    if (!url) return chain;
    const now = Date.now();
    if (!force && now - lastSaved < MIN_GAP_MS) return chain;
    lastSaved = now;
    o.onSave?.();
    const body = JSON.stringify({ position: time, duration, net: o.net?.() });
    chain = chain
      .catch(() => {})
      .then(() =>
        fetch(url, {
          method: 'PUT',
          keepalive: true,
          credentials: 'same-origin',
          headers: { 'content-type': 'application/json', 'x-fsfy': '1' },
          body,
        }),
      )
      .then(async (r) => {
        if (!o.onResult || !(r instanceof Response) || !r.ok) return;
        const j = (await r.json().catch(() => null)) as { done?: boolean; watched?: boolean } | null;
        if (j) o.onResult({ done: !!j.done, watched: !!j.watched });
      })
      .catch(() => {});
    return chain;
  }

  return {
    save,
    /** Последнее сохранение при закрытии плеера. */
    stop: () => save(true),
  };
}

/** Место на другом устройстве, если оно дальше чем на 20 секунд и не у самого конца; иначе null. */
export function elsewherePosition(remote: number | null, time: number, duration: number): number | null {
  if (remote === null || !Number.isFinite(remote)) return null;
  return Math.abs(remote - time) > 20 && remote < duration - 30 ? remote : null;
}
