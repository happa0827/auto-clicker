// Rust 側 (src-tauri/src/model.rs) の JSON 形と対応させる。

export type TimeUnit = "milliseconds" | "seconds" | "minutes";

export interface Interval {
  value: number;
  unit: TimeUnit;
}

export type Position = { type: "current" } | { type: "fixed"; x: number; y: number };

export type Button = "left" | "right" | "middle";

export type ClickKind = { type: "single" } | { type: "double" } | { type: "hold"; duration: Interval };

export interface ClickTarget {
  position: Position;
  button: Button;
  kind: ClickKind;
}

export interface Step {
  target: ClickTarget;
  interval: Interval;
}

export interface Sequence {
  steps: Step[];
}

export interface Preset {
  name: string;
  sequence: Sequence;
}

export type StopCondition = { type: "manual" } | { type: "rounds"; count: number };

export interface Settings {
  toggle_hotkey: string;
  capture_hotkey: string;
  stop: StopCondition;
  autostart: boolean;
  start_in_tray: boolean;
}
