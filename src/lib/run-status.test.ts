import { describe, expect, it } from "vitest";
import { describeNotice, initialRunStatus, runStatusReducer, type Action, type RunStatus } from "./run-status";

const apply = (actions: Action[], from: RunStatus = initialRunStatus): RunStatus =>
  actions.reduce(runStatusReducer, from);

const running: Action = { type: "run-event", event: { type: "state", data: "running" } };
const idle: Action = { type: "run-event", event: { type: "state", data: "idle" } };
const round = (n: number): Action => ({ type: "run-event", event: { type: "round", data: n } });
const finished = (data: object): Action => ({ type: "run-event", event: { type: "finished", data } as never });

describe("runStatusReducer: progress", () => {
  it("starts idle with no rounds", () => {
    expect(initialRunStatus).toMatchObject({ status: "idle", rounds: 0, notice: null });
  });

  it("follows running, rounds and idle", () => {
    expect(apply([running]).status).toBe("running");
    expect(apply([running, round(1), round(2)]).rounds).toBe(2);
    expect(apply([running, round(2), idle]).status).toBe("idle");
  });

  it("keeps the last round count after the run ends", () => {
    expect(apply([running, round(3), idle]).rounds).toBe(3);
  });

  it("resets rounds and clears the previous notice when a new run starts", () => {
    const ended = apply([running, round(4), idle, finished({ type: "completed", rounds: 4 })]);
    expect(ended.notice).not.toBeNull();
    const again = apply([running], ended);
    expect(again.rounds).toBe(0);
    expect(again.notice).toBeNull();
  });
});

describe("runStatusReducer: why a run ended", () => {
  it("reports completion", () => {
    const s = apply([running, round(5), idle, finished({ type: "completed", rounds: 5 })]);
    expect(s.notice).toEqual({ kind: "info", code: "completed", rounds: 5 });
  });

  it("reports a manual stop", () => {
    const s = apply([running, round(2), idle, finished({ type: "cancelled", rounds: 2 })]);
    expect(s.notice).toEqual({ kind: "info", code: "stopped", rounds: 2 });
  });

  it("reports a screen layout change as the reason, not a plain stop", () => {
    const s = apply([
      running,
      round(1),
      { type: "run-event", event: { type: "screen_layout_changed" } },
      idle,
      finished({ type: "cancelled", rounds: 1 }),
    ]);
    expect(s.notice).toEqual({ kind: "warning", code: "screen_changed", rounds: 1 });
  });

  it("does not blame the screen on a later manual stop", () => {
    const first = apply([
      running,
      { type: "run-event", event: { type: "screen_layout_changed" } },
      idle,
      finished({ type: "cancelled", rounds: 0 }),
    ]);
    const second = apply([running, idle, finished({ type: "cancelled", rounds: 0 })], first);
    expect(second.notice).toEqual({ kind: "info", code: "stopped", rounds: 0 });
  });

  it("reports an input failure with its message", () => {
    const s = apply([running, idle, finished({ type: "failed", rounds: 0, error: "boom" })]);
    expect(s.notice).toEqual({ kind: "error", code: "input_failed", rounds: 0, message: "boom" });
  });
});

describe("runStatusReducer: refused starts and environment", () => {
  it("reports why a start was refused", () => {
    expect(apply([{ type: "start-error", error: { type: "hotkeys_unavailable" } }]).notice).toEqual({
      kind: "error",
      code: "hotkeys_unavailable",
    });
    expect(apply([{ type: "start-error", error: { type: "already_running" } }]).notice).toEqual({
      kind: "info",
      code: "already_running",
    });
    const off = apply([
      {
        type: "start-error",
        error: { type: "off_screen", data: [{ type: "off_screen", step_index: 2, x: 9, y: 9 }] },
      },
    ]);
    expect(off.notice).toEqual({ kind: "error", code: "off_screen", stepNumbers: [3] });
  });

  it("tracks whether both hotkeys are registered", () => {
    expect(apply([{ type: "hotkeys", toggle: "F6", capture: "F7" }]).hotkeys).toEqual({
      known: true,
      ready: true,
      toggle: "F6",
      capture: "F7",
    });
    expect(apply([{ type: "hotkeys", toggle: null, capture: "F7" }]).hotkeys.ready).toBe(false);
  });

  it("does not claim the hotkeys failed before their status is known", () => {
    expect(initialRunStatus.hotkeys.known).toBe(false);
    expect(apply([{ type: "hotkeys", toggle: null, capture: null }]).hotkeys.known).toBe(true);
  });

  it("keeps environment warnings and lets the notice be dismissed", () => {
    const s = apply([{ type: "warnings", warnings: ["WaylandUnsupported"] }]);
    expect(s.environmentWarnings).toEqual(["WaylandUnsupported"]);
    const refused = apply([{ type: "start-error", error: { type: "hotkeys_unavailable" } }]);
    expect(apply([{ type: "dismiss-notice" }], refused).notice).toBeNull();
  });
});

describe("describeNotice", () => {
  it("writes a plain Japanese sentence for each notice", () => {
    expect(describeNotice({ kind: "info", code: "completed", rounds: 5 })).toBe("5 回で完了しました。");
    expect(describeNotice({ kind: "info", code: "stopped", rounds: 2 })).toBe("停止しました。2 回実行しました。");
    expect(describeNotice({ kind: "warning", code: "screen_changed", rounds: 1 })).toBe(
      "画面の構成が変わったため停止しました。1 回実行しました。",
    );
    expect(describeNotice({ kind: "error", code: "input_failed", rounds: 0, message: "boom" })).toBe(
      "クリックに失敗したため停止しました: boom",
    );
    expect(describeNotice({ kind: "error", code: "hotkeys_unavailable" })).toBe(
      "ホットキーを登録できていません。設定でキーを変更してください。",
    );
    expect(describeNotice({ kind: "error", code: "off_screen", stepNumbers: [1, 3] })).toBe(
      "画面の外にある Step があります: 1, 3 番目",
    );
    expect(describeNotice({ kind: "info", code: "already_running" })).toBe("すでに実行中です。");
  });

  it("never uses an em dash", () => {
    const all = [
      describeNotice({ kind: "info", code: "stopped", rounds: 1 }),
      describeNotice({ kind: "warning", code: "screen_changed", rounds: 1 }),
    ].join("");
    expect(all).not.toContain("—");
  });
});
