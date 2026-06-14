#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# Conduit Docker Sidecar Setup for hKask
# ──────────────────────────────────────────────────────────────────────────────
#
# Starts a local-only Conduit Matrix homeserver in Docker for hKask's
# agent-to-agent communication. No federation, no TLS — single-machine only.
#
# Usage:
#   ./scripts/conduit-docker.sh start       # Start Conduit
#   ./scripts/conduit-docker.sh stop        # Stop Conduit
#   ./scripts/conduit-docker.sh status      # Check if running
#   ./scripts/conduit-docker.sh logs        # Tail logs
#   ./scripts/conduit-docker.sh reset       # Stop, delete DB, restart fresh
#   ./scripts/conduit-docker.sh register    # Register first admin user
#
# Requirements:
#   - Docker (docker compose or docker-compose)
#   - curl (for health checks and registration)
#
# After starting, the homeserver is available at http://localhost:8008.
# The first user to register becomes the admin.
# hKask agents auto-register via the communication MCP server.
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/conduit-docker.yml"
HOMESERVER_URL="http://localhost:8008"

# Detect docker compose variant (v2 uses "docker compose", v1 uses "docker-compose")
if docker compose version &>/dev/null; then
    DOCKER_COMPOSE="docker compose"
else
    DOCKER_COMPOSE="docker-compose"
fi

# ── Colors ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# ── Commands ─────────────────────────────────────────────────────────────────

cmd_start() {
    log_info "Starting Conduit Matrix homeserver..."
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" up -d

    log_info "Waiting for Conduit to become healthy..."
    local max_attempts=30
    local attempt=1
    while [ $attempt -le $max_attempts ]; do
        if curl -s "$HOMESERVER_URL/_matrix/client/versions" > /dev/null 2>&1; then
            log_info "Conduit is healthy and responding at $HOMESERVER_URL"
            echo ""
            echo "  ┌─────────────────────────────────────────────────────────┐"
            echo "  │  Conduit is running at http://localhost:8008            │"
            echo "  │                                                         │"
            echo "  │  Next: register an admin user:                          │"
            echo "  │    ./scripts/conduit-docker.sh register                 │"
            echo "  │                                                         │"
            echo "  │  Then start hKask with Matrix enabled:                  │"
            echo "  │    HKASK_MATRIX_URL=http://localhost:8008 kask chat     │"
            echo "  └─────────────────────────────────────────────────────────┘"
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done

    log_error "Conduit did not become healthy within ${max_attempts}s"
    log_error "Check logs: $0 logs"
    return 1
}

cmd_stop() {
    log_info "Stopping Conduit..."
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" down
    log_info "Conduit stopped"
}

cmd_status() {
    if curl -s "$HOMESERVER_URL/_matrix/client/versions" > /dev/null 2>&1; then
        log_info "Conduit is running and healthy at $HOMESERVER_URL"
        echo ""
        curl -s "$HOMESERVER_URL/_matrix/client/versions" | python3 -m json.tool 2>/dev/null || true
    else
        log_warn "Conduit is not responding at $HOMESERVER_URL"
        if $DOCKER_COMPOSE -f "$COMPOSE_FILE" ps | grep -q "hkask-conduit"; then
            log_warn "Container exists but may be starting up. Check logs: $0 logs"
        else
            log_warn "Container is not running. Start it: $0 start"
        fi
    fi
}

cmd_logs() {
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" logs -f --tail=50
}

cmd_reset() {
    log_warn "This will DELETE the Conduit database and all user accounts!"
    read -rp "Are you sure? [y/N] " confirm
    if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
        log_info "Aborted"
        return 0
    fi

    log_info "Stopping Conduit..."
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" down -v
    log_info "Database volume deleted"
    log_info "Starting fresh Conduit instance..."
    cmd_start
}

cmd_register() {
    local username="${1:-admin}"
    local password="${2:-hKaskAdmin123!}"

    log_info "Registering admin user '$username' on $HOMESERVER_URL..."

    # Use the Matrix registration API (no auth required for initial registration)
    local response
    response=$(curl -s -X POST "$HOMESERVER_URL/_matrix/client/v3/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$username\",
            \"password\": \"$password\",
            \"initial_device_display_name\": \"hKask Admin\",
            \"auth\": {\"type\": \"m.login.dummy\"}
        }")

    if echo "$response" | grep -q "access_token"; then
        log_info "Admin user '$username' registered successfully!"
        echo ""
        echo "  Username: @$username:localhost"
        echo "  Password: $password"
        echo ""
        echo "  Save these credentials. This user has admin privileges."
    else
        log_error "Registration failed. Response:"
        echo "$response" | python3 -m json.tool 2>/dev/null || echo "$response"
        log_warn "If the server already has users, the first user is the admin."
        log_warn "Try logging in with existing credentials instead."
    fi
}

# ── Main ─────────────────────────────────────────────────────────────────────

case "${1:-}" in
    start)
        cmd_start
        ;;
    stop)
        cmd_stop
        ;;
    status)
        cmd_status
        ;;
    logs)
        cmd_logs
        ;;
    reset)
        cmd_reset
        ;;
    register)
        cmd_register "${2:-}" "${3:-}"
        ;;
    *)
        echo "Usage: $0 {start|stop|status|logs|reset|register [username] [password]}"
        echo ""
        echo "Commands:"
        echo "  start       Start Conduit Docker container"
        echo "  stop        Stop Conduit Docker container"
        echo "  status      Check if Conduit is healthy"
        echo "  logs        Tail Conduit logs"
        echo "  reset       Stop, delete database, and restart fresh"
        echo "  register    Register an admin user (default: admin / hKaskAdmin123!)"
        echo ""
        echo "After starting, hKask agents connect via:"
        echo "  HKASK_MATRIX_URL=http://localhost:8008 kask chat"
        exit 1
        ;;
esac
