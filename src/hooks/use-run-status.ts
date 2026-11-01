import { useEffect, useReducer } from "react";
import { api } from "@/lib/api";
import { initialRunStatus, runStatusReducer, type Action } from "@/lib/run-status";

/** Rust から届くイベントを購読して、画面に出す実行状態にまとめる。 */
export function useRunStatus(onCaptured: (point: { x: number; y: number }) => void) {
  const [status, dispatch] = useReducer(runStatusReducer, initialRunStatus);

  useEffect(() => {
    const unsubscribe = api.subscribe({
      onRunEvent: (event) => dispatch({ type: "run-event", event }),
      onStartError: (error) => dispatch({ type: "start-error", error }),
      onCaptured,
    });
    api.warnings().then((warnings) => dispatch({ type: "warnings", warnings }));
    api.hotkeyStatus().then(({ toggle, capture }) => dispatch({ type: "hotkeys", toggle, capture }));
    return unsubscribe;
  }, [onCaptured]);

  return [status, dispatch as (action: Action) => void] as const;
}
