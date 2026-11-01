# 入力注入とホットキーに enigo と global-shortcut を使い、Linux は X11 のみ対応する

## Status
Accepted (2026-11-01)

## Context
クロスプラットフォームのオートクリッカーには、擬似クリックの注入と、アプリ外でも効くグローバルホットキーが必要。Wayland はセキュリティ上、この両方を制限している。

## Decision
- クリック注入は `enigo` クレート、グローバルホットキーは `tauri-plugin-global-shortcut` で実装する。
- 対象は Windows、macOS、Linux (X11)。Wayland は非対応とし、起動時に検出したら警告する。
- macOS は初回にアクセシビリティ権限を要求する。

## Alternatives
- OS ごとに Win32 `SendInput` などを直接呼ぶ: 精度は上がるが、実装量が増える。
- Wayland 向けに portal や uinput で対応する: 権限モデルが複雑で、v1 の範囲を超える。

## Consequences
Wayland ユーザーは使えない。将来 Wayland に対応する場合は、この ADR を見直す。
