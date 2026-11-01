import { Input } from "@/components/arc/input/input";
import { MorphSelect } from "@/components/arc/morph-select/morph-select";
import type { IntervalDraft, RowErrors, TargetDraft } from "@/lib/draft";
import { describeFieldError } from "@/lib/field-errors";
import type { Button, TimeUnit } from "@/lib/types";
import styles from "./target-fields.module.css";

const UNITS: { value: TimeUnit; label: string }[] = [
  { value: "milliseconds", label: "ミリ秒" },
  { value: "seconds", label: "秒" },
  { value: "minutes", label: "分" },
];

const BUTTONS: { value: Button; label: string }[] = [
  { value: "left", label: "左" },
  { value: "right", label: "右" },
  { value: "middle", label: "中" },
];

const KINDS: { value: TargetDraft["kind"]; label: string }[] = [
  { value: "single", label: "単発" },
  { value: "double", label: "ダブルクリック" },
  { value: "hold", label: "長押し" },
];

interface Props {
  idPrefix: string;
  target: TargetDraft;
  interval: IntervalDraft;
  errors?: RowErrors;
  /** 座標の入力欄を出す（固定座標モードとシーケンスの行）。 */
  showCoordinates: boolean;
  /** 位置の種類（現在位置か固定座標）を選べるようにする（シーケンスの行）。 */
  choosePosition?: boolean;
  disabled?: boolean;
  onChange: (target: TargetDraft, interval: IntervalDraft) => void;
}

export function TargetFields({
  idPrefix,
  target,
  interval,
  errors = {},
  showCoordinates,
  choosePosition = false,
  disabled,
  onChange,
}: Props) {
  const setTarget = (patch: Partial<TargetDraft>) => onChange({ ...target, ...patch }, interval);
  const setInterval = (patch: Partial<IntervalDraft>) => onChange(target, { ...interval, ...patch });
  const fixed = !choosePosition || target.position === "fixed";

  return (
    <div className={styles.fields}>
      {choosePosition ? (
        <MorphSelect
          label="位置"
          value={target.position}
          disabled={disabled}
          onValueChange={(value) => setTarget({ position: value as TargetDraft["position"] })}
          items={[
            { value: "fixed", label: "固定座標" },
            { value: "current", label: "現在位置" },
          ]}
        />
      ) : null}

      {showCoordinates && fixed ? (
        <div className={styles.pair}>
          <Input
            label="X"
            inputMode="numeric"
            disabled={disabled}
            value={target.x}
            error={describeFieldError(errors.x)}
            onChange={(event) => setTarget({ x: event.target.value })}
          />
          <Input
            label="Y"
            inputMode="numeric"
            disabled={disabled}
            value={target.y}
            error={describeFieldError(errors.y)}
            onChange={(event) => setTarget({ y: event.target.value })}
          />
        </div>
      ) : null}

      <div className={styles.pair}>
        <MorphSelect
          label="ボタン"
          value={target.button}
          disabled={disabled}
          onValueChange={(value) => setTarget({ button: value as Button })}
          items={BUTTONS}
        />
        <MorphSelect
          align="end"
          label="クリックの種類"
          value={target.kind}
          disabled={disabled}
          onValueChange={(value) => setTarget({ kind: value as TargetDraft["kind"] })}
          items={KINDS}
        />
      </div>

      {target.kind === "hold" ? (
        <div className={styles.valueWithUnit}>
          <Input
            label="押し続ける時間"
            inputMode="decimal"
            disabled={disabled}
            value={target.holdText}
            error={describeFieldError(errors.hold)}
            onChange={(event) => setTarget({ holdText: event.target.value })}
          />
          <MorphSelect
            align="end"
            label="単位"
            value={target.holdUnit}
            disabled={disabled}
            onValueChange={(value) => setTarget({ holdUnit: value as TimeUnit })}
            items={UNITS}
          />
        </div>
      ) : null}

      <div className={styles.valueWithUnit}>
        <Input
          id={`${idPrefix}-interval`}
          label="クリックの間隔"
          inputMode="decimal"
          disabled={disabled}
          value={interval.text}
          error={describeFieldError(errors.interval)}
          onChange={(event) => setInterval({ text: event.target.value })}
        />
        <MorphSelect
          align="end"
          label="単位"
          value={interval.unit}
          disabled={disabled}
          onValueChange={(value) => setInterval({ unit: value as TimeUnit })}
          items={UNITS}
        />
      </div>
    </div>
  );
}
