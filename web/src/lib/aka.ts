/** «Также»: оригинальное название и названия с кириллицей (испанские, немецкие, турецкие - убираем). */
export function akaShown(aka: string[], original?: string | null): string[] {
  const out: string[] = [];
  const add = (s: string) => {
    if (s && !out.includes(s)) out.push(s);
  };
  if (original) add(original);
  for (const a of aka) if (/[а-яё]/i.test(a)) add(a);
  return out.slice(0, 4);
}
