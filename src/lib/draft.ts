import { parseInterval, type IntervalError } from "./interval";
import type { Button, ClickTarget, Interval, Sequence, Step, TimeUnit } from "./types";

export type Mode = "current" | "fixed" | "sequence";

export interface TargetDraft {
  /** シーケンスの行でだけ使う。current / fixed モードではモードが位置を決める。 */
  position: "current" | "fixed";
  button: Button;
  kind: "single" | "double" | "hold";
  holdText: string;
  holdUnit: TimeUnit;
  x: string;
  y: string;
}

export interface IntervalDraft {
  text: string;
  unit: TimeUnit;
}

export interface StepDraft {
  target: TargetDraft;
  interval: IntervalDraft;
}

/** 画面のフォームの状態。入力欄は文字列のまま持つ。 */
export interface Draft {
  mode: Mode;
  target: TargetDraft;
  interval: IntervalDraft;
  /** シーケンスモードの行。 */
  steps: StepDraft[];
}

export type FieldError = IntervalError | "not_an_integer";

export interface RowErrors {
  interval?: FieldError;
  hold?: FieldError;
  x?: FieldError;
  y?: FieldError;
}

export interface DraftErrors extends RowErrors {
  steps?: "empty";
  /** シーケンスモードで、エラーのある行の番号（0 始まり）ごとのエラー。 */
  stepErrors?: Record<number, RowErrors>;
}

export type BuildResult = { ok: true; sequence: Sequence } | { ok: false; errors: DraftErrors };

export function defaultDraft(): Draft {
  return {
    mode: "current",
    target: {
      position: "fixed",
      button: "left",
      kind: "single",
      holdText: "500",
      holdUnit: "milliseconds",
      x: "0",
      y: "0",
    },
    interval: { text: "200", unit: "milliseconds" },
    steps: [],
  };
}

/** キャプチャした座標から、シーケンスの新しい行を作る。 */
export function newStepDraft(x: number, y: number): StepDraft {
  const base = defaultDraft();
  return { target: { ...base.target, position: "fixed", x: String(x), y: String(y) }, interval: base.interval };
}

const INT32_MIN = -2_147_483_648;
const INT32_MAX = 2_147_483_647;

function parseCoordinate(text: string): { ok: true; value: number } | { ok: false; error: FieldError } {
  const normalized = text.normalize("NFKC").trim();
  if (normalized === "") return { ok: false, error: "empty" };
  if (!/^-?\d+$/.test(normalized)) return { ok: false, error: "not_an_integer" };
  const value = Number(normalized);
  if (value < INT32_MIN || value > INT32_MAX) return { ok: false, error: "not_an_integer" };
  return { ok: true, value };
}

function buildStep(
  target: TargetDraft,
  intervalDraft: IntervalDraft,
  position: "current" | "fixed",
): { ok: true; step: Step } | { ok: false; errors: RowErrors } {
  const errors: RowErrors = {};

  const interval = parseInterval(intervalDraft.text, intervalDraft.unit);
  if (!interval.ok) errors.interval = interval.error;

  let where: ClickTarget["position"] = { type: "current" };
  if (position === "fixed") {
    const x = parseCoordinate(target.x);
    const y = parseCoordinate(target.y);
    if (!x.ok) errors.x = x.error;
    if (!y.ok) errors.y = y.error;
    if (x.ok && y.ok) where = { type: "fixed", x: x.value, y: y.value };
  }

  let kind: ClickTarget["kind"] = { type: "single" };
  if (target.kind === "double") kind = { type: "double" };
  if (target.kind === "hold") {
    const hold = parseInterval(target.holdText, target.holdUnit);
    if (hold.ok) kind = { type: "hold", duration: hold.value };
    else errors.hold = hold.error;
  }

  if (!interval.ok || Object.keys(errors).length > 0) return { ok: false, errors };
  return { ok: true, step: { target: { position: where, button: target.button, kind }, interval: interval.value } };
}

/** フォームの下書きを、Rust が受け取る Sequence にする。エラーはフィールドごとに返す。 */
export function buildSequence(draft: Draft): BuildResult {
  if (draft.mode === "sequence") {
    if (draft.steps.length === 0) return { ok: false, errors: { steps: "empty" } };
    const steps: Step[] = [];
    const stepErrors: Record<number, RowErrors> = {};
    draft.steps.forEach((row, index) => {
      const built = buildStep(row.target, row.interval, row.target.position);
      if (built.ok) steps.push(built.step);
      else stepErrors[index] = built.errors;
    });
    if (Object.keys(stepErrors).length > 0) return { ok: false, errors: { stepErrors } };
    return { ok: true, sequence: { steps } };
  }

  const built = buildStep(draft.target, draft.interval, draft.mode);
  if (!built.ok) return { ok: false, errors: built.errors };
  return { ok: true, sequence: { steps: [built.step] } };
}

function textOf(interval: Interval): IntervalDraft {
  return { text: String(interval.value), unit: interval.unit };
}

function stepToDraft(step: Step): StepDraft {
  const base = defaultDraft().target;
  const { position, button, kind } = step.target;
  const target: TargetDraft = {
    ...base,
    position: position.type,
    button,
    kind: kind.type,
    ...(position.type === "fixed" ? { x: String(position.x), y: String(position.y) } : {}),
    ...(kind.type === "hold" ? { holdText: String(kind.duration.value), holdUnit: kind.duration.unit } : {}),
  };
  return { target, interval: textOf(step.interval) };
}

/** 保存済みの Sequence を、フォームの下書きに戻す。 */
export function draftFromSequence(sequence: Sequence): Draft {
  const base = defaultDraft();
  const rows = sequence.steps.map(stepToDraft);
  if (rows.length === 1) {
    const [row] = rows;
    return { ...base, mode: row.target.position === "current" ? "current" : "fixed", target: row.target, interval: row.interval };
  }
  return { ...base, mode: "sequence", steps: rows };
}

/** F7 でキャプチャした座標を下書きに反映する。現在位置モードでは座標を使わないので何もしない。 */
export function applyCapture(draft: Draft, x: number, y: number): Draft {
  switch (draft.mode) {
    case "sequence":
      return { ...draft, steps: [...draft.steps, newStepDraft(x, y)] };
    case "fixed":
      return { ...draft, target: { ...draft.target, x: String(x), y: String(y) } };
    case "current":
      return draft;
  }
}
