import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Preset, Sequence, Settings, StopCondition } from "./types";
import type { RunEvent, StartError } from "./run-status";
import type { HotkeyError, Hotkeys } from "./hotkeys";

export type SetHotkeysResult = { ok: true; hotkeys: Hotkeys } | { ok: false; error: HotkeyError };

export interface Listeners {
  onRunEvent: (event: RunEvent) => void;
  onStartError: (error: StartError) => void;
  onCaptured: (point: { x: number; y: number }) => void;
}

export interface Api {
  getSettings(): Promise<Settings>;
  /** 停止条件だけを保存する。他の設定には触れない。 */
  setStop(stop: StopCondition): Promise<void>;
  setAutostart(enabled: boolean): Promise<{ ok: true } | { ok: false; error: string }>;
  setStartInTray(enabled: boolean): Promise<void>;
  getSequence(): Promise<Sequence>;
  saveSequence(sequence: Sequence): Promise<void>;
  listPresets(): Promise<Preset[]>;
  savePreset(preset: Preset): Promise<void>;
  deletePreset(name: string): Promise<void>;
  toggle(): Promise<void>;
  warnings(): Promise<string[]>;
  hotkeyStatus(): Promise<Hotkeys>;
  setHotkeys(toggle: string, capture: string): Promise<SetHotkeysResult>;
  subscribe(listeners: Listeners): () => void;
}

const tauriApi: Api = {
  getSettings: () => invoke("get_settings"),
  setStop: (stop) => invoke("set_stop", { stop }),
  async setAutostart(enabled) {
    try {
      await invoke("set_autostart", { enabled });
      return { ok: true };
    } catch (error) {
      return { ok: false, error: String(error) };
    }
  },
  setStartInTray: (enabled) => invoke("set_start_in_tray", { enabled }),
  getSequence: () => invoke("get_sequence"),
  saveSequence: (sequence) => invoke("save_sequence", { sequence }),
  listPresets: () => invoke("list_presets"),
  savePreset: (preset) => invoke("save_preset", { preset }),
  deletePreset: (name) => invoke("delete_preset", { name }),
  async toggle() {
    // Rust は拒否理由を StartError の JSON で reject する。画面には run-start-error イベントで届く。
    await invoke("toggle").catch(() => undefined);
  },
  warnings: () => invoke("warnings"),
  hotkeyStatus: () => invoke("hotkey_status"),
  async setHotkeys(toggle, capture) {
    try {
      return { ok: true, hotkeys: await invoke<Hotkeys>("set_hotkeys", { toggle, capture }) };
    } catch (error) {
      return { ok: false, error: error as HotkeyError };
    }
  },
  subscribe(listeners) {
    const pending = [
      listen<RunEvent>("run-event", (e) => listeners.onRunEvent(e.payload)),
      listen<StartError>("run-start-error", (e) => listeners.onStartError(e.payload)),
      listen<{ x: number; y: number }>("captured", (e) => listeners.onCaptured(e.payload)),
    ];
    return () => pending.forEach((p) => p.then((off) => off()));
  },
};

/** ブラウザだけで見た目を確認するためのモック。Tauri の外でしか使わない。 */
function createMockApi(): Api {
  let settings: Settings = {
    toggle_hotkey: "F6",
    capture_hotkey: "F7",
    stop: { type: "manual" },
    autostart: false,
    start_in_tray: false,
  };
  let sequence: Sequence = {
    steps: [
      {
        target: { position: { type: "current" }, button: "left", kind: { type: "single" } },
        interval: { value: 200, unit: "milliseconds" },
      },
    ],
  };
  const demo = new URLSearchParams(window.location.search).get("demo");
  if (demo === "sequence") {
    const fixed = (x: number, y: number) => ({
      position: { type: "fixed" as const, x, y },
      button: "left" as const,
      kind: { type: "single" as const },
    });
    sequence = {
      steps: [
        { target: fixed(120, 240), interval: { value: 500, unit: "milliseconds" } },
        {
          target: { position: { type: "current" }, button: "right", kind: { type: "hold", duration: { value: 2, unit: "seconds" } } },
          interval: { value: 1, unit: "seconds" },
        },
        { target: fixed(-1920, 80), interval: { value: 1.5, unit: "minutes" } },
      ],
    };
    settings = { ...settings, stop: { type: "rounds", count: 5 } };
  }
  let presets: Preset[] =
    demo === "sequence" ? [{ name: "ログイン", sequence }, { name: "毎朝の作業", sequence }] : [];
  let running = false;
  let demoStarted = false;
  let hotkeys: Hotkeys = { toggle: "F6", capture: "F7" };
  let rounds = 0;
  let timer: number | undefined;
  const subscribers = new Set<Listeners>();
  const emit = (event: RunEvent) => subscribers.forEach((s) => s.onRunEvent(event));
  const toggleRun = async () => {
    if (running) {
      window.clearInterval(timer);
      running = false;
      emit({ type: "state", data: "idle" });
      emit({ type: "finished", data: { type: "cancelled", rounds } });
      return;
    }
    running = true;
    rounds = 0;
    emit({ type: "state", data: "running" });
    timer = window.setInterval(() => emit({ type: "round", data: ++rounds }), 400);
  };

  return {
    getSettings: async () => settings,
    setStop: async (stop) => void (settings = { ...settings, stop }),
    async setAutostart(enabled) {
      // 見た目の確認用: ?demo=autostart-fails のときは、OS への登録に失敗したことにする。
      if (demo === "autostart-fails") return { ok: false, error: "アクセスが拒否されました" };
      settings = { ...settings, autostart: enabled };
      return { ok: true };
    },
    setStartInTray: async (enabled) => void (settings = { ...settings, start_in_tray: enabled }),
    getSequence: async () => sequence,
    saveSequence: async (next) => void (sequence = next),
    listPresets: async () => presets,
    savePreset: async (preset) => void (presets = [...presets.filter((p) => p.name !== preset.name), preset].sort((a, b) => a.name.localeCompare(b.name))),
    deletePreset: async (name) => void (presets = presets.filter((p) => p.name !== name)),
    toggle: toggleRun,
    warnings: async () => [],
    hotkeyStatus: async () => hotkeys,
    async setHotkeys(toggle, capture) {
      // 見た目の確認用: F1 は「他のアプリが使っている」ことにする。
      if (toggle === "F1" || capture === "F1") return { ok: false, error: { type: "conflict", data: "F1" } };
      if (toggle.trim().toLowerCase() === capture.trim().toLowerCase()) return { ok: false, error: { type: "duplicate" } };
      hotkeys = { toggle: toggle.trim(), capture: capture.trim() };
      return { ok: true, hotkeys };
    },
    subscribe(listeners) {
      subscribers.add(listeners);
      if (demo === "running" && !demoStarted) {
        demoStarted = true;
        window.setTimeout(() => void toggleRun(), 200);
      }
      return () => void subscribers.delete(listeners);
    },
  };
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const api: Api = inTauri ? tauriApi : createMockApi();
