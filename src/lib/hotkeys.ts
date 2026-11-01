// Rust (src-tauri/src/hotkeys.rs) が返す JSON の形と対応させる。

export interface Hotkeys {
  toggle: string | null;
  capture: string | null;
}

export type HotkeyError =
  | { type: "duplicate" }
  | { type: "conflict"; data: string }
  | { type: "invalid"; data: string };

const EXAMPLE = "例: F6 または Ctrl+Shift+S";

export function describeHotkeyError(error: HotkeyError): string {
  switch (error.type) {
    case "duplicate":
      return "開始と停止のキーと、キャプチャのキーは別にしてください。";
    case "conflict":
      return `${error.data} は他のアプリが使っています。別のキーを選んでください。元のキーのままです。`;
    case "invalid":
      return error.data === ""
        ? `キーを入力してください。${EXAMPLE}`
        : `${error.data} はキーとして使えません。${EXAMPLE}`;
  }
}

export interface KeyPress {
  code: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}

export type Recorded = { type: "done"; shortcut: string } | { type: "waiting" } | { type: "cancel" };

const MODIFIER_CODES = /^(Control|Shift|Alt|Meta|OS)(Left|Right)$/;

export function keyEventToShortcut(event: KeyPress): Recorded {
  if (MODIFIER_CODES.test(event.code)) return { type: "waiting" };
  const anyModifier = event.ctrlKey || event.altKey || event.shiftKey || event.metaKey;
  if (event.code === "Escape" && !anyModifier) return { type: "cancel" };
  const modifiers = [
    event.ctrlKey && "Ctrl",
    event.altKey && "Alt",
    event.shiftKey && "Shift",
    event.metaKey && "Super",
  ].filter(Boolean);
  return { type: "done", shortcut: [...modifiers, keyName(event.code)].join("+") };
}

function keyName(code: string): string {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1];
  const digit = /^Digit(\d)$/.exec(code);
  if (digit) return digit[1];
  return code;
}
