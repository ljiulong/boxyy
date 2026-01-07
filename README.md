# Boxy

# 请注意：Boxy不是包管理器，只是包管理器的统一入口。

## 目录
- [项目简介](#项目简介)
- [项目说明](#项目说明)
- [平台支持细节](#平台支持细节)
- [使用指南](#使用指南)
- [CLI](#cli)
- [TUI](#tui)
- [GUI](#gui)
- [下载指南](#下载指南)
- [macOS CLI/TUI 运行提示](#macos-clitui-运行提示)
- [macOS GUI 安装提示](#macos-gui-安装提示)
- [常见问题](#常见问题)
- [许可证](#许可证)

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
推荐使用 TUI 作为日常入口（启动快、功能全、无需离开终端）。

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
macOS 推荐下载方式：Homebrew（避免系统拦截，安装后可直接使用）
 
说明：
- Homebrew 仅适用于 macOS；Linux/Windows 请使用 GitHub Releases 下载对应包。

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

## 常见问题

### macOS 提示“无法打开”或“来自不明开发者”
可执行以下命令移除隔离属性（不需要重装）：

```bash
sudo xattr -rd com.apple.quarantine /Applications/Boxy.app
```

### macOS CLI/TUI 可执行文件被拦截
如果直接从 Releases 下载 CLI/TUI 压缩包并解压，可能同样被标记为“来自互联网”。可对解压目录执行：

```bash
sudo xattr -rd com.apple.quarantine /path/to/extracted
```

### Windows 提示已保护或 SmartScreen 拦截
在提示窗口点击“更多信息” -> “仍要运行”。如果是压缩包解压后的可执行文件，可在 PowerShell 中执行：

```powershell
Unblock-File -Path "C:\path\to\boxy-cli-windows.exe"
```

### Linux 提示权限不足或无法执行
解压后需要确保可执行权限：

```bash
chmod +x /path/to/boxy-cli-Linux
```

macOS CLI/TUI 运行提示：
- 从 GitHub Releases 直接下载的 CLI/TUI 可执行文件会被 Gatekeeper 标记为“来自互联网”，首次运行可能需要手动允许。
- 若希望“下载后直接可用”，请使用上面的 Homebrew 安装方式。
- 若仍选择手动下载，可将可执行文件放入 PATH（无需 sudo）：

```bash
mkdir -p ~/.local/bin
mv ~/Downloads/boxy-cli-macOS ~/.local/bin/boxy
chmod +x ~/.local/bin/boxy
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

macOS GUI 安装提示：

1. 下载 `.dmg` 后双击打开，将应用拖到 “Applications”。
2. 首次打开如果被拦截，前往“系统设置 → 隐私与安全性”，在“已阻止打开的应用”处选择“仍要打开”。
3. 也可以在 Finder 中右键应用选择“打开”，按提示确认。


## 
MIT License，详见 `LICENSE`。

# 请注意：Boxy不是包管理器，只是包管理器的统一入口。
