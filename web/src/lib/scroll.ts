// Телефон и планшет (касание без мыши): страница не прокручивается сама, прокручивается `body` на весь экран
// (`app.css`, «оболочка»). Тогда нижняя панель разделов прибита к краю экрана и не ездит, когда Safari
// прячет и показывает свои панели, а резиновая прокрутка не открывает пустоту под ней.

const SHELL = '(hover: none) and (pointer: coarse)';

/** Работает ли оболочка: прокручивается `body`, а не окно. */
export function touchShell(): boolean {
  return typeof matchMedia !== 'undefined' && matchMedia(SHELL).matches;
}

/** Насколько прокручена страница (в оболочке - `body`). */
export function scrolledBy(): number {
  return touchShell() ? document.body.scrollTop : window.scrollY;
}

/** Прокрутить страницу (в оболочке - `body`). */
export function scrollToTop(top: number) {
  if (touchShell()) document.body.scrollTop = top;
  else window.scrollTo({ top, behavior: 'instant' as ScrollBehavior });
}

/**
 * Вернуться на прежнее место (25.11: «когда перехожу обратно в прошлые вкладки - не кидало наверх»). Страница
 * достраивается после перехода (герой-карусель, ряды, картинки), поэтому держим место до 2,5 с, пока высота страницы не
 * успокоится. Якорение прокрутки браузера на это время выключено: иначе, когда сверху дорастает герой, браузер сдвигает
 * страницу на его высоту (с главной на фильм и назад уносило на 1600 px ниже). Человек сам начал прокручивать - не мешаем.
 */
export function restoreScroll(top: number) {
  if (top <= 0) return;
  const roots = [document.documentElement, document.body];
  const anchors = roots.map((el) => el.style.overflowAnchor);
  for (const el of roots) el.style.overflowAnchor = 'none';
  let stop = false;
  const quit = () => (stop = true);
  addEventListener('wheel', quit, { once: true, passive: true });
  addEventListener('touchstart', quit, { once: true, passive: true });
  addEventListener('keydown', quit, { once: true });
  const end = performance.now() + 2500;
  let height = 0;
  let calm = 0;
  const done = () => {
    roots.forEach((el, i) => (el.style.overflowAnchor = anchors[i]));
    removeEventListener('wheel', quit);
    removeEventListener('touchstart', quit);
    removeEventListener('keydown', quit);
  };
  const step = () => {
    if (stop) return done();
    if (Math.abs(scrolledBy() - top) > 2) scrollToTop(top);
    const h = document.documentElement.scrollHeight;
    calm = h === height && Math.abs(scrolledBy() - top) <= 2 ? calm + 1 : 0;
    height = h;
    // Пятнадцать кадров подряд та же высота и то же место - страница достроилась.
    if (calm < 15 && performance.now() < end) requestAnimationFrame(step);
    else done();
  };
  step();
}

/** Запрет масштабирования щипком и двойным касанием: интерфейс всегда в своём размере. */
export function lockZoom(): () => void {
  if (!touchShell()) return () => {};
  const stop = (e: Event) => e.preventDefault();
  const pinch = (e: TouchEvent) => e.touches.length > 1 && e.preventDefault();
  // Жесты Safari (iOS) и многопальцевое касание (остальные).
  document.addEventListener('gesturestart', stop);
  document.addEventListener('gesturechange', stop);
  document.addEventListener('gestureend', stop);
  document.addEventListener('touchmove', pinch, { passive: false });
  return () => {
    document.removeEventListener('gesturestart', stop);
    document.removeEventListener('gesturechange', stop);
    document.removeEventListener('gestureend', stop);
    document.removeEventListener('touchmove', pinch);
  };
}
