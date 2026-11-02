import { useState } from "react";
import { Alert } from "@/components/arc/alert/alert";
import { Button } from "@/components/arc/button/button";
import { api } from "@/lib/api";
import styles from "./about-section.module.css";

type Result = { tone: "info" | "danger"; title: string };

export function AboutSection({ version }: { version: string }) {
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<Result | null>(null);

  async function check() {
    setChecking(true);
    setResult(null);
    const outcome = await api.checkForUpdate();
    if (!outcome.ok) {
      setResult({ tone: "danger", title: `更新を確認できませんでした。${outcome.error}` });
    } else if (outcome.outcome === "up_to_date") {
      setResult({ tone: "info", title: "最新のバージョンです。" });
    }
    setChecking(false);
  }

  return (
    <div className={styles.section}>
      <p className={styles.version}>バージョン {version}</p>
      <div className={styles.actions}>
        <Button variant="secondary" loading={checking} onClick={() => void check()}>
          更新を確認
        </Button>
        <Button variant="ghost" onClick={() => void api.openRepository()}>
          GitHub を開く
        </Button>
      </div>
      {result ? <Alert tone={result.tone} title={result.title} onDismiss={() => setResult(null)} /> : null}
    </div>
  );
}
