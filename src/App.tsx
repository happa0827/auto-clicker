import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Alert } from "@/components/arc/alert/alert";
import { Badge } from "@/components/arc/badge/badge";
import { Button } from "@/components/arc/button/button";
import { Input } from "@/components/arc/input/input";
import { RadioGroup } from "@/components/arc/radio-group/radio-group";
import SegmentedControl from "@/components/arc/segmented-control/segmented-control";
import { HotkeysSection } from "@/components/app/hotkeys-section";
import { PresetsSection } from "@/components/app/presets-section";
import { StartupSection } from "@/components/app/startup-section";
import { StepsEditor } from "@/components/app/steps-editor";
import { TargetFields } from "@/components/app/target-fields";
import { useRunStatus } from "@/hooks/use-run-status";
import { api } from "@/lib/api";
import {
  applyCapture,
  buildSequence,
  defaultDraft,
  draftFromSequence,
  type Draft,
  type Mode,
} from "@/lib/draft";
import { describeFieldError } from "@/lib/field-errors";
import { parseRounds } from "@/lib/rounds";
import { describeNotice } from "@/lib/run-status";
import type { Preset, Settings, StopCondition } from "@/lib/types";
import pkg from "../package.json";
import styles from "./App.module.css";

const MODES: { value: Mode; label: string }[] = [
  { value: "current", label: "現在位置" },
  { value: "fixed", label: "固定座標" },
  { value: "sequence", label: "シーケンス" },
];

type Tab = "click" | "app";

const TABS: { value: Tab; label: string }[] = [
  { value: "click", label: "クリック設定" },
  { value: "app", label: "アプリ設定" },
];

const ENVIRONMENT_WARNINGS: Record<string, string> = {
  WaylandUnsupported:
    "Wayland では動作しません。X11 のセッションで使ってください。",
  MacAccessibilityMissing:
    "macOS のアクセシビリティ権限が必要です。システム設定で許可してください。",
};

const NOTICE_TONE = {
  info: "info",
  warning: "warning",
  error: "danger",
} as const;

export default function App() {
  const [draft, setDraft] = useState<Draft>(defaultDraft());
  const [stopMode, setStopMode] = useState<"manual" | "rounds">("manual");
  const [roundsText, setRoundsText] = useState("10");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState("");
  const [presetName, setPresetName] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [tab, setTab] = useState<Tab>("click");
  const deletedPreset = useRef<Preset | null>(null);

  const onCaptured = useCallback(
    ({ x, y }: { x: number; y: number }) =>
      setDraft((current) => applyCapture(current, x, y)),
    [],
  );
  const [run, dispatch] = useRunStatus(onCaptured);
  const running = run.status === "running";

  useEffect(() => {
    Promise.all([api.getSettings(), api.getSequence(), api.listPresets()]).then(
      ([s, sequence, list]) => {
        setSettings(s);
        setDraft(draftFromSequence(sequence));
        setPresets(list);
        if (s.stop.type === "rounds") {
          setStopMode("rounds");
          setRoundsText(String(s.stop.count));
        }
        setLoaded(true);
      },
    );
  }, []);

  const built = useMemo(() => buildSequence(draft), [draft]);
  const rounds = useMemo(() => parseRounds(roundsText), [roundsText]);
  const stop: StopCondition | null =
    stopMode === "manual"
      ? { type: "manual" }
      : rounds.ok
        ? { type: "rounds", count: rounds.count }
        : null;
  const valid = built.ok && stop !== null;

  // 入力が正しい間は、実行内容を自動で保存する。ホットキーで開始したときも最新の内容で動く。
  const lastSaved = useRef("");
  useEffect(() => {
    if (!loaded || !built.ok || !stop) return;
    const snapshot = JSON.stringify([built.sequence, stop]);
    if (snapshot === lastSaved.current) return;
    const timer = window.setTimeout(() => {
      lastSaved.current = snapshot;
      void api.saveSequence(built.sequence);
      void api.setStop(stop);
    }, 150);
    return () => window.clearTimeout(timer);
  }, [loaded, built, stop]);

  async function toggle() {
    if (!running && built.ok && stop) {
      lastSaved.current = JSON.stringify([built.sequence, stop]);
      await api.saveSequence(built.sequence);
      await api.setStop(stop);
    }
    await api.toggle();
  }

  async function refreshPresets() {
    setPresets(await api.listPresets());
  }

  async function savePreset() {
    if (!built.ok) return;
    const name = presetName.trim();
    await api.savePreset({ name, sequence: built.sequence });
    await refreshPresets();
    setSelectedPreset(name);
  }

  const toggleKey = run.hotkeys.toggle;
  const errors = built.ok ? undefined : built.errors;

  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <p className={styles.version}>v{pkg.version}</p>
        <div className={styles.titleRow}>
          <h1 className={styles.title}>オートクリッカー</h1>
          <Badge tone={running ? "success" : "neutral"} dot={running}>
            {running ? "実行中" : "待機中"}
          </Badge>
        </div>
        <p className={styles.rounds} aria-live="polite">
          {running || run.rounds > 0
            ? `実行済み ${run.rounds} 回`
            : "開始すると、ここに回数が出ます。"}
        </p>
        <Button
          size="lg"
          variant={running ? "secondary" : "primary"}
          disabled={!running && !valid}
          onClick={toggle}
        >
          {running ? "停止" : "開始"}
        </Button>
        <p className={styles.hint}>
          {toggleKey
            ? `${toggleKey} キーでも開始と停止ができます。`
            : "ホットキーが使えないため、このボタンで操作します。"}
        </p>
      </header>

      <div className={styles.notices}>
        {run.notice ? (
          <Alert
            tone={NOTICE_TONE[run.notice.kind]}
            title={describeNotice(run.notice)}
            onDismiss={() => dispatch({ type: "dismiss-notice" })}
          />
        ) : null}
        {run.environmentWarnings.map((code) => (
          <Alert
            key={code}
            tone="warning"
            title={ENVIRONMENT_WARNINGS[code] ?? code}
          />
        ))}
        {loaded && run.hotkeys.known && !run.hotkeys.ready ? (
          <Alert tone="warning" title="ホットキーを登録できていません。">
            他のアプリが同じキーを使っている可能性があります。ホットキーが使えるようになるまで、開始できません。
          </Alert>
        ) : null}
        {loaded && !valid ? (
          <Alert tone="warning" title="入力に誤りがあります。">
            直すまでは、最後に保存された設定が使われます。
          </Alert>
        ) : null}
      </div>

      <SegmentedControl
        className={styles.tabs}
        label="設定"
        value={tab}
        onValueChange={(v) => setTab(v as Tab)}
        options={TABS}
      />

      {tab === "click" ? (
        <>
          <section className={styles.section} aria-labelledby="click-heading">
            <h2 id="click-heading" className={styles.heading}>
              クリックの設定
            </h2>
            <SegmentedControl
              className={styles.tabs}
              label="モード"
              value={draft.mode}
              onValueChange={(mode) =>
                setDraft({ ...draft, mode: mode as Mode })
              }
              options={MODES}
            />
            {draft.mode === "sequence" ? (
              <StepsEditor
                steps={draft.steps}
                errors={errors}
                captureKey={run.hotkeys.capture}
                disabled={running}
                onChange={(steps) => setDraft({ ...draft, steps })}
              />
            ) : (
              <>
                {draft.mode === "fixed" && run.hotkeys.capture ? (
                  <p className={styles.hint}>
                    {run.hotkeys.capture}{" "}
                    を押すと、今のカーソル位置が座標に入ります。
                  </p>
                ) : null}
                <TargetFields
                  idPrefix="single"
                  target={draft.target}
                  interval={draft.interval}
                  errors={errors}
                  showCoordinates={draft.mode === "fixed"}
                  disabled={running}
                  onChange={(target, interval) =>
                    setDraft({ ...draft, target, interval })
                  }
                />
              </>
            )}
            {errors?.steps === "empty" ? (
              <p className={styles.error}>Step を 1 つ以上追加してください。</p>
            ) : null}
          </section>

          <section className={styles.section} aria-labelledby="stop-heading">
            <h2 id="stop-heading" className={styles.heading}>
              停止条件
            </h2>
            <RadioGroup
              label="いつ止めますか"
              name="stop"
              value={stopMode}
              onValueChange={(value) =>
                setStopMode(value as "manual" | "rounds")
              }
              options={[
                {
                  value: "manual",
                  label: "手動で止める",
                  description: "停止ボタンかホットキーで止めます。",
                },
                {
                  value: "rounds",
                  label: "回数を決める",
                  description:
                    "シーケンスは、全 Step を 1 周で 1 回と数えます。",
                },
              ]}
            />
            {stopMode === "rounds" ? (
              <Input
                label="回数"
                inputMode="numeric"
                disabled={running}
                value={roundsText}
                error={rounds.ok ? undefined : describeFieldError(rounds.error)}
                onChange={(event) => setRoundsText(event.target.value)}
              />
            ) : null}
          </section>

          <section className={styles.section} aria-labelledby="preset-heading">
            <h2 id="preset-heading" className={styles.heading}>
              Preset
            </h2>
            <PresetsSection
              presets={presets}
              selected={selectedPreset}
              name={presetName}
              canSave={built.ok}
              disabled={running}
              onSelect={(name) => {
                setSelectedPreset(name);
                setPresetName(name);
              }}
              onNameChange={setPresetName}
              onLoad={() => {
                const preset = presets.find((p) => p.name === selectedPreset);
                if (preset) setDraft(draftFromSequence(preset.sequence));
              }}
              onSave={savePreset}
              onDelete={async () => {
                deletedPreset.current =
                  presets.find((p) => p.name === selectedPreset) ?? null;
                await api.deletePreset(selectedPreset);
                await refreshPresets();
              }}
              onUndoDelete={async () => {
                if (deletedPreset.current) {
                  await api.savePreset(deletedPreset.current);
                  await refreshPresets();
                }
              }}
            />
          </section>
        </>
      ) : (
        <>
          <section className={styles.section} aria-labelledby="hotkey-heading">
            <h2 id="hotkey-heading" className={styles.heading}>
              ホットキー
            </h2>
            <HotkeysSection
              toggle={run.hotkeys.toggle}
              capture={run.hotkeys.capture}
              disabled={running}
              onApply={async (toggle, capture) => {
                const result = await api.setHotkeys(toggle, capture);
                if (!result.ok) return result;
                dispatch({
                  type: "hotkeys",
                  toggle: result.hotkeys.toggle,
                  capture: result.hotkeys.capture,
                });
                return { ok: true };
              }}
            />
          </section>

          <section className={styles.section} aria-labelledby="startup-heading">
            <h2 id="startup-heading" className={styles.heading}>
              起動
            </h2>
            {settings ? (
              <StartupSection
                autostart={settings.autostart}
                startInTray={settings.start_in_tray}
                onAutostart={async (enabled) => {
                  const result = await api.setAutostart(enabled);
                  if (result.ok)
                    setSettings({ ...settings, autostart: enabled });
                  return result;
                }}
                onStartInTray={async (enabled) => {
                  await api.setStartInTray(enabled);
                  setSettings({ ...settings, start_in_tray: enabled });
                }}
              />
            ) : null}
          </section>
        </>
      )}
    </main>
  );
}
