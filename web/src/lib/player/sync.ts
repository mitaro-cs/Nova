// Совместный просмотр: свои нажатия - в комнату, команды комнаты - сюда. Правила сверки времени - те же,
// что у прежнего плеера.
import type { Mode } from './plan';

export interface PlayerSync {
  /** Зритель сам: пауза/смотреть, перемотка, следующая серия (`fileId`). */
  local(s: { playing: boolean; position: number; fileId?: number }): void;
  /** Команды комнаты; вернуть - отписка. */
  listen(apply: (s: { playing: boolean; position: number }) => void): () => void;
  /** «Назад» из комнаты. */
  back?: () => void;
}

/** Где сейчас комната: идёт - добавляем время, прошедшее с последнего сообщения. */
export function remotePosition(remote: { playing: boolean; position: number }, receivedAtMs: number, nowMs: number): number {
  return remote.playing ? remote.position + (nowMs - receivedAtMs) / 1000 : remote.position;
}

/** Большой разрыв - перемотка; у перепаковки это новый поток, поэтому порог больше. */
export function needsSeek(time: number, target: number, mode: Mode, seeking: boolean): boolean {
  return Math.abs(time - target) > (mode === 'remux' ? 6 : 2.5) && !seeking;
}
