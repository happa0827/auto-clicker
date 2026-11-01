import { useEffect, useState } from "react";
import { Alert } from "@/components/arc/alert/alert";
import { Button } from "@/components/arc/button/button";
import { Input } from "@/components/arc/input/input";
import {
  describeHotkeyError,
  keyEventToShortcut,
  type HotkeyError,
} from "@/lib/hotkeys";
import styles from "./hotkeys-section.module.css";

interface Props {
  toggle: string | null;
  capture: string | null;
  disabled?: boolean;
  onApply: (
    toggle: string,
    capture: string,
  ) => Promise<{ ok: true } | { ok: false; error: HotkeyError }>;
}

export function HotkeysSection({ toggle, capture, disabled, onApply }: Props) {
  const [toggleText, setToggleText] = useState(toggle ?? "");
  const [captureText, setCaptureText] = useState(capture ?? "");
  const [error, setError] = useState<HotkeyError | null>(null);
  const [saved, setSaved] = useState(false);
  const [recording, setRecording] = useState<"toggle" | "capture" | null>(null);

  // 記録中は、次に押したキーをその欄に入れる。ブラウザ側の動作（Tab 移動など）は止める。
  useEffect(() => {
    if (!recording) return;
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      const result = keyEventToShortcut(event);
      if (result.type === "waiting") return;
      setRecording(null);
      if (result.type === "cancel") return;
      (recording === "toggle" ? setToggleText : setCaptureText)(
        result.shortcut,
      );
      setSaved(false);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [recording]);

  // 登録結果が届いたら、入力欄に反映する。
  useEffect(() => setToggleText(toggle ?? ""), [toggle]);
  useEffect(() => setCaptureText(capture ?? ""), [capture]);

  const unchanged =
    toggleText.trim() === (toggle ?? "") &&
    captureText.trim() === (capture ?? "");

  async function apply() {
    setSaved(false);
    const result = await onApply(toggleText, captureText);
    if (result.ok) {
      setError(null);
      setSaved(true);
    } else {
      setError(result.error);
    }
  }

  return (
    <div className={styles.section}>
      <div className={styles.pair}>
        <div className={styles.field}>
          <Input
            label="開始と停止のキー"
            disabled={disabled}
            value={toggleText}
            description="例: F6 または Ctrl+Shift+S"
            onChange={(event) => {
              setToggleText(event.target.value);
              setSaved(false);
            }}
          />
          <Button
            size="sm"
            variant="secondary"
            disabled={disabled}
            aria-label="開始と停止のキーを記録"
            aria-pressed={recording === "toggle"}
            onClick={() =>
              setRecording(recording === "toggle" ? null : "toggle")
            }
          >
            {recording === "toggle" ? "キーを押してください…" : "キーを記録"}
          </Button>
        </div>
        <div className={styles.field}>
          <Input
            label="座標を追加するキー"
            disabled={disabled}
            value={captureText}
            description="今のカーソル位置を取り込みます。"
            onChange={(event) => {
              setCaptureText(event.target.value);
              setSaved(false);
            }}
          />
          <Button
            size="sm"
            variant="secondary"
            disabled={disabled}
            aria-label="座標を追加するキーを記録"
            aria-pressed={recording === "capture"}
            onClick={() =>
              setRecording(recording === "capture" ? null : "capture")
            }
          >
            {recording === "capture" ? "キーを押してください…" : "キーを記録"}
          </Button>
        </div>
      </div>
      {error ? (
        <Alert tone="danger" title={describeHotkeyError(error)} />
      ) : null}
      <div>
        <Button
          variant="secondary"
          disabled={disabled || unchanged}
          onClick={apply}
        >
          {saved ? "変更しました" : "キーを変更"}
        </Button>
      </div>
    </div>
  );
}
