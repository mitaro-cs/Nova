/** В плеере показываем название, а не подпись источника с годом, качеством и оригиналом. */
export function playbackTitle(value:string):string {
  return value.split(/\s*[|｜]\s*/)[0].replace(/\s*\((?:19|20)\d{2}(?:\s*[,·].*?)?\)\s*$/u,'').trim();
}
