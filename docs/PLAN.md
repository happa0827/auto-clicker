# 実装計画

用語は `GLOSSARY.md`、設計判断は `docs/adr/` を参照。

## 方針
- ロジックは Rust 側（`src-tauri/src/`）に置き、React は表示と設定入力だけを担う。
- OS 依存の入力注入は trait の裏に隠し、実行ループをフェイクでテストできるようにする（TDD、`cargo test`）。
- 各フェーズの最後に、動く状態で確認する。

## モジュール構成（Rust）
| モジュール | 役割 |
|---|---|
| `model` | `ClickTarget`, `Step`, `Sequence`, `Interval`, `Settings`, `Preset`（serde 対応） |
| `validate` | 開始前検査（画面外の Step、Interval 最小値、Step 数 0） |
| `input` | `InputBackend` trait（click, press, release, cursor_position）と `enigo` 実装 |
| `runner` | Click Run の実行ループ。別スレッド、キャンセル可能、Round 数のカウント、停止条件 |
| `hotkeys` | トグル・キャプチャの登録、競合検出 |
| `display` | モニター構成の取得と変更検出 |
| `store` | 設定と Preset の JSON 保存・復元 |
| `tray` | トレイアイコン、メニュー、状態でのアイコン切替 |
| `commands` | フロントに公開する Tauri コマンドとイベント |

## フェーズ

### 1. モデルと検査（純ロジック）
- `model` と `validate` をテスト先行で作る。
- テスト: Interval の単位換算と最小 1 ms、画面外 Step の検出、単独モードが 1 Step の Sequence になること、JSON の往復。
- 完了条件: `cargo test` が通る。

### 2. 実行ループ
- `InputBackend` trait とフェイク実装を作る。
- `runner` を実装する。
  - 1 つの実行ループで全モードを扱う（ADR 0002）。
  - 手動停止、N Round での終了、長押しは Hold Duration → Interval の順で進める。
  - 注入エラーで停止して理由を返す。
- テスト: Round 数、Step の順序、停止の即応、長押しのタイミング、エラー停止。
- 完了条件: フェイクでの全シナリオが通る。

### 3. 実入力とホットキー
- `enigo` 実装（ADR 0001）を作る。「現在位置」モードはクリック時にカーソル位置を取得する。
- `tauri-plugin-global-shortcut` で F6（トグル）と F7（キャプチャ）を登録する。競合時は登録失敗を状態として保持し、開始を拒否する。
- 起動時に Wayland と macOS の権限を検出して警告を返す。
- 完了条件: メモ帳などに対して、実際にクリックの開始・停止ができる（Windows で手動確認）。

### 4. 画面構成の監視
- `display` で構成のスナップショットを取り、実行中に変化を検出したら Click Run を停止して通知する。
- 完了条件: モニターの接続や解像度変更で停止する（手動確認）。

### 5. 永続化
- `store` で最後の設定の自動保存と復元、名前付き Preset の保存・読込・削除を実装する。保存先は OS のアプリデータ領域。
- テスト: 破損 JSON の読込時に既定値へ戻ること、Preset の往復。

### 6. UI（React）
- 画面: 状態表示（待機中/実行中、Round 数）、モード選択、Step 一覧（数値編集・並べ替え・削除）、Interval 入力（数値 + 単位ドロップダウン）、停止条件、ホットキー設定、Preset 管理、設定（自動起動、トレイ起動）。
- Rust からのイベント（状態、Round 数、エラー）を購読する。
- UI は Arc UI（shadcn/ui の CLI で導入する）で組む。`arc` スキルの規約（トークン、タイポ、コピー、レイアウト、モーション、アクセシビリティ）に従う。
- 自前でコンポーネントを作る前に、Arc の該当コンポーネントやブロック（フォーム、テーブル、ダイアログなど）を探して使う。
- 完了条件: UI だけで 3 モードの設定と実行ができる。

### 7. トレイと常駐
- `tray` を実装する。メニューは「開始/停止」「ウィンドウを表示」「終了」。状態でアイコンを切り替え、左クリックでウィンドウを表示する。
- × ボタンはトレイへの格納にし、Click Run は止めない。「終了」だけがアプリを終了して Click Run も止める。
- `tauri-plugin-single-instance` で多重起動を禁止し、既存のウィンドウを前面に出す。
- `tauri-plugin-autostart` による自動起動、およびトレイ常駐での起動の設定を、それぞれ別に実装する（初期値はどちらもオフ）。
- 完了条件: ウィンドウなしで起動し、トレイとホットキーだけで操作できる。

### 8. 仕上げ
- エラー表示の整理（開始前検査、ホットキー競合、権限なし、実行中の停止理由）。
- macOS と Linux (X11) でのビルドと手動確認。
- `README.md` を差し替え、Wayland 非対応と macOS の権限手順を書く。
- 不要な雛形（`greet` コマンド、`App.css` の初期内容）を削除する。

## 依存の追加
- Rust: `enigo`, `tauri-plugin-global-shortcut`, `tauri-plugin-autostart`, `tauri-plugin-single-instance`, `tauri-plugin-notification`, `thiserror`
- 既存の `serde` / `serde_json` を使う。

## リスク
- 長押しと短い Interval のタイミング精度は OS のスケジューラに左右される。1 ms は目標値で、保証はしない。
- macOS でのトレイ・権限まわりと Linux X11 は、手元に環境がないと検証できない。フェーズ 8 で実機確認の手段を決める。
- Wayland の検出方法（`XDG_SESSION_TYPE`）は誤検出があり得る。警告にとどめ、起動は止めない。
