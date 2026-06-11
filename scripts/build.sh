#!/bin/bash
# hKask Build Script
#
# This script builds hKask with proper configuration for Linux systems.
#
# Usage: ./scripts/build.sh [OPTIONS]
# Options:
#   --release     Build in release mode (default: debug)
#   --all         Build all targets including tests
#   --doc         Build documentation
#   --clean       Clean before building
#   --help        Show help

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[BUILD]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse arguments
BUILD_TYPE="debug"
BUILD_ALL=false
BUILD_DOC=false
CLEAN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_TYPE="release"
            shift
            ;;
        --all)
            BUILD_ALL=true
            shift
            ;;
        --doc)
            BUILD_DOC=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release     Build in release mode (default: debug)"
            echo "  --all         Build all targets including tests"
            echo "  --doc         Build documentation"
            echo "  --clean       Clean before building"
            echo "  --help        Show this help"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Get workspace root
WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$WORKSPACE_ROOT"

log "Building hKask in $WORKSPACE_ROOT"

# Clean if requested
if [ "$CLEAN" = true ]; then
    log "Cleaning build artifacts..."
    cargo clean
fi

# Build command
CARGO_CMD="cargo"
CARGO_ARGS=()

if [ "$BUILD_TYPE" = "release" ]; then
    CARGO_ARGS+=("--release")
fi

if [ "$BUILD_ALL" = true ]; then
    CARGO_ARGS+=("--all-targets")
fi

CARGO_ARGS+=("--workspace")

# Check for required system dependencies
check_dependencies() {
    log "Checking system dependencies..."

    local missing=()

    # Check for build essentials
    if ! command -v gcc &> /dev/null; then
        missing+=("gcc")
    fi

    if ! command -v pkg-config &> /dev/null; then
        missing+=("pkg-config")
    fi

    if ! command -v cmake &> /dev/null; then
        missing+=("cmake")
    fi

    if ! command -v protoc &> /dev/null; then
        missing+=("protobuf-compiler")
    fi

    # Check for required development libraries
    if ! pkg-config --exists openssl 2>/dev/null; then
        missing+=("libssl-dev")
    fi

    if ! pkg-config --exists sqlite3 2>/dev/null; then
        missing+=("libsqlite3-dev")
    fi

    if ! pkg-config --exists libzstd 2>/dev/null; then
        missing+=("libzstd-dev")
    fi

    if ! pkg-config --exists dbus-1 2>/dev/null; then
        missing+=("libdbus-1-dev")
    fi

    if ! command -v llvm-config &> /dev/null && ! pkg-config --exists libclang 2>/dev/null; then
        missing+=("libclang-dev/llvm-dev")
    fi

    if [ ${#missing[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing[*]}"
        log "Please install using your package manager:"
        log "  Debian/Ubuntu: sudo apt-get install build-essential pkg-config libssl-dev libsqlite3-dev libdbus-1-dev libclang-dev llvm-dev cmake protobuf-compiler libprotobuf-dev libzstd-dev"
        log "  Fedora/RHEL: sudo dnf install gcc pkg-config openssl-devel sqlite-devel clang-devel llvm-devel cmake protobuf-compiler protobuf-devel libzstd-devel"
        log "  Arch: sudo pacman -S base-devel pkg-config openssl sqlite clang llvm cmake protobuf protobuf-c zstd"
        exit 1
    fi

    log_success "System dependencies OK"
}

# Check Rust toolchain
check_rust() {
    log "Checking Rust toolchain..."

    if ! command -v rustc &> /dev/null; then
        log_error "Rust not found. Install with:"
        log "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    local rust_version=$(rustc --version)
    log "Rust version: $rust_version"

    # Check for required components
    if ! rustup component list --installed | grep -q rustfmt; then
        log "Installing rustfmt..."
        rustup component add rustfmt
    fi

    if ! rustup component list --installed | grep -q clippy; then
        log "Installing clippy..."
        rustup component add clippy
    fi

    log_success "Rust toolchain OK"
}

# Main build
main() {
    check_dependencies
    check_rust

    log "Starting build (type: $BUILD_TYPE)..."

    # Run cargo check first for faster feedback
    log "Running cargo check..."
    cargo check "${CARGO_ARGS[@]}"

    # Full build
    log "Building workspace..."
    $CARGO_CMD build "${CARGO_ARGS[@]}"

    # Build documentation if requested
    if [ "$BUILD_DOC" = true ]; then
        log "Building documentation..."
        cargo doc --workspace --no-deps
    fi

    # Run tests if --all was specified
    if [ "$BUILD_ALL" = true ]; then
        log "Running tests..."
        cargo test --workspace --lib
    fi

    log_success "Build complete!"

    # Show binary location
    if [ "$BUILD_TYPE" = "release" ]; then
        echo ""
        log "Release binary: target/release/kask"
        ls -lh target/release/kask 2>/dev/null || true
    else
        echo ""
        log "Debug binary: target/debug/kask"
        ls -lh target/debug/kask 2>/dev/null || true
    fi
}

main
