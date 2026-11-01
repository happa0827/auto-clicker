import { describe, expect, it } from "vitest";
import { describeHotkeyError, keyEventToShortcut } from "./hotkeys";

describe("describeHotkeyError", () => {
  it("says the two keys must differ", () => {
    expect(describeHotkeyError({ type: "duplicate" })).toBe("開始と停止のキーと、キャプチャのキーは別にしてください。");
  });

  it("names the key that another app already uses", () => {
    expect(describeHotkeyError({ type: "conflict", data: "F9" })).toBe(
      "F9 は他のアプリが使っています。別のキーを選んでください。元のキーのままです。",
    );
  });

  it("names the key that is not a valid key", () => {
    expect(describeHotkeyError({ type: "invalid", data: "Banana" })).toBe(
      "Banana はキーとして使えません。例: F6 または Ctrl+Shift+S",
    );
  });

  it("asks for input when the key is empty", () => {
    expect(describeHotkeyError({ type: "invalid", data: "" })).toBe("キーを入力してください。例: F6 または Ctrl+Shift+S");
  });
});

const press = (code: string, mods: Partial<{ ctrlKey: boolean; shiftKey: boolean; altKey: boolean; metaKey: boolean }> = {}) => ({
  code,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  metaKey: false,
  ...mods,
});

describe("keyEventToShortcut", () => {
  it("turns a function key into its name", () => {
    expect(keyEventToShortcut(press("F6"))).toEqual({ type: "done", shortcut: "F6" });
  });

  it("writes letters and digits without the Key/Digit prefix", () => {
    expect(keyEventToShortcut(press("KeyS"))).toEqual({ type: "done", shortcut: "S" });
    expect(keyEventToShortcut(press("Digit1"))).toEqual({ type: "done", shortcut: "1" });
  });

  it("puts modifiers first in a fixed order", () => {
    expect(keyEventToShortcut(press("KeyS", { shiftKey: true, ctrlKey: true }))).toEqual({
      type: "done",
      shortcut: "Ctrl+Shift+S",
    });
    expect(keyEventToShortcut(press("F6", { metaKey: true, altKey: true }))).toEqual({
      type: "done",
      shortcut: "Alt+Super+F6",
    });
  });

  it("keeps waiting while only a modifier is held", () => {
    for (const code of ["ControlLeft", "ShiftRight", "AltLeft", "MetaLeft", "OSRight"]) {
      expect(keyEventToShortcut(press(code, { ctrlKey: true }))).toEqual({ type: "waiting" });
    }
  });

  it("cancels on a bare Escape but accepts Escape with a modifier", () => {
    expect(keyEventToShortcut(press("Escape"))).toEqual({ type: "cancel" });
    expect(keyEventToShortcut(press("Escape", { ctrlKey: true }))).toEqual({ type: "done", shortcut: "Ctrl+Escape" });
  });
});
