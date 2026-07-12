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
#   ./scripts/conduit-docker.sh register    # Register the Curator admin user
#
# Requirements:
#   - Docker (docker compose or docker-compose)
#   - curl (for health checks and registration)
#
# After starting, the homeserver is available at http://localhost:8008.
# The Curator (@curator:localhost) is the Matrix admin — manages account
# creation, deletion, and moderation on the server.
# System bots (hkask-curator, 7R7, etc.) auto-register during bootstrap.
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/conduit-docker.yml"
HOMESERVER_URL="http://localhost:8008"

# Detect container runtime (docker compose > podman compose > docker-compose > podman-compose)
if docker compose version &>/dev/null; then
    DOCKER_COMPOSE="docker compose"
elif podman compose version &>/dev/null; then
    DOCKER_COMPOSE="podman compose"
elif docker-compose --version &>/dev/null; then
    DOCKER_COMPOSE="docker-compose"
elif podman-compose --version &>/dev/null; then
    DOCKER_COMPOSE="podman-compose"
else
    echo "Error: No container runtime found. Install Docker or Podman with compose support."
    echo "  Docker: https://docs.docker.com/engine/install/"
    echo "  Podman: https://podman.io/getting-started/installation"
    exit 1
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

    # Handle pre-existing container from any compose project (or bare docker run).
    # docker compose ps only sees containers from the current project;
    # a container created under a different project name will collide by name.
    if docker ps -a --format '{{.Names}}' 2>/dev/null | grep -qx "hkask-conduit"; then
        if docker ps --format '{{.Names}}' 2>/dev/null | grep -qx "hkask-conduit"; then
            log_info "Conduit container is already running"
        else
            log_info "Removing stale Conduit container..."
            docker rm -f hkask-conduit 2>/dev/null || true
        fi
    fi

    # Also handle stale volume from a different compose project.
    local vol_count
    vol_count=$(docker volume ls --format '{{.Name}}' 2>/dev/null | grep -c "hkask-conduit-db" || true)
    if [ "$vol_count" -gt 0 ]; then
        # Check if the volume is not in use by any container
        if ! docker ps -a --filter "volume=hkask-conduit-db" --format '{{.Names}}' 2>/dev/null | grep -q .; then
            log_info "Removing orphaned hkask-conduit-db volume..."
            docker volume rm -f hkask-conduit-db 2>/dev/null || true
        fi
    fi

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
            echo "  │  Next: register the Curator user:                       │"
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
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" down 2>/dev/null || true
    # Also handle container created under a different compose project name
    docker rm -f hkask-conduit 2>/dev/null || true
    log_info "Conduit stopped"
}

cmd_status() {
    if curl -s "$HOMESERVER_URL/_matrix/client/versions" > /dev/null 2>&1; then
        log_info "Conduit is running and healthy at $HOMESERVER_URL"
        echo ""
        if command -v python3 &>/dev/null; then curl -s "$HOMESERVER_URL/_matrix/client/versions" | python3 -m json.tool 2>/dev/null || true; else curl -s "$HOMESERVER_URL/_matrix/client/versions"; fi
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
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" down -v 2>/dev/null || true
    # Force-remove container and volume at Docker level (handles cross-project collisions)
    docker rm -f hkask-conduit 2>/dev/null || true
    docker volume rm -f hkask-conduit-db 2>/dev/null || true
    log_info "Container and database volume removed"
    log_info "Starting fresh Conduit instance..."
    cmd_start
}

cmd_register() {
    # Register the Curator as the Matrix admin. The Curator manages account
    # creation, deletion, and moderation on the Conduit homeserver.
    # Default credentials: curator / UserSovereignty
    local username="${1:-curator}"
    local password="${2:-UserSovereignty}"
    # Warn if default password is being used (local dev only)
    if [ "${2:-}" = "" ]; then
        log_warn "Using default Curator password. Change it for production use."
    fi
    local reg_token="${HKASK_MATRIX_REGISTRATION_TOKEN:-hkask-dev}"

    log_info "Registering Curator user '$username' on $HOMESERVER_URL..."

    # Use the Matrix registration API with registration token
    local response
    response=$(curl -s -X POST "$HOMESERVER_URL/_matrix/client/v3/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"$username\",
            \"password\": \"$password\",
            \"initial_device_display_name\": \"hKask Admin\",
            \"auth\": {\"type\": \"m.login.registration_token\", \"token\": \"$reg_token\"}
        }")

    if echo "$response" | grep -q "access_token" && ! echo "$response" | grep -q "errcode"; then
        log_info "Curator user '$username' registered successfully!"
        echo ""
        echo "  Username: @$username:localhost"
        echo "  Password: $password"
        echo ""
        echo "  Save these credentials. This user has admin privileges."
    else
        log_error "Registration failed. Response:"
        if command -v python3 &>/dev/null; then echo "$response" | python3 -m json.tool 2>/dev/null || echo "$response"; else echo "$response"; fi
        log_warn "If the server already has users, the first user is the Curator."
        log_warn "Try logging in with existing credentials instead."
    fi
}

# ── LAN Setup (TLS + well-known for phone access) ───────────────────────────

# Directory for TLS certificates
TLS_DIR="${SCRIPT_DIR}/conduit-tls"

cmd_setup_lan() {
    local lan_host="${1:-}"

    # Detect LAN hostname if not provided
    if [ -z "$lan_host" ]; then
        lan_host="$(hostname).local"
        log_info "Detected LAN hostname: $lan_host"
        echo ""
        echo "  This hostname will be used for phone connections."
        echo "  To use a different name: $0 setup-lan <hostname>"
        echo ""
    fi

    # Create TLS directory
    mkdir -p "$TLS_DIR"

    # Generate self-signed TLS certificate for the LAN hostname
    if [ ! -f "$TLS_DIR/fullchain.pem" ]; then
        log_info "Generating self-signed TLS certificate for $lan_host..."
        openssl req -x509 -newkey rsa:4096 -keyout "$TLS_DIR/privkey.pem" \
            -out "$TLS_DIR/fullchain.pem" -days 3650 -nodes \
            -subj "/CN=$lan_host" 2>/dev/null
        log_info "TLS certificate generated (valid 10 years)"
    else
        log_info "TLS certificate already exists at $TLS_DIR"
    fi

    # Create LAN override compose file
    local lan_compose="$SCRIPT_DIR/conduit-docker.lan.yml"
    cat > "$lan_compose" << 'LANEOF'
# Conduit LAN Override — TLS + well-known for phone access
#
# Extends conduit-docker.yml with:
#   - TLS certificates for HTTPS
#   - Well-known discovery so Matrix clients auto-connect
#   - Server name set to LAN hostname
#
# Generated by: conduit-docker.sh setup-lan

services:
  conduit:
    ports:
      - "8448:8000"
    volumes:
      - TLS_DIR_PLACEHOLDER:/etc/conduit-tls:ro
    environment:
      CONDUIT_SERVER_NAME: "LAN_HOST_PLACEHOLDER"
      CONDUIT_TLS_CERTS: "/etc/conduit-tls/fullchain.pem"
      CONDUIT_TLS_KEY: "/etc/conduit-tls/privkey.pem"
      CONDUIT_WELL_KNOWN_CLIENT: "https://LAN_HOST_PLACEHOLDER:8448"
LANEOF

    # Substitute placeholders with actual values
    sed -i "s|TLS_DIR_PLACEHOLDER|$TLS_DIR|g" "$lan_compose"
    sed -i "s|LAN_HOST_PLACEHOLDER|$lan_host|g" "$lan_compose"

    log_info "LAN configuration written to $lan_compose"

    # Restart Conduit with LAN override
    log_info "Restarting Conduit with LAN TLS configuration..."
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" -f "$lan_compose" down 2>/dev/null || true
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" -f "$lan_compose" up -d

    # Wait for TLS health
    log_info "Waiting for Conduit TLS to become healthy..."
    local max_attempts=30
    local attempt=1
    while [ $attempt -le $max_attempts ]; do
        if curl -sk "https://localhost:8448/_matrix/client/versions" > /dev/null 2>&1; then
            log_info "Conduit TLS is healthy at https://localhost:8448"
            break
        fi
        sleep 1
        attempt=$((attempt + 1))
    done

    # Print phone connection instructions
    echo ""
    echo "  ╔══════════════════════════════════════════════════════════╗"
    echo "  ║  Matrix LAN Access — Connect from your devices           ║"
    echo "  ╠══════════════════════════════════════════════════════════╣"
    echo "  ║                                                          ║"
    echo "  ║  Server URL:  https://$lan_host:8448                     ║"
    echo "  ║                                                          ║"
    echo "  ║  How to connect:                                         ║"
    echo "  ║  1. Curator creates an account for each human user:     ║"
    echo "  ║     kask matrix register-user <name>                     ║"
    echo "  ║     Share the MXID + password with them securely.        ║"
    echo "  ║                                                          ║"
    echo "  ║  2. Human user installs a Matrix client:                ║"
    echo "  ║     Mobile:  FluffyChat or Element X                     ║"
    echo "  ║     Desktop: Element or FluffyChat                       ║"
    echo "  ║     (any Matrix-compliant client works)                  ║"
    echo "  ║                                                          ║"
    echo "  ║  3. In the client, select \"Use custom server\"          ║"
    echo "  ║     Enter: https://$lan_host:8448                        ║"
    echo "  ║                                                          ║"
    echo "  ║  4. Log in with the MXID and password from step 1       ║"
    echo "  ║                                                          ║"
    echo "  ║  ⚠ Self-signed certificate — accept the warning         ║"
    echo "  ║                                                          ║"
    echo "  ║  Curator admin: @curator:$lan_host / UserSovereignty    ║"
    echo "  ║                                                          ║"
    echo "  ╚══════════════════════════════════════════════════════════╝"
    echo ""
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
    setup-lan)
        cmd_setup_lan "${2:-}"
        ;;
    *)
        echo "Usage: $0 {start|stop|status|logs|reset|register|setup-lan [hostname]}"
        echo ""
        echo "Commands:"
        echo "  start       Start Conduit Docker container"
        echo "  stop        Stop Conduit Docker container"
        echo "  status      Check if Conduit is healthy"
        echo "  logs        Tail Conduit logs"
        echo "  reset       Stop, delete database, and restart fresh"
        echo "  register    Register the Curator admin user (default: curator / UserSovereignty)"
        echo "  setup-lan   Enable TLS + well-known for phone access over LAN"
        echo ""
        echo "After starting, hKask agents connect via:"
        echo "  HKASK_MATRIX_URL=http://localhost:8008 kask chat"
        echo ""
        echo "For phone access (FluffyChat etc.):"
        echo "  $0 setup-lan"
        exit 1
        ;;
esac
