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

HKASK_VERSION="${HKASK_VERSION:-0.27.0}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local}"
BIN_DIR="${INSTALL_DIR}/bin"
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
SYSTEM_BIN="/usr/local/bin"

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
                libdbus-1-dev \
                git \
                curl \
                wget \
                jq \
                xz-utils
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
                dbus-devel \
                git \
                curl \
                wget \
                jq \
                xz
            ;;
        pacman)
            log "Installing build dependencies (Arch Linux)..."
            sudo pacman -Sy --noconfirm \
                base-devel \
                openssl \
                sqlite \
                dbus \
                git \
                curl \
                wget \
                jq \
                xz
            ;;
        zypper)
            log "Installing build dependencies (openSUSE)..."
            sudo zypper install -y \
                -t pattern devel_basis \
                pkg-config \
                libopenssl-devel \
                sqlite3-devel \
                dbus-1-devel \
                git \
                curl \
                wget \
                jq \
                xz
            ;;
        apk)
            log "Installing build dependencies (Alpine)..."
            sudo apk add --no-cache \
                build-base \
                openssl-dev \
                sqlite-dev \
                dbus-dev \
                git \
                curl \
                wget \
                jq \
                xz
            ;;
        unknown)
            log_warning "Unknown package manager. Please install dependencies manually."
            log "Required: build-essential, pkg-config, libssl-dev, libsqlite3-dev, libdbus-1-dev, git, curl, jq, xz-utils"
            return 1
            ;;
    esac

    log_success "System dependencies installed"
}

install_rust() {
    if command -v rustc &> /dev/null; then
        local rust_version=$(rustc --version)
        log "Rust already installed: $rust_version"

        # Parse version: e.g. "rustc 1.91.0 (...)", extract major.minor
        local rust_major_minor=$(rustc --version | awk '{print $2}' | cut -d. -f1,2)
        local rust_major=$(echo "$rust_major_minor" | cut -d. -f1)
        local rust_minor=$(echo "$rust_major_minor" | cut -d. -f2)
        if [ -n "$rust_major" ] && [ -n "$rust_minor" ] && { [ "$rust_major" -lt 1 ] || { [ "$rust_major" -eq 1 ] && [ "$rust_minor" -lt 91 ]; }; }; then
            log_warning "Rust version too old (project requires 1.91+). Update with 'rustup update' or install from https://rustup.rs"
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
    log "Cloning hKask repository (v${HKASK_VERSION})..."
    rm -rf "$clone_dir"

    # Try version tag first, fall back to main branch
    if git clone --depth 1 --branch "v${HKASK_VERSION}" "$HKASK_REPO_URL" "$clone_dir" 2>/dev/null; then
        log "Checked out tag v${HKASK_VERSION}"
    else
        log "Tag v${HKASK_VERSION} not found, cloning main branch"
        git clone --depth 1 "$HKASK_REPO_URL" "$clone_dir"
    fi
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

# Add kask to PATH. Tries symlink to /usr/local/bin first (system-wide),
# falls back to shell config PATH manipulation (user-local).
add_to_path() {
    # Strategy 1: symlink into /usr/local/bin (already in PATH on all Linux)
    if [ -w "$SYSTEM_BIN" ] || [ "${HKASK_SYSTEM_INSTALL:-false}" = "true" ]; then
        log "Creating symlink at $SYSTEM_BIN/kask → $BIN_DIR/kask"
        if ln -sf "$BIN_DIR/kask" "$SYSTEM_BIN/kask" 2>/dev/null; then
            log_success "kask linked into $SYSTEM_BIN (system PATH)"
            return 0
        fi
        # Symlink failed even with --system — fall through to shell config
        log_warning "Cannot write to $SYSTEM_BIN, falling back to shell config"
    elif command -v sudo &> /dev/null; then
        log "Creating system symlink (requires sudo)..."
        if sudo ln -sf "$BIN_DIR/kask" "$SYSTEM_BIN/kask" 2>/dev/null; then
            log_success "kask linked into $SYSTEM_BIN (system PATH)"
            return 0
        fi
        log_warning "Cannot write to $SYSTEM_BIN, falling back to shell config"
    else
        log "No sudo access — configuring PATH in shell config"
    fi

    # Strategy 2: add BIN_DIR to PATH via shell config files
    local added=false
    local configs=()

    # Detect all applicable shell configs
    if [ -n "${ZSH_VERSION:-}" ]; then
        configs=("$HOME/.zshrc")
    elif [ -n "${BASH_VERSION:-}" ]; then
        configs=("$HOME/.bashrc")
    fi

    # Also add to .profile for login shells (covers ssh, systemd, etc.)
    configs+=("$HOME/.profile")

    # Check if ~/.local/bin needs PATH on this system
    # (systemd 0.25+ ships ~/.local/bin in PATH by default)
    local needs_local_path=false
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        needs_local_path=true
    fi

    if [ "$needs_local_path" = true ]; then
        for cfg in "${configs[@]}"; do
            if [ -f "$cfg" ] || [ "$(basename "$cfg")" = ".profile" ]; then
                if ! grep -q "$BIN_DIR" "$cfg" 2>/dev/null; then
                    echo "" >> "$cfg"
                    echo "# hKask" >> "$cfg"
                    echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$cfg"
                    log "Added PATH entry to $cfg"
                    added=true
                fi
            fi
        done
    fi

    if [ "$added" = true ]; then
        log_success "PATH configured in shell profile(s)"
        log "Restart your shell or run: source ~/.profile"
    else
        log_warning "Could not add $BIN_DIR to PATH automatically"
        log "Please add this line to your shell config:"
        log "  export PATH=\"$BIN_DIR:\$PATH\""
    fi
}

setup_environment() {
    log "Setting up environment..."

    # Add kask to PATH
    add_to_path

    # Also export for this script's process
    export PATH="$BIN_DIR:$PATH"

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
# Conduit Setup (Matrix Homeserver)
# ============================================================================

# Print OS-specific instructions for installing Docker or Podman.
# Called when setup_conduit() detects no container runtime.
print_container_runtime_guide() {
    local pkg_mgr=$(detect_package_manager)

    echo ""
    log_warning "Conduit (Matrix homeserver) requires Docker or Podman."
    echo ""
    echo "  Install a container runtime for your system:"
    echo ""

    case "$pkg_mgr" in
        apt)
            echo "    # Docker (Debian/Ubuntu):"
            echo "    sudo apt-get install -y docker.io"
            echo "    sudo systemctl enable --now docker"
            echo "    sudo usermod -aG docker \$USER  # log out and back in after this"
            echo ""
            echo "    # Or Podman:"
            echo "    sudo apt-get install -y podman podman-compose"
            ;;
        dnf|yum)
            echo "    # Docker (Fedora/RHEL):"
            echo "    sudo dnf install -y docker docker-compose"
            echo "    sudo systemctl enable --now docker"
            echo "    sudo usermod -aG docker \$USER  # log out and back in after this"
            echo ""
            echo "    # Or Podman:"
            echo "    sudo dnf install -y podman podman-compose"
            ;;
        pacman)
            echo "    # Docker (Arch):"
            echo "    sudo pacman -S --noconfirm docker docker-compose"
            echo "    sudo systemctl enable --now docker"
            echo "    sudo usermod -aG docker \$USER  # log out and back in after this"
            echo ""
            echo "    # Or Podman:"
            echo "    sudo pacman -S --noconfirm podman podman-compose"
            ;;
        zypper)
            echo "    # Docker (openSUSE):"
            echo "    sudo zypper install -y docker docker-compose"
            echo "    sudo systemctl enable --now docker"
            echo "    sudo usermod -aG docker \$USER  # log out and back in after this"
            echo ""
            echo "    # Or Podman:"
            echo "    sudo zypper install -y podman podman-compose"
            ;;
        apk)
            echo "    # Docker (Alpine):"
            echo "    sudo apk add docker docker-compose"
            echo "    sudo rc-update add docker boot"
            echo "    sudo service docker start"
            echo "    sudo addgroup \$USER docker  # log out and back in after this"
            ;;
        *)
            echo "    See: https://docs.docker.com/engine/install/"
            echo "    Or:  https://podman.io/getting-started/installation"
            ;;
    esac

    echo ""
    echo "  After installing, start Conduit:"
    echo "    ./scripts/conduit-docker.sh start"
    echo ""
}

# Start Conduit Matrix homeserver via the conduit-docker.sh management script.
#
# If a container runtime (Docker/Podman) is available, pulls the Conduit image,
# starts the container, and waits for it to become healthy. If no runtime is
# found, prints OS-specific install instructions and skips Conduit (non-fatal).
#
# Requires the repo to be cloned (HKASK_SOURCE_DIR must be set).
setup_conduit() {
    local conduit_script="$HKASK_SOURCE_DIR/scripts/conduit-docker.sh"

    if [ ! -f "$conduit_script" ]; then
        log_warning "conduit-docker.sh not found at $conduit_script — skipping Conduit setup"
        return 0
    fi

    log "Setting up Conduit Matrix homeserver..."

    # conduit-docker.sh handles its own runtime detection and will exit with
    # a clear error if neither Docker nor Podman is available.
    if bash "$conduit_script" start; then
        log_success "Conduit Matrix homeserver is running at http://localhost:8008"
        log "Agents will auto-register Matrix accounts on first launch."
        CONDUIT_READY=true
    else
        print_container_runtime_guide
        CONDUIT_READY=false
    fi
}

# ============================================================================
# Verification
# ============================================================================

verify_installation() {
    log "Verifying installation..."

    # Check the binary file exists
    if [ ! -f "$BIN_DIR/kask" ]; then
        log_error "Binary not found at $BIN_DIR/kask"
        return 1
    fi

    local version=$("$BIN_DIR/kask" --version 2>&1 || echo "unknown")
    log "Binary: $BIN_DIR/kask ($version)"

    # Check symlink in /usr/local/bin
    if [ -L "$SYSTEM_BIN/kask" ]; then
        log "Symlink: $SYSTEM_BIN/kask → $(readlink "$SYSTEM_BIN/kask")"
    fi

    # Check if kask is reachable via PATH
    if command -v kask &> /dev/null; then
        local resolved=$(command -v kask)
        log_success "kask is in PATH: $resolved ($version)"
    else
        log_warning "kask command not yet in PATH for this shell session"
        log "The PATH will take effect in new shell sessions. For now:"
        log "  export PATH=\"$BIN_DIR:\$PATH\""
    fi

    # Check Conduit health if it was set up
    if [ "${CONDUIT_READY:-false}" = "true" ]; then
        if curl -s "http://localhost:8008/_matrix/client/versions" > /dev/null 2>&1; then
            log_success "Conduit Matrix homeserver: healthy at http://localhost:8008"
        else
            log_warning "Conduit Matrix homeserver: not responding — check logs with ./scripts/conduit-docker.sh logs"
        fi
    fi
}

# ============================================================================
# Uninstall
# ============================================================================

uninstall_hkask() {
    log "Uninstalling hKask..."

    # Remove system symlink
    if [ -L "$SYSTEM_BIN/kask" ]; then
        sudo rm -f "$SYSTEM_BIN/kask" 2>/dev/null || rm -f "$SYSTEM_BIN/kask" 2>/dev/null || true
        log "Removed symlink: $SYSTEM_BIN/kask"
    fi

    # Remove user binary
    if [ -f "$BIN_DIR/kask" ]; then
        rm -f "$BIN_DIR/kask"
        log "Removed $BIN_DIR/kask"
    fi

    # Remove PATH entries from shell configs
    for cfg in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
        if [ -f "$cfg" ] && grep -q '# hKask' "$cfg" 2>/dev/null; then
            sed -i '/# hKask/d' "$cfg"
            sed -i "/export PATH.*$BIN_DIR/d" "$cfg"
            log "Cleaned PATH entry from $cfg"
        fi
    done

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
    --system            Install system-wide (binary in /usr/local/libexec/hkask, symlink in /usr/local/bin)
    --skip-deps         Skip system dependency installation
    --skip-rust         Skip Rust installation
    --skip-conduit      Skip Conduit Matrix homeserver setup
    --install-dir DIR   Install to custom directory (default: \$HOME/.local)
    --help              Show this help message

Environment Variables:
    HKASK_VERSION       Version to install (default: 0.27.0)
    HKASK_BUILD_TYPE    Build type: release or debug (default: release)
    INSTALL_DIR         Installation directory (default: \$HOME/.local)
    CARGO_HOME          Cargo installation directory (default: \$HOME/.cargo)
    HKASK_SYSTEM_INSTALL Force system-wide install (default: false)
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
    local skip_conduit=false

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
            --system)
                HKASK_SYSTEM_INSTALL="true"
                INSTALL_DIR="/usr/local/libexec/hkask"
                BIN_DIR="/usr/local/libexec/hkask/bin"
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
            --skip-conduit)
                skip_conduit=true
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

    # Post-process: --system requires BIN_DIR to track INSTALL_DIR if the
    # user also supplied --install-dir (which may appear before or after --system).
    if [ "${HKASK_SYSTEM_INSTALL:-false}" = "true" ]; then
        BIN_DIR="${INSTALL_DIR}/bin"
    fi

    echo ""
    echo "╔══════════════════════════════════════════════════════════╗"
    echo "║                    hKask Installer                      ║"
    echo "║        ℏKask - A Minimal Viable Container for Agents    ║"
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

            if [ "$skip_conduit" = false ]; then
                setup_conduit
            else
                log "Skipping Conduit setup (--skip-conduit)"
            fi

            verify_installation

            echo ""
            log_success "Installation complete!"
            echo ""
            echo "To get started:"
            echo "  1. Run hKask:"
            echo "     kask --help"
            echo ""
            echo "  2. Start interactive chat:"
            echo "     kask chat"
            echo ""
            if [ "${CONDUIT_READY:-false}" = "true" ]; then
                echo "  Matrix communication is ready at http://localhost:8008"
                echo "  Manage Conduit: ./scripts/conduit-docker.sh {status|stop|logs|reset}"
                echo ""
            elif [ "$skip_conduit" = false ]; then
                echo "  Matrix communication not available — Conduit not running."
                echo "  Install Docker/Podman, then: ./scripts/conduit-docker.sh start"
                echo ""
            fi
            if ! command -v kask &> /dev/null; then
                echo "  Note: Start a new shell session for PATH changes to take effect."
                echo ""
            fi
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
