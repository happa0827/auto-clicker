import type { Interval, TimeUnit } from "./types";

export type IntervalError = "empty" | "not_a_number" | "too_short";

export type IntervalResult = { ok: true; value: Interval } | { ok: false; error: IntervalError };

const MILLIS_PER_UNIT: Record<TimeUnit, number> = {
  milliseconds: 1,
  seconds: 1000,
  minutes: 60_000,
};

const PLAIN_NUMBER = /^(\d+\.?\d*|\.\d+)$/;

/** 入力欄の文字列を Interval にする。最小は Rust 側と同じ 1 ms。 */
export function parseInterval(text: string, unit: TimeUnit): IntervalResult {
  const normalized = text.normalize("NFKC").trim();
  if (normalized === "") return { ok: false, error: "empty" };
  if (!PLAIN_NUMBER.test(normalized)) return { ok: false, error: "not_a_number" };
  const value = Number(normalized);
  if (value * MILLIS_PER_UNIT[unit] < 1) return { ok: false, error: "too_short" };
  return { ok: true, value: { value, unit } };
}
