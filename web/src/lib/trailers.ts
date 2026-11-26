/** Трейлеры сами не играют: экономия трафика у браузера, зритель выключил их в профиле или это телефон/планшет и
 *  зритель их явно не включал (26.11 «на телефоне всё лагает»: видео за каруселью грузило телефон и связь). */
export function trailersOff(): boolean {
  try {
    const saveData = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection?.saveData === true;
    const v = localStorage.getItem('nova-trailers');
    const touch = matchMedia('(hover: none) and (pointer: coarse)').matches;
    return saveData || v === 'off' || (touch && v !== 'on');
  } catch {
    return false;
  }
}
