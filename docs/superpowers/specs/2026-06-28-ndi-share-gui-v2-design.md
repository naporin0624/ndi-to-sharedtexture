# ndi-share-gui v2 設計（スリム化 + フォント同梱 + トレイ常駐）

- 日付: 2026-06-28
- 対象: 既存 `ndi-share-gui`（最小ランチャー）の改修
- 前提: [v1 設計](2026-06-28-ndi-share-gui-design.md) の上に乗る。ブランチ `feature/gui-launcher`
- ステータス: 設計確定（実装計画前）

## 目的

v1 の最小ランチャーを、実運用しやすい常駐ユーティリティへ拡張する。3点:

1. **スリム化** — UI をさらに簡素に。Name 欄を廃止し、情報を 1 行に集約。
2. **フォント同梱** — 日本語 NDI ソース名が豆腐（□）になる問題を、LINE Seed JP を埋め込んで解消。
3. **トレイ常駐** — ウィンドウの×で終了せずトレイ（macOS=メニューバー / Windows=通知領域）に格納。再配信はバックグラウンドで継続。終了はトレイの Quit のみ。

## 非目標（YAGNI）

- macOS の Dock アイコン非表示（Accessory 化）。将来の追加候補。今回は Dock アイコンは出たままでよい。
- トレイメニューからの Start/Stop や Show/Hide 項目（メニューは「ステータス表示 + Quit」のみ）。
- フォントの太さ切替やテーマ。Regular 1 ウェイトのみ。
- 設定の永続化・自動再接続。
- cannelloni の凝った演出（ホバー時 2px トランスレートの neo-brutalism プレス、neon マーチングアント focus ring、アニメーション）。色・形・境界の静的な再現に留める。

## 0. スタイリング（cannelloni テーマ）

ローカルリポジトリ `naporin0624/Cannelloni` の配色・UI 設計（neo-brutalist / terminal-print、ダーク専用）を参考に、egui の `Visuals`/`Style` を起動時に設定する。要点: **角丸ゼロ・境界線は常時可視・影なし・単一のエレクトリックブルー強調・LINE Seed JP**（フォント同梱と一致）。

### カラートークン（sRGB）

| 役割 | 色 | egui への割当（目安） |
|---|---|---|
| canvas 背景 | `#121212` | `panel_fill`, `window_fill` |
| surface（沈み） | `#1C1C1C` | `extreme_bg_color`, `faint_bg_color`, inactive widget bg |
| muted（ホバー面） | `#262626` | hovered widget bg |
| emphasis（選択面） | `#2E2E2E` | active/open widget bg |
| 主テキスト | `#EEEFF2` | inactive/active `fg_stroke` |
| 副テキスト | `#C9CCD2` | noninteractive `fg_stroke` |
| 弱テキスト/無効 | `#9A9EA7` | disabled |
| 境界（標準） | `#696969` | inactive `bg_stroke`(1px) |
| 境界（弱） | `#424242` | noninteractive `bg_stroke`(1px) |
| 境界（強） | `#868686` | hovered `bg_stroke` |
| 強調（青） | `#3996FF` | `selection.bg_fill`/`stroke`, `hyperlink_color`, active 枠, ● Running |
| 強調ホバー | `#2B78E8` | （任意） |
| 危険（赤・文字） | `#FF2135` | エラー時の info テキスト |

### 形・余白

- **角丸 = 0**: 全 `WidgetVisuals` の角丸を 0 にする。
  - 注意: egui 0.35 では `Rounding` が **`CornerRadius`** に改名され、フィールドも `corner_radius`。実装時にレジストリソースで正確な型/フィールド名を確認すること（`~/.cargo/registry/src/*/egui-0.35.0/`）。
- **境界線は常時可視**: inactive ウィジェットにも 1px の境界（`#696969`）を出す（フラットなゴーストにしない）。
- **影なし**: `window_shadow` / `popup_shadow` を `NONE`。
- **余白**: 4px グリッド。`spacing.item_spacing ≈ (8, 8)`、ボタンは最小高さ ~24px を確保（`spacing.button_padding` 調整）。

### 全体の雰囲気

ダーク・フラット・高コントラスト・シャープ。記号（●▶■）と LINE Seed JP の文字で構成し、青の強調 1 色で状態を示す。

## 1. スリム化

UI を縦に最小化する。Name 欄（公開名の手入力）は廃止し、**公開名 = 選択中ソース名**で固定する。

レイアウト:

```
Source: [ select ▼ ]  ⟳
[ ▶ Start ]   ← 実行中は [ ■ Stop ]
● Running     ← 状態行（実行中のみ。idle 時は非表示か淡色）
info: 1234 frames · CamName · Syphon
```

- **Source 行**: ドロップダウン + 右にリフレッシュアイコンボタン（同一行）。ソース無し時は無効化。
- **Start/Stop**: ソース未選択時は無効。
- **info 行**: 1 行に集約する単一の文字列。状態に応じて切替:
  - 探索中: `info: searching…`
  - ソース無し: `info: no NDI sources`
  - 待機中: `info: ready`（または直近の `stopped after N frames`）
  - 実行中: `info: {frames} frames · {name} · {Syphon|Spout}`
  - エラー: `info: error: {message}`
- 既存の `name` / `name_edited` フィールドと Name 関連 UI を削除。`start()` は選択ソース名を公開名に使う。
- ウィンドウ既定サイズを縮小（約 360×170）。

## 2. フォント同梱（LINE Seed JP）

egui 既定の proportional フォント（Ubuntu-Light）は CJK を含まず、日本語ソース名が豆腐になる。LINE Seed JP（SIL OFL）を埋め込んで解消する。

### 取得（fetch スクリプト方式）

既存の vendoring 流儀（`scripts/setup-syphon.sh`, `scripts/fetch-spout2.ps1` が `vendor/` に取得 → gitignore）に揃える。

- `scripts/fetch-fonts.sh` を新設。公式 `line/seed` の GitHub Release（固定タグ **`v20251119`**）の zip を取得し、Regular ウェイトのフォント（`*Rg*` の ttf/otf）と LICENSE を `vendor/fonts/` へ展開。
  - URL: `https://github.com/line/seed/releases/download/v20251119/seed-v20251119.zip`
  - 出力を固定名 `vendor/fonts/LINESeedJP-Regular.ttf` に正規化（中身が OTF/CFF でも egui の `ab_glyph`/`ttf-parser` は内容で判定するため拡張子は問わない）。
  - LICENSE を `vendor/fonts/LICENSE` に保存。
- `.gitignore` に `vendor/fonts/` を追加（3.5MB のバイナリを git に入れない）。

### 埋め込みと適用

- `src/bin/gui.rs` で `include_bytes!("../../vendor/fonts/LINESeedJP-Regular.ttf")`。
  - したがって `cargo build --features gui` の前に `scripts/fetch-fonts.sh` の実行が必須（Syphon framework と同じ前提）。未取得ならコンパイルエラーで気付ける。
- 起動時（`GuiApp::new` / 生成クロージャ内）に egui の `FontDefinitions` を構築:
  - LINE Seed を font データとして登録し、`Proportional` ファミリの**先頭**に挿入（日本語・ラテンを LINE Seed で描画）。
  - egui 既定フォント（絵文字フォント含む）は**フォールバックとして残す**（🔄・● などの記号を維持）。
  - `ctx.set_fonts(fonts)` で適用。
- `THIRD-PARTY-NOTICES` に LINE Seed（OFL）を追記。`vendor/fonts/LICENSE` を参照。
- README（日本語・英語）に「GUI ビルド前に `scripts/fetch-fonts.sh` を実行」を追記。

## 3. トレイ常駐 + close で最小化

`tray-icon`（v0.24, cross-platform）でトレイ/メニューバーにアイコンを置く。`eframe`/`tray-icon` は `gui` フィーチャに同梱。

### 依存とフィーチャ

```toml
[features]
gui = ["dep:eframe", "dep:tray-icon"]

[dependencies]
tray-icon = { version = "0.24", optional = true }
```

### 挙動

- **close で最小化**: 毎フレーム `ctx.input(|i| i.viewport().close_requested())` を確認し、true なら:
  - `ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose)`（終了を取り消し）
  - `ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false))`（ウィンドウを隠す）
- **トレイから復帰**: トレイアイコンのクリック（`TrayIconEvent`）、およびメニューのステータス項目クリックで:
  - `ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true))` + `Focus`
- **トレイメニュー**: 「ステータス表示（クリックで Show）」 + セパレータ + 「Quit」。
  - ステータス項目ラベルは現状を反映（例: `Running — CamName — 1234 frames` / `Idle`）。毎更新で文字列を更新。
  - **Quit**: 実行中ワーカーへ停止要求 → join → `ctx.send_viewport_cmd(ViewportCommand::Close)` で本当に終了。
- **常駐（バックグラウンド継続）**: 再配信ワーカーはウィンドウ表示状態と独立したスレッド（v1 の `RunState` 設計のまま）。ウィンドウを隠しても再配信は継続する。隠れている間も `request_repaint_after` のティックは不要だが、トレイのメニュー/アイコン操作は OS イベントループを起こし `ui` が呼ばれるため Quit/Show を拾える。

### イベント受信

- `tray-icon` の `TrayIconEvent::receiver()` と `MenuEvent::receiver()`（`muda`）はグローバルな `mpsc` 風レシーバ。`fn ui` の冒頭で `try_recv` を回し、保留中のトレイ/メニューイベントを処理する。
- トレイアイコンは生成クロージャ（main スレッド、イベントループ初期化後）で構築し、`GuiApp` が `TrayIcon` を保持して生存させる。
- トレイアイコン画像はコード内で生成（小さな RGBA バッファ。外部画像アセットは持たない）。

### プラットフォーム注記

- macOS: メニューバーにアイコンが出る。Dock アイコンは出たまま（非表示化は非目標）。
- Windows: 通知領域にアイコン。`tray-icon` が吸収。
- トレイの実挙動（×→格納、Quit→終了、復帰）は**人手による目視確認**（このリポジトリの CI は GUI をビルドしないため、コンパイルが自動ゲート）。

## クレート/ファイル構成

- `Cargo.toml` — `tray-icon` を `gui` フィーチャに追加。
- `scripts/fetch-fonts.sh` — 新規。フォント取得。
- `.gitignore` — `vendor/fonts/` 追加。
- `src/bin/gui.rs` — スリム化（Name 削除・info 行）、フォント適用、トレイ統合、close 横取り。
- `THIRD-PARTY-NOTICES` — LINE Seed (OFL) 追記。
- `README.md` / `README.en.md` — フォント取得手順とトレイ挙動を追記。

コア（lib 側 `ndi`/`output`/`run`）は変更しない。すべて GUI バイナリ内の変更。

## テスト方針

- 自動ゲート: `cargo build --features gui --bin ndi-share-gui`（クリーン）、`cargo build`（無フィーチャ、tray-icon/eframe 非依存を維持）、`cargo test`（18/18 不変）。
- フォント未取得時はコンパイルエラーになる（include_bytes）。CI は GUI を建てないため影響なし。
- 人手確認（macOS、ディスプレイ + ライブ NDI ソース）:
  - 日本語ソース名が豆腐にならない。
  - ×でトレイ格納、再配信が継続、トレイから復帰、Quit で終了。
