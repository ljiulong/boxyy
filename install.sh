#!/usr/bin/env bash
# Boxy 一键安装脚本
# 用法: curl -fsSL https://raw.githubusercontent.com/ljiulong/boxyy/main/install.sh | bash

set -e  # 遇到错误立即退出

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 打印带颜色的信息
info() { echo -e "${BLUE}ℹ${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1" >&2; exit 1; }
warning() { echo -e "${YELLOW}⚠${NC} $1"; }

# 检测操作系统
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS="Linux";;
        Darwin*)    OS="macOS";;
        MINGW*|MSYS*|CYGWIN*) OS="Windows";;
        *)          error "不支持的操作系统: $(uname -s)";;
    esac
}

# 检测架构
detect_arch() {
    local machine_arch="$(uname -m)"

    case "$machine_arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64|armv7l|armv6l)
            error "当前不支持 $machine_arch 架构。
目前仅提供 x86_64/amd64 架构的预编译二进制文件。

如需其他架构支持，请：
  1. 访问 https://github.com/ljiulong/boxyy/issues 提交需求
  2. 或从源码编译: cargo build --release -p boxy-cli -p boxy-tui

支持的架构：x86_64, amd64"
            ;;
        *)
            error "不支持的架构: $machine_arch

当前仅支持 x86_64/amd64 架构。
请访问 https://github.com/ljiulong/boxyy/issues 反馈您的需求。"
            ;;
    esac
}

# 获取最新版本号
get_latest_version() {
    info "获取最新版本..."

    # 优先使用用户指定的版本
    if [ -n "$BOXY_VERSION" ]; then
        VERSION="$BOXY_VERSION"
        info "使用指定版本: $VERSION"
        return
    fi

    # 从 GitHub API 获取最新 release
    if command -v curl &> /dev/null; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/ljiulong/boxyy/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
    elif command -v wget &> /dev/null; then
        VERSION=$(wget -qO- "https://api.github.com/repos/ljiulong/boxyy/releases/latest" 2>/dev/null | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
    fi

    if [ -z "$VERSION" ]; then
        error "无法获取最新版本号。请检查网络连接或手动指定版本: export BOXY_VERSION=v1.0.0"
    fi

    success "最新版本: $VERSION"
}

# 构建下载 URL
build_download_url() {
    if [ "$OS" = "Windows" ]; then
        FILENAME="boxy-cli-tui-${VERSION}-${OS}.zip"
    else
        FILENAME="boxy-cli-tui-${VERSION}-${OS}.tar.gz"
    fi

    DOWNLOAD_URL="https://github.com/ljiulong/boxyy/releases/download/${VERSION}/${FILENAME}"
    info "下载地址: $DOWNLOAD_URL"
}

# 下载文件
download_file() {
    TMPDIR=$(mktemp -d)
    info "下载 ${FILENAME}..."

    if command -v curl &> /dev/null; then
        curl -fL --progress-bar "$DOWNLOAD_URL" -o "${TMPDIR}/${FILENAME}" || error "下载失败。请检查网络连接或版本号是否正确"
    elif command -v wget &> /dev/null; then
        wget --show-progress -q "$DOWNLOAD_URL" -O "${TMPDIR}/${FILENAME}" || error "下载失败。请检查网络连接或版本号是否正确"
    else
        error "未找到 curl 或 wget。请先安装其中之一：\n  Ubuntu/Debian: sudo apt-get install curl\n  CentOS/RHEL: sudo yum install curl"
    fi

    success "下载完成"
}

# 解压文件
extract_file() {
    info "解压文件..."

    cd "$TMPDIR"

    if [ "$OS" = "Windows" ]; then
        if command -v unzip &> /dev/null; then
            unzip -q "$FILENAME" || error "解压失败"
        else
            error "未找到 unzip。请先安装: sudo apt-get install unzip"
        fi
    else
        tar -xzf "$FILENAME" || error "解压失败"
    fi

    success "解压完成"
}

# 安装到 PATH
install_binaries() {
    # 优先使用用户指定的安装目录
    if [ -n "$BOXY_INSTALL_DIR" ]; then
        INSTALL_DIR="$BOXY_INSTALL_DIR"
    else
        INSTALL_DIR="$HOME/.local/bin"
    fi

    # 确保目录存在
    mkdir -p "$INSTALL_DIR" || error "无法创建目录 $INSTALL_DIR"

    info "安装到 $INSTALL_DIR..."

    # 查找并移动二进制文件
    if [ "$OS" = "Windows" ]; then
        if [ -f "boxy-cli-${OS}.exe" ]; then
            mv "boxy-cli-${OS}.exe" "$INSTALL_DIR/boxy.exe" || error "安装失败"
        fi
        if [ -f "boxy-tui-${OS}.exe" ]; then
            mv "boxy-tui-${OS}.exe" "$INSTALL_DIR/boxy-tui.exe" || error "安装失败"
        fi
    else
        if [ -f "boxy-cli-${OS}" ]; then
            mv "boxy-cli-${OS}" "$INSTALL_DIR/boxy" || error "安装失败"
            chmod +x "$INSTALL_DIR/boxy"
        else
            error "未找到可执行文件 boxy-cli-${OS}"
        fi

        if [ -f "boxy-tui-${OS}" ]; then
            mv "boxy-tui-${OS}" "$INSTALL_DIR/boxy-tui" || error "安装失败"
            chmod +x "$INSTALL_DIR/boxy-tui"
        else
            error "未找到可执行文件 boxy-tui-${OS}"
        fi
    fi

    # 清理临时文件
    cd "$HOME"
    rm -rf "$TMPDIR"

    success "安装完成到 $INSTALL_DIR"
}

# 配置 PATH
configure_path() {
    if [ -n "$BOXY_INSTALL_DIR" ]; then
        INSTALL_DIR="$BOXY_INSTALL_DIR"
    else
        INSTALL_DIR="$HOME/.local/bin"
    fi

    # 检查是否已在 PATH 中
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        return
    fi

    warning "$INSTALL_DIR 不在 PATH 中"

    # 跳过 PATH 配置（CI 环境或用户不想自动配置）
    if [ -n "$BOXY_SKIP_PATH_CONFIG" ]; then
        info "跳过 PATH 配置（BOXY_SKIP_PATH_CONFIG 已设置）"
        echo ""
        info "请手动将以下内容添加到 shell 配置文件："
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        return
    fi

    # 检测 shell
    if [ -n "$SHELL" ]; then
        SHELL_NAME=$(basename "$SHELL")
    else
        SHELL_NAME="bash"
    fi

    case "$SHELL_NAME" in
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                SHELL_RC="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                SHELL_RC="$HOME/.bash_profile"
            else
                SHELL_RC="$HOME/.bashrc"
                touch "$SHELL_RC"
            fi
            ;;
        zsh)
            SHELL_RC="$HOME/.zshrc"
            [ ! -f "$SHELL_RC" ] && touch "$SHELL_RC"
            ;;
        fish)
            SHELL_RC="$HOME/.config/fish/config.fish"
            mkdir -p "$(dirname "$SHELL_RC")"
            [ ! -f "$SHELL_RC" ] && touch "$SHELL_RC"
            if ! grep -q "set -gx PATH $INSTALL_DIR" "$SHELL_RC"; then
                echo "" >> "$SHELL_RC"
                echo "# Boxy CLI/TUI" >> "$SHELL_RC"
                echo "set -gx PATH $INSTALL_DIR \$PATH" >> "$SHELL_RC"
                success "已添加到 $SHELL_RC"
            fi
            info "请运行: source $SHELL_RC"
            return
            ;;
        *)
            warning "无法识别的 shell: $SHELL_NAME"
            echo ""
            info "请手动将以下内容添加到 shell 配置文件："
            echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
            return
            ;;
    esac

    # 添加到 shell 配置（避免重复添加）
    if ! grep -q "# Boxy CLI/TUI" "$SHELL_RC" 2>/dev/null; then
        echo "" >> "$SHELL_RC"
        echo "# Boxy CLI/TUI" >> "$SHELL_RC"
        echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$SHELL_RC"
        success "已添加到 $SHELL_RC"
        info "请运行: source $SHELL_RC"
    else
        info "PATH 配置已存在于 $SHELL_RC"
    fi
}

# 验证安装
verify_installation() {
    info "验证安装..."

    if [ -n "$BOXY_INSTALL_DIR" ]; then
        INSTALL_DIR="$BOXY_INSTALL_DIR"
    else
        INSTALL_DIR="$HOME/.local/bin"
    fi

    if [ "$OS" = "Windows" ]; then
        # Windows: 检查文件存在性和可执行性
        if [ ! -f "$INSTALL_DIR/boxy.exe" ]; then
            error "验证失败：未找到 boxy.exe"
        fi
        if [ ! -f "$INSTALL_DIR/boxy-tui.exe" ]; then
            error "验证失败：未找到 boxy-tui.exe"
        fi

        # 尝试执行验证
        if ! "$INSTALL_DIR/boxy.exe" --version &> /dev/null; then
            error "验证失败：boxy.exe 无法执行或已损坏"
        fi
        if ! "$INSTALL_DIR/boxy-tui.exe" --version &> /dev/null; then
            error "验证失败：boxy-tui.exe 无法执行或已损坏"
        fi

        success "验证成功！"
        return 0
    else
        # Linux/macOS: 检查文件存在性和可执行性
        if [ ! -f "$INSTALL_DIR/boxy" ]; then
            error "验证失败：未找到 boxy 可执行文件"
        fi
        if [ ! -x "$INSTALL_DIR/boxy" ]; then
            error "验证失败：boxy 没有可执行权限"
        fi
        if [ ! -f "$INSTALL_DIR/boxy-tui" ]; then
            error "验证失败：未找到 boxy-tui 可执行文件"
        fi
        if [ ! -x "$INSTALL_DIR/boxy-tui" ]; then
            error "验证失败：boxy-tui 没有可执行权限"
        fi

        # 临时添加到 PATH 进行验证
        export PATH="$INSTALL_DIR:$PATH"

        # 验证 boxy 可以正常执行
        if ! boxy --version &> /dev/null; then
            error "验证失败：boxy 无法执行（可能是架构不兼容或二进制文件损坏）"
        fi

        # 验证 boxy-tui 可以正常执行
        if ! boxy-tui --version &> /dev/null; then
            error "验证失败：boxy-tui 无法执行（可能是架构不兼容或二进制文件损坏）"
        fi

        # 获取版本信息
        BOXY_VERSION_OUTPUT=$(boxy --version 2>&1 | head -n 1)

        success "验证成功！"
        info "已安装: $BOXY_VERSION_OUTPUT"
        return 0
    fi
}

# 显示使用说明
show_usage() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    success "Boxy 安装完成！"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    info "快速开始："
    echo ""
    echo "  boxy scan          # 扫描所有包管理器"
    echo "  boxy-tui           # 启动终端界面（推荐）"
    echo "  boxy list          # 列出已安装的包"
    echo "  boxy --help        # 查看帮助"
    echo ""
    info "文档: https://github.com/ljiulong/boxyy"
    info "反馈: https://github.com/ljiulong/boxyy/issues"
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# 主函数
main() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}     Boxy 一键安装脚本${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    detect_os
    detect_arch
    info "检测到系统: $OS ($ARCH)"

    # macOS 用户提示使用 Homebrew
    if [ "$OS" = "macOS" ] && [ -z "$BOXY_FORCE_INSTALL" ]; then
        echo ""
        warning "检测到 macOS 系统"
        info "推荐使用 Homebrew 安装（更简单，自动更新）："
        echo ""
        echo -e "  ${GREEN}brew tap ljiulong/boxyy https://github.com/ljiulong/boxyy${NC}"
        echo -e "  ${GREEN}brew install boxy${NC}"
        echo ""
        echo "如果仍要使用此脚本安装，请设置环境变量："
        echo "  export BOXY_FORCE_INSTALL=1"
        echo "  curl -fsSL https://raw.githubusercontent.com/ljiulong/boxyy/main/install.sh | bash"
        echo ""
        exit 0
    fi

    echo ""
    get_latest_version
    build_download_url
    echo ""
    download_file
    extract_file
    install_binaries
    echo ""
    configure_path
    echo ""
    verify_installation

    show_usage
}

# 错误处理
trap 'error "安装过程中发生错误。请查看上方错误信息"' ERR

# 运行主函数
main "$@"
