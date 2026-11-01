// Rust (src-tauri/src/controller.rs) が送る JSON の形と対応させる。

export type RunOutcome =
  | { type: "completed"; rounds: number }
  | { type: "cancelled"; rounds: number }
  | { type: "failed"; rounds: number; error: string };

export type RunEvent =
  | { type: "state"; data: "idle" | "running" }
  | { type: "round"; data: number }
  | { type: "screen_layout_changed" }
  | { type: "finished"; data: RunOutcome };

export type StartError =
  | { type: "already_running" }
  | { type: "hotkeys_unavailable" }
  | { type: "off_screen"; data: { type: "off_screen"; step_index: number; x: number; y: number }[] };

export type Action =
  | { type: "run-event"; event: RunEvent }
  | { type: "start-error"; error: StartError }
  | { type: "hotkeys"; toggle: string | null; capture: string | null }
  | { type: "warnings"; warnings: string[] }
  | { type: "dismiss-notice" };

export type Notice =
  | { kind: "info"; code: "completed" | "stopped"; rounds: number }
  | { kind: "warning"; code: "screen_changed"; rounds: number }
  | { kind: "error"; code: "input_failed"; rounds: number; message: string }
  | { kind: "error"; code: "hotkeys_unavailable" }
  | { kind: "error"; code: "off_screen"; stepNumbers: number[] }
  | { kind: "info"; code: "already_running" };

export interface RunStatus {
  status: "idle" | "running";
  rounds: number;
  notice: Notice | null;
  /** known: 起動時の登録結果を受け取り済みか。未取得の間は失敗とみなさない。 */
  hotkeys: { known: boolean; ready: boolean; toggle: string | null; capture: string | null };
  environmentWarnings: string[];
  /** 画面構成の変更を検出した直後。次の停止の理由として使う。 */
  screenChanged: boolean;
}

export const initialRunStatus: RunStatus = {
  status: "idle",
  rounds: 0,
  notice: null,
  hotkeys: { known: false, ready: false, toggle: null, capture: null },
  environmentWarnings: [],
  screenChanged: false,
};

function onFinished(state: RunStatus, outcome: RunOutcome): RunStatus {
  const base = { ...state, screenChanged: false };
  switch (outcome.type) {
    case "completed":
      return { ...base, notice: { kind: "info", code: "completed", rounds: outcome.rounds } };
    case "cancelled":
      return {
        ...base,
        notice: state.screenChanged
          ? { kind: "warning", code: "screen_changed", rounds: outcome.rounds }
          : { kind: "info", code: "stopped", rounds: outcome.rounds },
      };
    case "failed":
      return {
        ...base,
        notice: { kind: "error", code: "input_failed", rounds: outcome.rounds, message: outcome.error },
      };
  }
}

function onStartError(state: RunStatus, error: StartError): RunStatus {
  switch (error.type) {
    case "already_running":
      return { ...state, notice: { kind: "info", code: "already_running" } };
    case "hotkeys_unavailable":
      return { ...state, notice: { kind: "error", code: "hotkeys_unavailable" } };
    case "off_screen":
      return {
        ...state,
        notice: { kind: "error", code: "off_screen", stepNumbers: error.data.map((p) => p.step_index + 1) },
      };
  }
}

function onRunEvent(state: RunStatus, event: RunEvent): RunStatus {
  switch (event.type) {
    case "state":
      return event.data === "running"
        ? { ...state, status: "running", rounds: 0, notice: null, screenChanged: false }
        : { ...state, status: "idle" };
    case "round":
      return { ...state, rounds: event.data };
    case "screen_layout_changed":
      return { ...state, screenChanged: true };
    case "finished":
      return onFinished(state, event.data);
  }
}

export function runStatusReducer(state: RunStatus, action: Action): RunStatus {
  switch (action.type) {
    case "run-event":
      return onRunEvent(state, action.event);
    case "start-error":
      return onStartError(state, action.error);
    case "hotkeys":
      return {
        ...state,
        hotkeys: {
          known: true,
          ready: action.toggle !== null && action.capture !== null,
          toggle: action.toggle,
          capture: action.capture,
        },
      };
    case "warnings":
      return { ...state, environmentWarnings: action.warnings };
    case "dismiss-notice":
      return { ...state, notice: null };
  }
}

export function describeNotice(notice: Notice): string {
  switch (notice.code) {
    case "completed":
      return `${notice.rounds} 回で完了しました。`;
    case "stopped":
      return `停止しました。${notice.rounds} 回実行しました。`;
    case "screen_changed":
      return `画面の構成が変わったため停止しました。${notice.rounds} 回実行しました。`;
    case "input_failed":
      return `クリックに失敗したため停止しました: ${notice.message}`;
    case "hotkeys_unavailable":
      return "ホットキーを登録できていません。設定でキーを変更してください。";
    case "off_screen":
      return `画面の外にある Step があります: ${notice.stepNumbers.join(", ")} 番目`;
    case "already_running":
      return "すでに実行中です。";
  }
}
