import { goto } from '$app/navigation';
import type { ExtCard } from '$lib/types';

// Фильм из ленты IMDb открывается своей страницей `/ext/<id>`. Карточку, по которой нажали,
// помним: страница сразу показывает постер и название, остальное приходит следом.
// `version` - поменялся сам каталог (импорт, источники): ленты перечитываются. `marks` - человек поставил отметку:
// карточка уже обновлена на месте, ленты не перечитываем (иначе ряды перестраивались и менялись местами).
export const extSheet = $state<{ seed: ExtCard | null; version: number; marks: number }>({
  seed: null,
  version: 0,
  marks: 0,
});

export function remember(card: ExtCard) {
  extSheet.seed = card;
}

export function openExt(card: ExtCard) {
  remember(card);
  goto(`/ext/${card.id}`);
}

/** Поменялся каталог - страницы с лентами перечитают их. */
export function extChanged() {
  extSheet.version++;
}

/** Поставлена или снята отметка: перечитывают только списки отметок («Коллекция») и подробности фильма. */
export function marksChanged() {
  extSheet.marks++;
}
