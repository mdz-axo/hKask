#!/bin/bash
# hKask Installation Script for Linux
#
# This script installs hKask and its dependencies on Linux systems.
# Supports: Debian/Ubuntu, Fedora/RHEL, Arch Linux, openSUSE, Alpine
#
# Usage: curl -fsSL https://raw.githubusercontent.com/mdz-axo/hKask/main/scripts/install.sh | bash
# Or: wget -O - https://raw.githubusercontent.com/mdz-axo/hKask/main/scripts/install.sh | bash

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================

HKASK_VERSION="${HKASK_VERSION:-0.21.1}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local}"
BIN_DIR="${INSTALL_DIR}/bin"
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ============================================================================
# Logging Functions
# ============================================================================

log() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# ============================================================================
# System Detection
# ============================================================================

detect_package_manager() {
    if command -v apt-get &> /dev/null; then
        echo "apt"
    elif command -v dnf &> /dev/null; then
        echo "dnf"
    elif command -v yum &> /dev/null; then
        echo "yum"
    elif command -v pacman &> /dev/null; then
        echo "pacman"
    elif command -v zypper &> /dev/null; then
        echo "zypper"
    elif command -v apk &> /dev/null; then
        echo "apk"
    else
        echo "unknown"
    fi
}

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "$ID"
    else
        echo "unknown"
    fi
}

# ============================================================================
# Dependency Installation
# ============================================================================

install_system_dependencies() {
    local pkg_mgr=$(detect_package_manager)
    local os=$(detect_os)

    log "Detected package manager: $pkg_mgr"
    log "Detected OS: $os"

    case "$pkg_mgr" in
        apt)
            log "Updating package lists..."
            sudo apt-get update -qq

            log "Installing build dependencies (Debian/Ubuntu)..."
            sudo apt-get install -y -qq \
                build-essential \
                pkg-config \
                libssl-dev \
                libsqlite3-dev \
                libclang-dev \
                llvm-dev \
                liblldb-dev \
                libzstd-dev \
                cmake \
                git \
                curl \
                wget \
                jq \
                protobuf-compiler \
                libprotobuf-dev
            ;;
        dnf|yum)
            log "Installing build dependencies (Fedora/RHEL)..."
            sudo $pkg_mgr install -y \
                gcc \
                gcc-c++ \
                make \
                pkg-config \
                openssl-devel \
                sqlite-devel \
                clang-devel \
                llvm-devel \
                cmake \
                git \
                curl \
                wget \
                jq \
                protobuf-compiler \
                protobuf-devel
            ;;
        pacman)
            log "Installing build dependencies (Arch Linux)..."
            sudo pacman -Sy --noconfirm \
                base-devel \
                openssl \
                sqlite \
                clang \
                llvm \
                cmake \
                git \
                curl \
                wget \
                jq \
                protobuf \
                protobuf-c
            ;;
        zypper)
            log "Installing build dependencies (openSUSE)..."
            sudo zypper install -y \
                -t pattern devel_basis \
                pkg-config \
                libopenssl-devel \
                sqlite3-devel \
                clang \
                llvm \
                cmake \
                git \
                curl \
                wget \
                jq \
                protobuf-devel
            ;;
        apk)
            log "Installing build dependencies (Alpine)..."
            sudo apk add --no-cache \
                build-base \
                openssl-dev \
                sqlite-dev \
                clang \
                llvm \
                cmake \
                git \
                curl \
                wget \
                jq \
                protobuf-dev
            ;;
        unknown)
            log_warning "Unknown package manager. Please install dependencies manually."
            log "Required: build-essential, pkg-config, libssl-dev, libsqlite3-dev, libclang-dev, llvm-dev, cmake, git, curl, jq"
            return 1
            ;;
    esac

    log_success "System dependencies installed"
}

install_rust() {
    if command -v rustc &> /dev/null; then
        local rust_version=$(rustc --version)
        log "Rust already installed: $rust_version"

        if ! rustc --version | grep -qE 'rustc 1\.(8[5-9]|9[0-9]|[1-9][0-9]{2,})\.'; then
            log_warning "Rust version too old for edition 2024 (requires 1.85+). Consider updating with 'rustup update'"
        fi
    else
        log "Installing Rust toolchain..."

        if [ "${CI:-}" != "true" ]; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.91

            if [ -f "$HOME/.cargo/env" ]; then
                source "$HOME/.cargo/env"
            fi
        else
            log "Running in CI environment, skipping Rust installation"
        fi
    fi

    log "Adding Rust components..."
    rustup component add rustfmt clippy rust-src 2>/dev/null || true

    log_success "Rust toolchain ready"
}

# ============================================================================
# Build and Install
# ============================================================================

HKASK_REPO_URL="${HKASK_REPO_URL:-https://github.com/mdz-axo/hKask.git}"
HKASK_SOURCE_DIR="${HKASK_SOURCE_DIR:-}"

clone_repo() {
    if [ -n "$HKASK_SOURCE_DIR" ]; then
        log "Using existing source directory: $HKASK_SOURCE_DIR"
        return 0
    fi

    if [ -f "Cargo.toml" ] && grep -q "hkask-cli" Cargo.toml 2>/dev/null; then
        HKASK_SOURCE_DIR="$(pwd)"
        log "Running from within hKask repo: $HKASK_SOURCE_DIR"
        return 0
    fi

    if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "$(dirname "${BASH_SOURCE[0]}")/../Cargo.toml" ]; then
        HKASK_SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
        log "Detected repo from script location: $HKASK_SOURCE_DIR"
        return 0
    fi

    local clone_dir="${XDG_CACHE_HOME:-$HOME/.cache}/hkask-build"
    log "Cloning hKask repository..."
    rm -rf "$clone_dir"
    git clone --depth 1 --branch "v${HKASK_VERSION}" "$HKASK_REPO_URL" "$clone_dir" 2>/dev/null || \
        git clone --depth 1 "$HKASK_REPO_URL" "$clone_dir"
    HKASK_SOURCE_DIR="$clone_dir"
    log_success "Repository cloned to $HKASK_SOURCE_DIR"
}

build_hkask() {
    clone_repo
    local workspace_root="$HKASK_SOURCE_DIR"

    log "Building hKask in $workspace_root..."
    cd "$workspace_root"

    if [ "${HKASK_BUILD_TYPE:-release}" = "release" ]; then
        log "Building in release mode..."
        cargo build --release --package hkask-cli
    else
        log "Building in debug mode..."
        cargo build --package hkask-cli
    fi

    log_success "Build complete"
}

install_binary() {
    local workspace_root="$HKASK_SOURCE_DIR"

    log "Installing hKask binary..."

    # Create bin directory if it doesn't exist
    mkdir -p "$BIN_DIR"

    # Copy binary
    if [ "${HKASK_BUILD_TYPE:-release}" = "release" ]; then
        cp "$workspace_root/target/release/kask" "$BIN_DIR/kask"
    else
        cp "$workspace_root/target/debug/kask" "$BIN_DIR/kask"
    fi

    chmod +x "$BIN_DIR/kask"

    log_success "Binary installed to $BIN_DIR/kask"
}

setup_environment() {
    log "Setting up environment..."

    # Add to PATH if not already present
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        log "Adding $BIN_DIR to PATH..."

        export PATH="$BIN_DIR:$PATH"

        local shell_config=""
        if [ -n "${ZSH_VERSION:-}" ]; then
            shell_config="$HOME/.zshrc"
        elif [ -n "${BASH_VERSION:-}" ]; then
            shell_config="$HOME/.bashrc"
        fi

        if [ -n "$shell_config" ]; then
            if ! grep -q "$BIN_DIR" "$shell_config" 2>/dev/null; then
                echo "" >> "$shell_config"
                echo "# hKask" >> "$shell_config"
                echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$shell_config"
                log "Added PATH to $shell_config. Please restart your shell or run: source $shell_config"
            fi
        else
            log_warning "Could not detect shell config. Please add $BIN_DIR to your PATH manually."
        fi
    fi

    # Create config directory
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/hkask"
    if [ ! -d "$config_dir" ]; then
        mkdir -p "$config_dir"
        log "Created config directory: $config_dir"
    fi

    # Create data directory
    local data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/hkask"
    if [ ! -d "$data_dir" ]; then
        mkdir -p "$data_dir"
        log "Created data directory: $data_dir"
    fi

    log_success "Environment configured"
}

# ============================================================================
# Verification
# ============================================================================

verify_installation() {
    log "Verifying installation..."

    if command -v kask &> /dev/null; then
        local version=$(kask --version 2>&1 || echo "unknown")
        log_success "hKask installed successfully: $version"
    else
        log_warning "kask command not found in PATH"
        log "Try: export PATH=\"$BIN_DIR:\$PATH\""
    fi
}

# ============================================================================
# Uninstall
# ============================================================================

uninstall_hkask() {
    log "Uninstalling hKask..."

    # Remove binary
    if [ -f "$BIN_DIR/kask" ]; then
        rm -f "$BIN_DIR/kask"
        log "Removed $BIN_DIR/kask"
    fi

    # Remove config (optional)
    if [ "${HKASK_REMOVE_CONFIG:-false}" = "true" ]; then
        local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/hkask"
        rm -rf "$config_dir"
        log "Removed config directory: $config_dir"

        local data_dir="${XDG_DATA_HOME:-$HOME/.local/share}/hkask"
        rm -rf "$data_dir"
        log "Removed data directory: $data_dir"
    fi

    log_success "hKask uninstalled"
}

# ============================================================================
# Help
# ============================================================================

show_help() {
    cat << EOF
hKask Installation Script

Usage: $0 [OPTIONS]

Options:
    --install           Install hKask (default)
    --uninstall         Remove hKask
    --build-only        Build without installing
    --debug             Build in debug mode
    --skip-deps         Skip system dependency installation
    --skip-rust         Skip Rust installation
    --install-dir DIR   Install to custom directory (default: \$HOME/.local)
    --help              Show this help message

Environment Variables:
    HKASK_VERSION       Version to install (default: 0.21.1)
    HKASK_BUILD_TYPE    Build type: release or debug (default: release)
    INSTALL_DIR         Installation directory (default: \$HOME/.local)
    CARGO_HOME          Cargo installation directory (default: \$HOME/.cargo)
    HKASK_REMOVE_CONFIG Remove config and data on uninstall (default: false)
    HKASK_SOURCE_DIR    Use existing source directory instead of cloning
    HKASK_REPO_URL      Git repository URL (default: https://github.com/mdz-axo/hKask.git)

Examples:
    # Install hKask
    curl -fsSL https://raw.githubusercontent.com/mdz-axo/hKask/main/scripts/install.sh | bash

    # Install with custom directory
    INSTALL_DIR=/opt/hkask bash install.sh

    # Debug build
    HKASK_BUILD_TYPE=debug bash install.sh

    # Uninstall
    bash install.sh --uninstall

    # Uninstall with config
    HKASK_REMOVE_CONFIG=true bash install.sh --uninstall

EOF
}

# ============================================================================
# Main
# ============================================================================

main() {
    local action="install"
    local skip_deps=false
    local skip_rust=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --install)
                action="install"
                shift
                ;;
            --uninstall)
                action="uninstall"
                shift
                ;;
            --build-only)
                action="build-only"
                shift
                ;;
            --debug)
                HKASK_BUILD_TYPE="debug"
                shift
                ;;
            --skip-deps)
                skip_deps=true
                shift
                ;;
            --skip-rust)
                skip_rust=true
                shift
                ;;
            --install-dir)
                INSTALL_DIR="$2"
                BIN_DIR="${INSTALL_DIR}/bin"
                shift 2
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    echo ""
    echo "╔══════════════════════════════════════════════════════════╗"
    echo "║                    hKask Installer                      ║"
    echo "║          ℏKask — Planck's Constant of Agents           ║"
    echo "╚══════════════════════════════════════════════════════════╝"
    echo ""

    case "$action" in
        install)
            log "Starting hKask installation..."

            if [ "$skip_deps" = false ]; then
                install_system_dependencies
            else
                log "Skipping system dependency installation"
            fi

            if [ "$skip_rust" = false ]; then
                install_rust
            else
                log "Skipping Rust installation"
            fi

            build_hkask
            install_binary
            setup_environment
            verify_installation

            echo ""
            log_success "Installation complete!"
            echo ""
            echo "To get started:"
            echo "  1. Add hKask to your PATH (if not already done):"
            echo "     export PATH=\"$BIN_DIR:\$PATH\""
            echo ""
            echo "  2. Run hKask:"
            echo "     kask --help"
            echo ""
            echo "  3. Start interactive chat:"
            echo "     kask chat"
            echo ""
            ;;
        uninstall)
            uninstall_hkask
            ;;
        build-only)
            if [ "$skip_deps" = false ]; then
                install_system_dependencies
            fi
            if [ "$skip_rust" = false ]; then
                install_rust
            fi
            build_hkask
            ;;
    esac
}

# Run main function
main "$@"
