# Boxy

## 项目简介
Boxy 是一套统一管理多种包管理器的工具集，提供 CLI、TUI 和 GUI 三种使用方式。目标是用一致的命令和界面管理不同生态的包，让日常安装、更新和查询更高效。

## 项目说明
- GUI 仅支持 macOS；CLI/TUI 为跨平台终端程序。
- 支持的包管理器：brew、mas、npm、pnpm、yarn、bun、pip、pipx、uv、cargo。
- 形态：
  - CLI：脚本化与自动化场景首选。
  - TUI：终端内交互式管理。
  - GUI：桌面应用（Tauri）。

### 平台支持细节
- macOS：
  - CLI/TUI：支持全部管理器（含 brew、mas）。
  - GUI：支持（macOS 原生打包）。
- Linux：
  - CLI/TUI：通常支持 npm/pnpm/yarn/bun/pip/pipx/uv/cargo。
  - brew/mas 不可用。
- Windows：
  - CLI/TUI：通常支持 npm/pnpm/yarn/bun/pip/pipx/uv/cargo。
  - brew/mas 不可用。
  - 依赖的包管理器需要已安装并在 PATH 中可用。

## 使用指南

### CLI
下载 CLI 后直接运行 `boxy`：

```bash
# 扫描可用包管理器
./boxy scan

# 列出已安装包
./boxy list --manager brew

# 搜索
./boxy search ripgrep --manager brew

# 安装
./boxy install ripgrep --manager brew

# 更新（可指定包名，不指定则更新全部）
./boxy update --manager brew

# 卸载
./boxy uninstall ripgrep --manager brew

# 列出可更新包
./boxy outdated --manager brew
```

范围与目录：

```bash
# 全局范围（npm/pnpm/yarn/bun）
./boxy list --manager npm --global

# 本地范围（指定目录）
./boxy list --manager npm --scope local --dir /path/to/project
```

JSON 输出：

```bash
./boxy scan --json
```

### TUI
运行 `boxy-tui` 进入终端界面：

```bash
./boxy-tui
```

常用按键：
- j/k 或 上/下：移动选择
- h/l 或 左/右：切换管理器
- Enter：查看详情
- /：搜索
- u：更新
- d：卸载
- r：刷新
- b 或 Esc：返回
- q 或 Ctrl+C：退出
- ?：帮助

### GUI
双击安装后的桌面应用即可使用。

## 下载指南
推荐下载方式：Homebrew（macOS）

```bash
# 订阅本仓库的 tap
brew tap ljiulong/boxyy https://github.com/ljiulong/boxyy

# 安装 CLI/TUI
brew install boxy

# 安装 GUI（macOS）
brew install --cask boxy-gui
```

项目在每次 push 后会自动构建并发布到 GitHub Releases 的 `nightly` 预发布版本：

1. 打开项目的 GitHub Releases 页面：<https://github.com/ljiulong/boxyy/releases>
2. 找到 `nightly` 版本。
3. 下载对应系统的文件：
   - CLI/TUI：
     - macOS：`boxy-cli-tui-v<版本>-macOS.tar.gz`
     - Linux：`boxy-cli-tui-v<版本>-Linux.tar.gz`
     - Windows：`boxy-cli-tui-v<版本>-Windows.zip`
   - GUI（仅 macOS）：
     - macOS：`.dmg`/`.app`
4. 解压后运行即可。

macOS GUI 安装提示：

1. 下载 `.dmg` 后双击打开，将应用拖到 “Applications”。
2. 首次打开如果被拦截，前往“系统设置 → 隐私与安全性”，在“已阻止打开的应用”处选择“仍要打开”。
3. 也可以在 Finder 中右键应用选择“打开”，按提示确认。

命令行下载（示例）：

使用 GitHub CLI（推荐）：

```bash
# 下载当前系统对应的 CLI/TUI 压缩包
gh release download nightly -R ljiulong/boxyy -p "boxy-cli-tui-*.tar.gz" -p "boxy-cli-tui-*.zip"

# 下载 GUI 安装包（按系统匹配）
gh release download nightly -R ljiulong/boxyy -p "*.dmg" -p "*.msi" -p "*.exe" -p "*.AppImage" -p "*.deb" -p "*.rpm"
```

