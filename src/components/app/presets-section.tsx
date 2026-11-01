import { Trash2 } from "lucide-react";
import { Button } from "@/components/arc/button/button";
import { ConfirmMorph } from "@/components/arc/confirm-morph/confirm-morph";
import { Input } from "@/components/arc/input/input";
import { MorphSelect } from "@/components/arc/morph-select/morph-select";
import type { Preset } from "@/lib/types";
import styles from "./presets-section.module.css";

interface Props {
  presets: Preset[];
  selected: string;
  name: string;
  canSave: boolean;
  disabled?: boolean;
  onSelect: (name: string) => void;
  onNameChange: (name: string) => void;
  onLoad: () => void;
  onSave: () => Promise<void>;
  onDelete: () => Promise<void>;
  onUndoDelete: () => Promise<void>;
}

export function PresetsSection({
  presets,
  selected,
  name,
  canSave,
  disabled,
  onSelect,
  onNameChange,
  onLoad,
  onSave,
  onDelete,
  onUndoDelete,
}: Props) {
  const hasSelection = selected !== "" && presets.some((p) => p.name === selected);

  return (
    <div className={styles.section}>
      <MorphSelect
        label="保存した Preset"
        placeholder={presets.length === 0 ? "まだありません" : "選んでください"}
        value={selected || null}
        disabled={disabled || presets.length === 0}
        onValueChange={onSelect}
        items={presets.map((p) => ({ value: p.name, label: p.name }))}
      />
      <div className={styles.actions}>
        <Button variant="secondary" disabled={disabled || !hasSelection} onClick={onLoad}>
          読み込む
        </Button>
        <ConfirmMorph
          label="削除"
          icon={<Trash2 size={16} strokeWidth={1.75} />}
          prompt="この Preset を削除しますか"
          confirmLabel="削除する"
          cancelLabel="やめる"
          pendingLabel="削除中"
          doneLabel="削除しました"
          undoLabel="元に戻す"
          disabled={disabled || !hasSelection}
          onConfirm={onDelete}
          onUndo={onUndoDelete}
        />
      </div>
      <div className={styles.save}>
        <Input
          label="Preset の名前"
          disabled={disabled}
          value={name}
          onChange={(event) => onNameChange(event.target.value)}
        />
        <Button variant="primary" disabled={disabled || !canSave || name.trim() === ""} onClick={onSave}>
          保存
        </Button>
      </div>
    </div>
  );
}
