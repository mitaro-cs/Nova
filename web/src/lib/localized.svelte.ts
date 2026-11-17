
/** Не показываем английский сюжет до готовности русского. Оригинальное название остаётся отдельно. */
export function localized(text: string | null | undefined, fallback = ''): string {
  if (!text) return fallback;
  if (/[а-яё]/i.test(text) || !/[a-z]/i.test(text)) return text;
  return fallback;
}
