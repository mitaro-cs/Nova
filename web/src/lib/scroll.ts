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
