# ndi-share

[English README](README.en.md)

NDI 映像ソースを受信し、その映像を **Syphon Metal テクスチャ** として再配信する macOS 向け CLI ツールです。NDI 映像を Resolume・OBS などの Syphon 対応アプリへ、GPU 上のテクスチャとして橋渡しします。

## 仕組み

NDI はネットワーク経由で **CPU メモリ上の映像フレーム** を運び、Syphon は **GPU 上の Metal テクスチャ** をプロセス間で共有します。本ツールは受信フレームを BGRA で受け取り、IOSurface 経由で Metal テクスチャ化して `SyphonMetalServer` で公開します（色変換シェーダ不要）。

```
NDI 送信元 ──(ネットワーク)──▶ ndi-share ──(Syphon / Metal)──▶ Resolume / OBS / Syphon Recorder
```

## 前提条件

- **フル版の Xcode**（Command Line Tools だけでは不可）。`xcrun` が `MacOSX.sdk` を見つけられない場合:
  ```bash
  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
  ```
- **Metal Toolchain**（Syphon の Metal シェーダのコンパイルに必要）。framework のビルドが `cannot execute tool 'metal'` で失敗する場合、一度だけ実行:
  ```bash
  xcodebuild -downloadComponent MetalToolchain   # 約 688 MB のダウンロード
  ```
- NDI ランタイム: `brew install libndi`（`/usr/local/lib/libndi.dylib` を提供）。
- Rust（stable）。

## ビルド

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer   # 必要な場合のみ
xcodebuild -downloadComponent MetalToolchain                            # 'metal' ツールが無い場合のみ、一度だけ
./scripts/setup-syphon.sh   # vendor/Syphon.framework をビルド（初回のみ）
cargo build --release
```

`scripts/setup-syphon.sh` は git submodule（`vendor/syphon-src` → Syphon-Framework）を取得し、`xcodebuild` で `vendor/Syphon.framework` をビルドします。

## 使い方

```bash
ndi-share --list                          # 検出された NDI ソース一覧を表示して終了
ndi-share --source "STUDIO (Camera 1)"    # 名前（部分一致）でソースを選んで配信
ndi-share                                 # 一覧から番号で対話的に選択
ndi-share --source Cam --name "MyFeed"    # Syphon 公開名を指定（既定: ソース名）
```

### オプション

| オプション | 説明 |
|---|---|
| `--list` | 検出された NDI ソースを一覧表示して終了 |
| `--source <名前>` | NDI ソース名（大文字小文字を無視した部分一致）。未指定かつ非 `--list` 時は対話選択 |
| `--name <名前>` | Syphon の公開名（既定: 選択した NDI ソース名） |
| `--timeout <ms>` | 検出・キャプチャのタイムアウト（ミリ秒、既定 5000） |
| `--verbose` | 受信解像度などのログを表示 |

配信を受け取るには Syphon 対応アプリ（Resolume、Syphon Recorder、OBS の Syphon プラグインなど）を開いてください。停止は **Ctrl-C** です。

## Windows / Spout（実験的・未検証）

Windows では Spout 出力に対応します（`SharedTextureOutput` trait による抽象化で、macOS=Syphon / Windows=Spout を切り替え）。

> ⚠️ **注意:** Windows/Spout バックエンドは現状 **GitHub Actions（windows-latest）でのコンパイル検証のみ**で、実機での動作確認は未実施です。色順・上下反転・SpoutDX 初期化まわりは実機検証で調整が必要な可能性があります。

ビルド手順（Windows / PowerShell）:

```powershell
./scripts/fetch-spout2.ps1            # vendor/Spout2 へ Spout2 SDK を取得
# NDI SDK をインストール（Processing.NDI.Lib.x64.lib を提供）
#   インストール先が標準と異なる場合は環境変数 NDI_SDK_DIR を設定
cargo build --release
```

NDI のインポートライブラリは `%NDI_SDK_DIR%\Lib\x64\Processing.NDI.Lib.x64.lib`（既定 `C:\Program Files\NDI\NDI 6 SDK`）を参照します。実行時は NDI ランタイム DLL が PATH 上にある必要があります。受信は Spout 対応アプリ（Resolume、OBS の Spout プラグインなど）で行います。

## 対応範囲

- **macOS / Syphon** — 実機検証済み（v1）。
- **Windows / Spout** — コンパイル検証のみ（実機動作は未検証）。

## ライセンス / 第三者ソフトウェア

本ツールは Syphon Framework（BSD）を同梱ビルドし、NDI ランタイム（libndi、別途インストール）にリンクします。詳細は [THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES) を参照してください。NDI® は Vizrt NV の登録商標です。
