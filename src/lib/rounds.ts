export type RoundsError = "empty" | "not_an_integer" | "too_small" | "too_large";

export type RoundsResult = { ok: true; count: number } | { ok: false; error: RoundsError };

const U32_MAX = 4_294_967_295;

/** 「N 回で終了」の N。Rust 側は 1 以上の u32。 */
export function parseRounds(text: string): RoundsResult {
  const normalized = text.normalize("NFKC").trim();
  if (normalized === "") return { ok: false, error: "empty" };
  if (!/^\d+$/.test(normalized)) return { ok: false, error: "not_an_integer" };
  const count = Number(normalized);
  if (count < 1) return { ok: false, error: "too_small" };
  if (count > U32_MAX) return { ok: false, error: "too_large" };
  return { ok: true, count };
}
