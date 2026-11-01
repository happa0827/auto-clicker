// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HotkeysSection } from "./hotkeys-section";

afterEach(cleanup);

function setup() {
  const onApply = vi.fn().mockResolvedValue({ ok: true });
  render(<HotkeysSection toggle="F6" capture="F7" onApply={onApply} />);
  return { onApply };
}

describe("HotkeysSection recording", () => {
  it("fills the field with the next key pressed after clicking record", () => {
    setup();
    const record = screen.getByRole("button", { name: "開始と停止のキーを記録" });
    fireEvent.click(record);
    expect(record.getAttribute("aria-pressed")).toBe("true");

    fireEvent.keyDown(document.body, { code: "KeyS", ctrlKey: true, shiftKey: true });

    expect((screen.getByLabelText("開始と停止のキー") as HTMLInputElement).value).toBe("Ctrl+Shift+S");
    expect(record.getAttribute("aria-pressed")).toBe("false");
  });

  it("cancels on Escape and leaves the field as it was", () => {
    setup();
    const record = screen.getByRole("button", { name: "開始と停止のキーを記録" });
    fireEvent.click(record);
    fireEvent.keyDown(document.body, { code: "Escape" });

    expect(record.getAttribute("aria-pressed")).toBe("false");
    expect((screen.getByLabelText("開始と停止のキー") as HTMLInputElement).value).toBe("F6");
  });

  it("keeps listening while only a modifier is held", () => {
    setup();
    const record = screen.getByRole("button", { name: "座標を追加するキーを記録" });
    fireEvent.click(record);
    fireEvent.keyDown(document.body, { code: "ControlLeft", ctrlKey: true });
    expect(record.getAttribute("aria-pressed")).toBe("true");

    fireEvent.keyDown(document.body, { code: "KeyK", ctrlKey: true });
    expect((screen.getByLabelText("座標を追加するキー") as HTMLInputElement).value).toBe("Ctrl+K");
  });

  it("does not change the registered keys until the apply button is pressed", () => {
    const { onApply } = setup();
    fireEvent.click(screen.getByRole("button", { name: "開始と停止のキーを記録" }));
    fireEvent.keyDown(document.body, { code: "F9" });
    expect(onApply).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "キーを変更" }));
    expect(onApply).toHaveBeenCalledWith("F9", "F7");
  });
});
