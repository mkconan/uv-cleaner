# UVCleaner

[![CI](https://github.com/<YOUR_USERNAME>/UVCleaner/actions/workflows/ci.yml/badge.svg)](https://github.com/<YOUR_USERNAME>/UVCleaner/actions/workflows/ci.yml)
[![Release](https://github.com/<YOUR_USERNAME>/UVCleaner/actions/workflows/release.yml/badge.svg)](https://github.com/<YOUR_USERNAME>/UVCleaner/releases/latest)

Python プロジェクトの古い `.venv` を一括削除するターミナル UI ツールです。  
`~/Development` 配下を再帰スキャンし、**30 日以上更新されていない仮想環境**を列挙。  
チェックを入れて `d` → `y` で即削除、ディスクをすっきり解放できます。

```
┌─ Scan root  [Tab: edit] ──────────────────────────────────────────────────┐
│ /Users/you/Development                                                     │
└────────────────────────────────────────────────────────────────────────────┘
┌─ Projects ─────────────────────────────────────────────────────────────────┐
│  [ ] /Users/you/Development/old-api  (2024-11-20)                          │
│      ██████████████░░░░░░  312 MB                                          │
│  [x] /Users/you/Development/legacy-bot  (2024-10-05)                       │
│      ███████████████████░  489 MB                                          │
└────────────────────────────────────────────────────────────────────────────┘
┌─ 1 selected  489 MB  │  d: delete  a: all  q: quit ──────────────────────┐
└────────────────────────────────────────────────────────────────────────────┘
```

## 機能

- `~/Development` 以下の Python プロジェクト（`pyproject.toml` 基準）を自動検出
- 最終更新日が **30 日以上前** の `.venv` のみ表示
- サイズバーで各 `.venv` のディスク使用量を視覚化
- スキャン対象ディレクトリをその場で変更可能（Tab キー + パス補完）
- 削除前にチェックボックスで個別 or 全選択

## インストール

### バイナリをダウンロード（推奨）

[Releases ページ](https://github.com/<YOUR_USERNAME>/UVCleaner/releases/latest) から OS に合ったアーカイブをダウンロードしてください。

| ファイル | 対象 |
|---|---|
| `UVCleaner-macos-arm64.tar.gz` | macOS (Apple Silicon) |
| `UVCleaner-macos-x86_64.tar.gz` | macOS (Intel) |
| `UVCleaner-linux-x86_64.tar.gz` | Linux x86_64 (静的バイナリ) |
| `UVCleaner-windows-x86_64.zip` | Windows x86_64 |

**macOS / Linux:**

```bash
# 例: Apple Silicon
curl -LO https://github.com/<YOUR_USERNAME>/UVCleaner/releases/latest/download/UVCleaner-macos-arm64.tar.gz
tar -xzf UVCleaner-macos-arm64.tar.gz
chmod +x UVCleaner
sudo mv UVCleaner /usr/local/bin/
```

**macOS Gatekeeper の警告が出る場合:**

```bash
xattr -d com.apple.quarantine /usr/local/bin/UVCleaner
```

**Windows:**

ZIP を展開し、`UVCleaner.exe` を任意のフォルダに配置してください。

### ソースからビルド

Rust 1.85 以上が必要です。

```bash
git clone https://github.com/<YOUR_USERNAME>/UVCleaner.git
cd UVCleaner
cargo build --release
# ./target/release/UVCleaner に生成されます
```

## 使い方

```bash
UVCleaner
```

起動すると `~/Development` 以下を自動スキャンします。

### キーバインド

**通常モード**

| キー | 動作 |
|---|---|
| `↑` / `↓` | カーソル移動 |
| `Space` | チェックボックス切り替え |
| `a` | 全選択 / 全解除 |
| `d` | 選択した `.venv` を削除（確認画面） |
| `y` | 削除を確定 |
| `n` | 削除をキャンセル |
| `Tab` | スキャンパス編集モードに移行 |
| `q` | 終了 |

**パス編集モード**

| キー | 動作 |
|---|---|
| `Enter` | 入力パスでスキャンを再実行 |
| `Tab` / `↑` / `↓` | ディレクトリ補完候補を選択 |
| `Esc` | 編集をキャンセル |
| `Backspace` | 1 文字削除 |

## 動作要件

- macOS 11 以降 / Ubuntu 20.04 以降 / Windows 10 以降
- 対応アーキテクチャ: x86_64, arm64 (Apple Silicon)
- ターミナルが必要（SSH 接続先でも動作します）

## ライセンス

MIT
