export interface AudioChoice {
  lang?: string | null;
  default?: boolean;
}

/** Prefer the stream's declared default, then the browser language, Russian, and the first track. */
export function defaultAudioTrack(
  tracks: AudioChoice[],
  languages: string[] = typeof navigator === 'undefined' ? [] : [...navigator.languages],
): number {
  if (!tracks.length) return -1;
  const declared = tracks.findIndex((track) => track.default);
  if (declared >= 0) return declared;

  const normalize = (value?: string | null) => value?.trim().toLowerCase().split(/[-_]/, 1)[0] ?? '';
  for (const language of languages) {
    const code = normalize(language);
    if (!code) continue;
    const match = tracks.findIndex((track) => normalize(track.lang) === code);
    if (match >= 0) return match;
  }

  const russian = tracks.findIndex((track) => normalize(track.lang) === 'ru');
  return russian >= 0 ? russian : 0;
}
