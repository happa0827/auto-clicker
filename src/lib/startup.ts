export function describeAutostartError(reason: string): string {
  const trimmed = reason.trim();
  return trimmed === ""
    ? "自動起動の設定を変更できませんでした。"
    : `自動起動の設定を変更できませんでした: ${trimmed}`;
}
