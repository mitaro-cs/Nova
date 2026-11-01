/** Трейлеры сами не играют: включена экономия трафика у браузера или зритель выключил их в профиле. */
export function trailersOff(): boolean {
  try {
    const saveData = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection?.saveData === true;
    return saveData || localStorage.getItem('nova-trailers') === 'off';
  } catch {
    return false;
  }
}
