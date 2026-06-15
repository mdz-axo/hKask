---
title: "Matrix Cloud Deployment Guide"
audience: [devops, system administrators]
last_updated: 2026-06-15
version: "0.27.0"
status: "Draft"
domain: "Technology"
mds_categories: [lifecycle]
---

# Matrix Cloud Deployment Guide

**Status:** Design document — not yet implemented  
**Audience:** hKask developers  
**Last updated:** 2026-06-15  
**Version:** v0.27.0  
**Domain:** deployment  
**MDS categories:** Domain, Composition, Lifecycle

## Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [Prerequisites](#3-prerequisites)
4. [Caddy Reverse Proxy](#4-caddy-reverse-proxy)
5. [Conduit Cloud Configuration](#5-conduit-cloud-configuration)
6. [Docker Compose Cloud Profile](#6-docker-compose-cloud-profile)
7. [setup-cloud Command](#7-setup-cloud-command)
8. [install.sh --matrix-cloud](#8-installsh---matrix-cloud)
9. [Security Considerations](#9-security-considerations)
10. [Federation (Optional)](#10-federation-optional)
11. [Implementation Checklist](#11-implementation-checklist)

---

## 1. Overview

The LAN setup (`setup-lan`) enables Matrix access for devices on the same WiFi using mDNS hostnames and self-signed TLS certificates. The cloud setup extends this to the public internet — users connect from anywhere using a real domain with trusted Let's Encrypt certificates.

**Key differences from LAN:**

| Concern | LAN | Cloud |
|---------|-----|-------|
| Server name | `<hostname>.local` (mDNS) | `matrix.example.com` (real domain) |
| TLS | Self-signed (user accepts warning) | Let's Encrypt (auto-renewed, trusted) |
| Well-known discovery | Served by Conduit directly | Served by Caddy on root domain |
| User discovery | Manual URL entry | Auto-discovery via `example.com` |
| Reverse proxy | None (Conduit handles TLS) | Caddy (TLS termination + well-known) |
| Network | Same WiFi LAN | Public internet, ports 80/443 open |
| Federation | Disabled | Optional |

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  CLOUD SERVER (public IP)                                        │
│                                                                  │
│  Internet ──→ :80 (HTTP)  ──→ Caddy                             │
│  Internet ──→ :443 (HTTPS) ──→ Caddy                             │
│                    │                                             │
│                    ├─ example.com/.well-known/matrix/client      │
│                    │  → {"m.homeserver": {"base_url":            │
│                    │      "https://matrix.example.com"}}          │
│                    │                                             │
│                    ├─ example.com/.well-known/matrix/server      │
│                    │  → {"m.server": "matrix.example.com:443"}   │
│                    │                                             │
│                    └─ matrix.example.com/*                       │
│                       → reverse_proxy conduit:8000               │
│                                                                  │
│  Conduit (Docker) ←── Caddy ──→ matrix.example.com/_matrix/*   │
│  :8000 (HTTP, internal)                                          │
│  server_name: matrix.example.com                                │
│  TLS: OFF (Caddy terminates TLS)                                 │
│                                                                  │
│  hKask daemon ──→ Conduit :8000 (HTTP, localhost)               │
│  (same machine, no TLS needed for agent communication)           │
└─────────────────────────────────────────────────────────────────┘
```

**Why Caddy?**
- Single static binary, no dependencies
- Automatic Let's Encrypt certificate provisioning and renewal
- Simple Caddyfile configuration (~15 lines)
- Built-in well-known response support
- Handles HTTP→HTTPS redirects automatically

**Why Conduit without TLS?**
- Caddy terminates TLS at the edge — Conduit only sees plain HTTP from the proxy
- Simpler Conduit configuration (no cert paths, no TLS env vars)
- Agent communication (daemon→Conduit) stays on localhost HTTP — no TLS overhead for internal traffic

---

## 3. Prerequisites

Before running cloud setup, the operator must have:

| Prerequisite | How to verify |
|-------------|---------------|
| **Domain name** (e.g., `example.com`) | Purchased from a registrar |
| **DNS A/AAAA records** pointing `example.com` and `matrix.example.com` to the server's public IP | `dig +short example.com` returns the server IP |
| **Ports 80 and 443 open** on the server firewall | `curl -s http://<server-ip>` reaches the server |
| **Docker or Podman** installed | `docker compose version` |
| **Server with public IP** | Cloud VM (AWS EC2, DigitalOcean Droplet, Hetzner, etc.) |

**DNS setup example:**
```
example.com         A     → 203.0.113.10
matrix.example.com  A     → 203.0.113.10
```

The script will verify DNS resolution before proceeding. If the domain doesn't resolve to the current server, setup aborts with a clear error.

---

## 4. Caddy Reverse Proxy

### Caddyfile (auto-generated by setup-cloud)

```
# Generated by: conduit-docker.sh setup-cloud example.com
# Location: scripts/conduit-caddy/Caddyfile

# Root domain — serves well-known discovery only
example.com {
    # Matrix client discovery
    header /.well-known/matrix/* Content-Type application/json
    header /.well-known/matrix/* Access-Control-Allow-Origin *
    respond /.well-known/matrix/client `{"m.homeserver": {"base_url": "https://matrix.example.com"}}`
    respond /.well-known/matrix/server `{"m.server": "matrix.example.com:443"}`

    # Everything else → 404 (this domain only does discovery)
    respond 404
}

# Matrix subdomain — proxies to Conduit
matrix.example.com {
    reverse_proxy conduit:8000
}
```

**How discovery works for the user:**
1. User opens FluffyChat
2. Enters their domain: `example.com` (not the matrix subdomain)
3. Client fetches `https://example.com/.well-known/matrix/client`
4. Receives `{"m.homeserver": {"base_url": "https://matrix.example.com"}}`
5. Client connects to `https://matrix.example.com` automatically
6. User never sees the subdomain — just enters their domain name

### Caddy Docker Compose Service

```yaml
# Added to conduit-docker.cloud.yml
caddy:
  image: caddy:2
  container_name: hkask-caddy
  restart: unless-stopped
  ports:
    - "80:80"
    - "443:443"
  volumes:
    - ./conduit-caddy/Caddyfile:/etc/caddy/Caddyfile:ro
    - caddy-data:/data
    - caddy-config:/config
  networks:
    - conduit-net
```

---

## 5. Conduit Cloud Configuration

Conduit runs without TLS — Caddy handles it. Key env var changes from the base config:

```yaml
conduit:
  environment:
    CONDUIT_CONFIG: ""
    CONDUIT_SERVER_NAME: "matrix.${DOMAIN}"     # real domain, not localhost
    CONDUIT_ADDRESS: "0.0.0.0"                  # accept connections from Caddy
    CONDUIT_DATABASE_BACKEND: "rocksdb"
    CONDUIT_DATABASE_PATH: "/var/lib/matrix-conduit"
    CONDUIT_ALLOW_REGISTRATION: "true"
    CONDUIT_ALLOW_FEDERATION: "false"           # optional — see §10
    CONDUIT_ALLOW_ENCRYPTION: "false"           # Caddy handles TLS
    CONDUIT_REGISTRATION_TOKEN: "${REG_TOKEN}"  # keep registration protected
    CONDUIT_MAX_REQUEST_SIZE: "10000000"
    CONDUIT_TRUSTED_SERVERS: "[]"
    CONDUIT_LOG: "info"
    # NO TLS env vars — Caddy terminates TLS
    # NO well-known env vars — Caddy serves discovery
  ports:
    - "127.0.0.1:8000:8000"   # only exposed to localhost (Caddy + daemon)
  networks:
    - conduit-net
```

**Key differences from base config:**
- `CONDUIT_SERVER_NAME` set to the real domain
- Port binding restricted to `127.0.0.1:8000` — Conduit is not directly exposed to the internet
- No `CONDUIT_TLS_*` vars — Caddy handles TLS
- No `CONDUIT_WELL_KNOWN_*` vars — Caddy serves discovery
- Both Caddy and Conduit share a Docker network (`conduit-net`) so Caddy can reach `conduit:8000` by container name

---

## 6. Docker Compose Cloud Profile

New file: `scripts/conduit-docker.cloud.yml`

```yaml
# Conduit Cloud Profile — Caddy reverse proxy + Let's Encrypt TLS
#
# Extends conduit-docker.yml for public internet deployment.
# Requires: domain name, DNS A records, ports 80/443 open.
#
# Generated by: conduit-docker.sh setup-cloud <domain>

services:
  caddy:
    image: caddy:2
    container_name: hkask-caddy
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - CADDYFILE_DIR_PLACEHOLDER/Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
      - caddy-config:/config

  conduit:
    ports:
      - "127.0.0.1:8000:8000"
    environment:
      CONDUIT_SERVER_NAME: "matrix.DOMAIN_PLACEHOLDER"
      CONDUIT_ALLOW_ENCRYPTION: "false"
    networks:
      default: null

networks:
  default:
    name: conduit-net
    driver: bridge

volumes:
  caddy-data:
    name: hkask-caddy-data
  caddy-config:
    name: hkask-caddy-config
```

The cloud profile is used as an override:
```bash
docker compose -f conduit-docker.yml -f conduit-docker.cloud.yml up -d
```

---

## 7. setup-cloud Command

New subcommand in `scripts/conduit-docker.sh`:

```
Usage: ./scripts/conduit-docker.sh setup-cloud <domain>
```

### Implementation pseudocode

```bash
cmd_setup_cloud() {
    local domain="${1:-}"
    if [ -z "$domain" ]; then
        log_error "Usage: $0 setup-cloud <domain>"
        log_error "Example: $0 setup-cloud example.com"
        return 1
    fi

    # 1. Verify DNS — domain must resolve to this server's public IP
    log_info "Verifying DNS for $domain and matrix.$domain..."
    local server_ip=$(curl -s ifconfig.me)
    local domain_ip=$(dig +short "$domain" | tail -1)
    if [ "$domain_ip" != "$server_ip" ]; then
        log_error "$domain resolves to $domain_ip, but server IP is $server_ip"
        log_error "Update your DNS A records and retry."
        return 1
    fi

    # 2. Verify ports 80 and 443 are reachable
    log_info "Checking ports 80 and 443..."
    # (firewall check — optional, may fail if Caddy isn't running yet)

    # 3. Create Caddyfile
    local caddy_dir="$SCRIPT_DIR/conduit-caddy"
    mkdir -p "$caddy_dir"
    cat > "$caddy_dir/Caddyfile" << CADDYEOF
$domain {
    header /.well-known/matrix/* Content-Type application/json
    header /.well-known/matrix/* Access-Control-Allow-Origin *
    respond /.well-known/matrix/client \`{"m.homeserver": {"base_url": "https://matrix.$domain"}}\`
    respond /.well-known/matrix/server \`{"m.server": "matrix.$domain:443"}\`
    respond 404
}
matrix.$domain {
    reverse_proxy conduit:8000
}
CADDYEOF

    # 4. Generate cloud compose override
    local cloud_compose="$SCRIPT_DIR/conduit-docker.cloud.yml"
    # (write the YAML from §6, substituting placeholders)

    # 5. Start the stack
    log_info "Starting Conduit + Caddy..."
    $DOCKER_COMPOSE -f "$COMPOSE_FILE" -f "$cloud_compose" up -d

    # 6. Wait for Caddy to obtain Let's Encrypt certificate
    log_info "Waiting for Let's Encrypt certificate provisioning..."
    # Caddy obtains certs on first HTTPS request — poll until valid
    local max_attempts=60
    local attempt=1
    while [ $attempt -le $max_attempts ]; do
        if curl -s "https://$domain/.well-known/matrix/client" > /dev/null 2>&1; then
            log_info "TLS certificate obtained and discovery responding"
            break
        fi
        sleep 2
        attempt=$((attempt + 1))
    done

    # 7. Register Curator under the new domain
    log_info "Registering Curator on matrix.$domain..."
    # Use the registration API with the cloud domain
    local response
    response=$(curl -s -X POST "https://matrix.$domain/_matrix/client/v3/register" \
        -H "Content-Type: application/json" \
        -d "{
            \"username\": \"curator\",
            \"password\": \"UserSovereignty\",
            \"initial_device_display_name\": \"hKask Curator\",
            \"auth\": {\"type\": \"m.login.registration_token\", \"token\": \"$reg_token\"}
        }")

    # 8. Print connection instructions
    echo ""
    echo "  ╔══════════════════════════════════════════════════════════╗"
    echo "  ║  Matrix Cloud Access — Connect from anywhere             ║"
    echo "  ╠══════════════════════════════════════════════════════════╣"
    echo "  ║                                                          ║"
    echo "  ║  Your Matrix server is live at:                          ║"
    echo "  ║    https://matrix.$domain                                ║"
    echo "  ║                                                          ║"
    echo "  ║  Users connect by entering their domain:                 ║"
    echo "  ║    $domain                                               ║"
    echo "  ║  (auto-discovery via .well-known)                        ║"
    echo "  ║                                                          ║"
    echo "  ║  How to connect:                                         ║"
    echo "  ║  1. Curator creates accounts:                            ║"
    echo "  ║     kask matrix register-user <name>                     ║"
    echo "  ║  2. Human installs a Matrix client:                      ║"
    echo "  ║     Mobile:  FluffyChat or Element X                     ║"
    echo "  ║     Desktop: Element or FluffyChat                       ║"
    echo "  ║  3. In the client, enter: $domain                        ║"
    echo "  ║     (the client auto-discovers the server)               ║"
    echo "  ║  4. Log in with provisioned credentials                  ║"
    echo "  ║                                                          ║"
    echo "  ║  Curator admin: @curator:$domain / UserSovereignty      ║"
    echo "  ║                                                          ║"
    echo "  ╚══════════════════════════════════════════════════════════╝"
}
```

---

## 8. install.sh --matrix-cloud

New flag in `scripts/install.sh`:

```bash
curl ... | bash -s -- --matrix-cloud example.com
```

### Implementation

```bash
# In main() argument parsing:
--matrix-cloud)
    matrix_cloud="$2"
    shift 2
    ;;

# In the install flow, after setup_conduit():
if [ -n "$matrix_cloud" ] && [ "${CONDUIT_READY:-false}" = "true" ]; then
    log "Setting up cloud access for domain: $matrix_cloud"
    bash "$HKASK_SOURCE_DIR/scripts/conduit-docker.sh" setup-cloud "$matrix_cloud"
fi
```

### Validation before proceeding

The install script should verify:
1. Domain resolves to this server's public IP
2. Ports 80/443 are not already bound
3. `dig` and `curl` are available (already installed as dependencies)

If validation fails, print a clear error and continue without Matrix (non-fatal — same as LAN setup).

---

## 9. Security Considerations

| Concern | Mitigation |
|---------|-----------|
| **Open registration** | Registration token required (`CONDUIT_REGISTRATION_TOKEN`). Curator provisions all accounts. No self-registration. |
| **Conduit not exposed to internet** | Port binding `127.0.0.1:8000` — only Caddy (on the same Docker network) can reach Conduit. |
| **Let's Encrypt renewal** | Caddy handles automatically. No cron jobs, no certbot scripts. |
| **Database encryption** | Conduit's RocksDB is on a Docker volume. For production, consider disk-level encryption (LUKS) or encrypted EBS volumes. |
| **Firewall** | Only ports 80 and 443 open. Port 8008 (agent HTTP) stays internal. Port 8448 only if federation is enabled. |
| **SSH access** | Standard cloud VM security — key-based SSH, fail2ban, non-standard port. |
| **Backups** | Regular backups of the `hkask-conduit-db` Docker volume. See `docs/guides/DEPLOYMENT.md` §8. |

---

## 10. Federation (Optional)

Federation allows your users to DM people on other Matrix servers (matrix.org, etc.). Enable by setting:

```yaml
CONDUIT_ALLOW_FEDERATION: "true"
CONDUIT_TRUSTED_SERVERS: '["matrix.org"]'
```

Additional requirements:
- Port 8448 must be open on the firewall
- `.well-known/matrix/server` must return `{"m.server": "matrix.example.com:443"}` (already in Caddyfile)
- SRV DNS record recommended: `_matrix._tcp.example.com SRV 10 0 443 matrix.example.com`

**Recommendation:** Start with federation disabled. Enable it once the server is stable and you've verified internal communication works. Federation adds complexity (key management, remote media caching, spam from other servers).

---

## 11. Implementation Checklist

When implementing the cloud deployment feature:

- [ ] **`scripts/conduit-docker.sh`** — add `cmd_setup_cloud()` function
  - [ ] DNS verification (domain resolves to server IP)
  - [ ] Caddyfile generation
  - [ ] Cloud compose override generation (`conduit-docker.cloud.yml`)
  - [ ] Stack startup (base + cloud compose files)
  - [ ] Let's Encrypt provisioning wait loop
  - [ ] Curator registration under cloud domain
  - [ ] Connection instructions banner
- [ ] **`scripts/conduit-docker.cloud.yml`** — template with placeholders
- [ ] **`scripts/install.sh`** — add `--matrix-cloud <domain>` flag
  - [ ] Argument parsing
  - [ ] Call `setup-cloud` after `setup_conduit()`
  - [ ] Help text update
- [ ] **`AGENTS.md`** — update pre-install section with cloud option
- [ ] **`docs/guides/DEPLOYMENT.md`** — add Matrix cloud deployment section referencing this guide
- [ ] **Test** — deploy on a cloud VM with a test domain, verify:
  - [ ] Caddy obtains Let's Encrypt certificate
  - [ ] Well-known discovery responds correctly
  - [ ] `kask matrix register-user` works against the cloud server
  - [ ] FluffyChat connects from a phone (not on LAN)
  - [ ] Agent communication works (daemon → Conduit on localhost)
