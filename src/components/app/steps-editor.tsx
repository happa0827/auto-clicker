import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/arc/button/button";
import { EmptyState } from "@/components/arc/empty-state/empty-state";
import { newStepDraft, type DraftErrors, type StepDraft } from "@/lib/draft";
import { addStep, moveStep, removeStep, updateStep } from "@/lib/steps";
import { TargetFields } from "./target-fields";
import styles from "./steps-editor.module.css";

interface Props {
  steps: StepDraft[];
  errors?: DraftErrors;
  captureKey: string | null;
  disabled?: boolean;
  onChange: (steps: StepDraft[]) => void;
}

export function StepsEditor({ steps, errors, captureKey, disabled, onChange }: Props) {
  const hint = captureKey
    ? `${captureKey} を押すと、今のカーソル位置が Step として追加されます。`
    : "ホットキーが使えないため、Step は手入力で追加します。";

  return (
    <div className={styles.editor}>
      <p className={styles.hint}>{hint}</p>

      {steps.length === 0 ? (
        <EmptyState title="Step がありません" description="追加すると、上から順にクリックします。" />
      ) : (
        <ol className={styles.list}>
          {steps.map((step, index) => (
            <li key={index} className={styles.row}>
              <div className={styles.rowHeader}>
                <h3 className={styles.rowTitle}>Step {index + 1}</h3>
                <div className={styles.rowActions}>
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={`Step ${index + 1} を上へ`}
                    disabled={disabled || index === 0}
                    onClick={() => onChange(moveStep(steps, index, index - 1))}
                  >
                    <ArrowUp size={16} strokeWidth={1.75} aria-hidden="true" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={`Step ${index + 1} を下へ`}
                    disabled={disabled || index === steps.length - 1}
                    onClick={() => onChange(moveStep(steps, index, index + 1))}
                  >
                    <ArrowDown size={16} strokeWidth={1.75} aria-hidden="true" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    aria-label={`Step ${index + 1} を削除`}
                    disabled={disabled}
                    onClick={() => onChange(removeStep(steps, index))}
                  >
                    <Trash2 size={16} strokeWidth={1.75} aria-hidden="true" />
                  </Button>
                </div>
              </div>
              <TargetFields
                idPrefix={`step-${index}`}
                target={step.target}
                interval={step.interval}
                errors={errors?.stepErrors?.[index]}
                showCoordinates
                choosePosition
                disabled={disabled}
                onChange={(target, interval) => onChange(updateStep(steps, index, { target, interval }))}
              />
            </li>
          ))}
        </ol>
      )}

      <Button variant="secondary" disabled={disabled} onClick={() => onChange(addStep(steps, newStepDraft(0, 0)))}>
        <Plus size={16} strokeWidth={1.75} aria-hidden="true" />
        Step を追加
      </Button>
    </div>
  );
}
