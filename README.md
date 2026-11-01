# auto-clicker

汎用のデスクトップ用オートクリッカーです。Tauri 2、React 19、Rust で作っています。

## できること

- **3 つのモード**
  - 現在位置: カーソルがある場所をクリックし続ける。
  - 固定座標: 指定した座標をクリックし続ける。
  - シーケンス: 複数の座標（Step）を順にクリックする。
- **クリックの種類**: 左・右・中ボタン、単発・ダブルクリック・長押し。
- **間隔**: 数値と単位（ミリ秒・秒・分）で指定する。最小は 1 ミリ秒。
- **停止条件**: 手動で止める、または指定した回数で止める。シーケンスは、全 Step を 1 周して 1 回と数える。
- **グローバルホットキー**: アプリが裏にあっても効く。初期値は F6（開始と停止）と F7（今のカーソル位置を取り込む）。設定で変更できる。
- **Preset**: シーケンスに名前を付けて保存・読み込み・削除できる。最後に使った設定は自動で保存され、次の起動で復元される。
- **トレイ常駐**: × ボタンはトレイに格納する。トレイのメニューから開始・停止・終了ができる。
- **起動の設定**: ログイン時の自動起動、ウィンドウを出さずにトレイで起動、のそれぞれをオン・オフできる。多重起動はできない。
- **安全のための停止**: 実行中にモニターの構成（解像度、台数）が変わると自動で止まる。画面の外の座標があると、開始前に拒否する。

## 対応環境

| OS | 状態 |
|---|---|
| Windows 11 | 動作確認済み |
| macOS | 未確認。初回に「アクセシビリティ」の権限が必要。権限がないときの警告は未実装 |
| Linux (X11) | 未確認。トレイには appindicator が必要（なければ × ボタンで終了する） |
| Linux (Wayland) | **非対応**。Wayland は他のアプリの入力操作とグローバルホットキーを制限しているため、動かない。起動時に警告を出す |

## 開発

必要なもの: [mise](https://mise.jdx.dev/)（Rust）、[Bun](https://bun.sh/)、Tauri の[前提ツール](https://tauri.app/start/prerequisites/)。

```bash
bun install
bun run tauri dev      # アプリを起動する
bun run test           # フロントのテスト（vitest）
bun run build          # 型チェックとビルド
cd src-tauri && cargo test --lib   # Rust のテスト
```

ブラウザだけで見た目を確認するときは `bun run dev` を使う。Tauri の代わりにモックが動く。`?demo=sequence`、`?demo=running` で状態を切り替えられる。

## 構成

- `src-tauri/src/` — Rust 側。入力注入、実行ループ、ホットキー、トレイ、保存。
  - `model` 値と保存形式、`validate` 開始前の検査、`runner` 実行ループ、`controller` 開始・停止・画面監視
  - `input` / `clock` / `enigo_backend` OS との境界（テストではフェイクに差し替える）
  - `hotkeys` / `autostart` / `tray` / `window_policy` 常駐まわりの判断
  - `store` 設定、最後の Sequence、Preset の保存
- `src/` — 画面（React、UI は [Arc](https://uiarc.dev/)）。入力検査などの純ロジックは `src/lib/` にあり、テストされている。

## ドキュメント

- [`GLOSSARY.md`](GLOSSARY.md): 用語集（Click Run、Click Target、Sequence、Step、Interval、Round など）
- [`docs/adr/`](docs/adr/): 設計の判断記録
- [`docs/PLAN.md`](docs/PLAN.md): 実装計画
- [`docs/manual-check.md`](docs/manual-check.md): 自動テストできない部分の手動確認チェックリスト

## 既知の制限

- 1 ミリ秒の間隔は目標値で、精度は OS のスケジューラに左右される。
- 保存先は OS のアプリデータ領域（Windows は `%APPDATA%\com.heyyo.auto-clicker`）。
