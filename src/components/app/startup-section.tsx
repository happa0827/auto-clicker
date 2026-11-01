import { useState } from "react";
import { Alert } from "@/components/arc/alert/alert";
import { Switch } from "@/components/arc/switch/switch";
import { describeAutostartError } from "@/lib/startup";
import styles from "./startup-section.module.css";

interface Props {
  autostart: boolean;
  startInTray: boolean;
  onAutostart: (enabled: boolean) => Promise<{ ok: true } | { ok: false; error: string }>;
  onStartInTray: (enabled: boolean) => Promise<void>;
}

export function StartupSection({ autostart, startInTray, onAutostart, onStartInTray }: Props) {
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function changeAutostart(enabled: boolean) {
    setPending(true);
    setError(null);
    const result = await onAutostart(enabled);
    if (!result.ok) setError(describeAutostartError(result.error));
    setPending(false);
  }

  return (
    <div className={styles.section}>
      <div className={styles.item}>
        <Switch
          label="ログイン時に自動で起動する"
          checked={autostart}
          disabled={pending}
          onCheckedChange={changeAutostart}
        />
      </div>
      {error ? <Alert tone="danger" title={error} onDismiss={() => setError(null)} /> : null}
      <div className={styles.item}>
        <Switch
          label="ウィンドウを出さずにトレイで起動する"
          checked={startInTray}
          onCheckedChange={(enabled) => void onStartInTray(enabled)}
        />
        <p className={styles.hint}>
          トレイのアイコンから開けます。トレイが使えない環境では、この設定は無視されます。
        </p>
      </div>
    </div>
  );
}
