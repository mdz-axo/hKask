# hKask Admin Install Guide

**Audience:** Server operators deploying hKask for a team.  
**Last updated:** 2026-06-23  
**Version:** 0.30.0

---

## Overview

hKask deploys as a single Rust binary (`kask`) with two Docker sidecars (Caddy for TLS, Conduit for Matrix messaging). Users access hKask through a browser — no client install required.

**Architecture at a glance:**

```
Browser ──[HTTPS]──▶ Caddy (TLS) ──▶ kask serve (axum)
                                   ──▶ Conduit (Matrix)
```

---

## Prerequisites

| Requirement | Minimum | Notes |
|-------------|---------|-------|
| **OS** | Linux (amd64) | Debian/Ubuntu recommended. Tested on bookworm. |
| **RAM** | 2 GB | More if running inference locally. |
| **Disk** | 10 GB | Grows with user data. SQLite-backed. |
| **Docker** | 24+ | For Caddy + Conduit sidecars. |
| **Domain** | One FQDN | e.g., `hkask.example.com`. DNS A record must point to server. |
| **Ports** | 80, 443 open | Required for Let's Encrypt HTTP challenge + HTTPS. |
| **Rust** | 1.91+ | Only if building from source. |

---

## Step 1: Build or Pull

### Option A: Pre-built Docker image

```bash
docker pull ghcr.io/mdz-axo/hkask:kask-main
```

The image bundles `kask` + Litestream (SQLite WAL backup) + Conduit (Matrix homeserver) managed by supervisord. Suitable for k8s or single-container deployments.

### Option B: Build from source

```bash
git clone https://github.com/mdz-axo/hKask.git
cd hKask
cargo build --release --bin kask
sudo cp target/release/kask /usr/local/bin/kask
```

---

## Step 2: Initialize the Server

Run the interactive setup:

```bash
kask init
```

You will be prompted for:

| Prompt | What to enter | Notes |
|--------|--------------|-------|
| **Master passphrase** | A strong passphrase (≥8 chars) | Encrypts the SQLCipher database. Store this securely. |
| **Data directory** | `/var/lib/hkask` (default) | Where all user data, exports, and configuration live. |
| **Domain name** | `hkask.example.com` | Must match DNS. Used for TLS and OAuth redirect URIs. |
| **GitHub Client ID** | From GitHub OAuth App | Create at https://github.com/settings/developers |
| **GitHub Client Secret** | From GitHub OAuth App | Stored in OS keychain, not on disk. |

**What this creates:**

```
~/.config/hkask/
  ├── config.json       # Server config (domain, data dir)
  └── hkask.service     # systemd unit file

/var/lib/hkask/         # Data directory (created)

OS keychain:
  hkask-master          # Master passphrase
  hkask-oauth-github-*  # OAuth credentials
```

### GitHub OAuth App Setup

1. Go to https://github.com/settings/developers → "New OAuth App"
2. Set **Homepage URL** to `https://hkask.example.com`
3. Set **Authorization callback URL** to `https://hkask.example.com/api/v1/auth/callback?provider=github`
4. Copy the Client ID and generate a Client Secret

---

## Step 3: Deploy Sidecars (Caddy + Conduit)

Generate Docker Compose configuration:

```bash
kask matrix deploy-sidecar --domain hkask.example.com
```

This creates `~/.config/hkask/sidecar/` with:

| File | Purpose |
|------|---------|
| `docker-compose.yml` | Caddy + Conduit containers |
| `Caddyfile` | Auto-TLS reverse proxy config |
| `conduit.toml` | Matrix homeserver configuration |

Start the sidecars:

```bash
cd ~/.config/hkask/sidecar
docker compose up -d
```

Wait ~30 seconds for Caddy to obtain a Let's Encrypt certificate, then verify:

```bash
kask matrix status-sidecar
```

Expected output:

```
Caddy:       healthy (HTTPS on port 443)
Conduit:     healthy (Matrix API on port 8008)
```

---

## Step 4: Start the Server

No environment variables needed — `kask init` stored OAuth credentials in the OS keychain, and the server reads them directly from there.

Only `HKASK_DOMAIN` may be needed if you chose a non-default domain:

```bash
export HKASK_DOMAIN="hkask.example.com"
```

### Option A: Foreground (for testing)

```bash
kask serve
```

The API server listens on `http://127.0.0.1:3000` by default. Caddy proxies HTTPS traffic to this port.

### Option B: systemd (for production)

```bash
sudo cp ~/.config/hkask/hkask.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now hkask
```

The systemd unit was generated during `kask init`. It runs `kask serve` as the `hkask` user with auto-restart on failure.

Check status:

```bash
sudo systemctl status hkask
```

---

## Step 5: Verify — First Sign-In

1. Open `https://hkask.example.com` in a browser.
2. Click **"Sign in with GitHub"**.
3. Authorize the OAuth app.
4. You should be redirected to `/terminal` — a browser-based terminal connected to `kask repl`.

If you see a terminal prompt, the deployment is working.

---

## Step 6: Configure Backups

### Operational backups (automatic)

hKask automatically snapshots pod state and configuration to a local Git content-addressed store. No configuration required — this runs via the CNS backup loop.

Check backup status:

```bash
kask backup snapshot --scope full
kask backup list
```

### Sovereignty exports (user-initiated)

Users can export their data as an encrypted archive:

```bash
kask export create --passphrase "their-chosen-passphrase"
```

Exports are stored at `/var/lib/hkask/exports/{webid}/` and downloadable via the API at `GET /api/v1/export/download`.

---

## Step 7: Register Additional Users

Additional team members sign in the same way — visit the domain, click "Sign in with GitHub." Each gets their own WebID-scoped terminal, replicants, and data.

To make someone an admin:

```bash
kask replicant list                    # Find their replicant name
# Admin promotion is done via the UserStore (CLI command TBD)
```

Invite codes can be generated for pre-authorized sign-ups:

```bash
# TBD: kask invite create --email user@example.com
```

---

## Directory Layout

```
~/.config/hkask/
  ├── config.json           # Server configuration
  ├── hkask.service          # systemd unit
  └── sidecar/               # Docker Compose for Caddy + Conduit
      ├── docker-compose.yml
      ├── Caddyfile
      └── conduit.toml

/var/lib/hkask/
  ├── kask.db               # SQLCipher-encrypted main database
  ├── agents/               # Per-pod SQLCipher databases
  ├── exports/{webid}/      # Sovereignty export archives
  └── registry/             # Template registry
```

---

## Troubleshooting

### "Connection refused" on port 443

- Check that Caddy is running: `docker ps | grep caddy`
- Check DNS: `dig hkask.example.com` should return your server's IP.
- Check firewall: ports 80 and 443 must be open for Let's Encrypt.

### "OAuth callback failed"

- Verify the callback URL in GitHub OAuth App settings matches exactly.
- Check that `HKASK_DOMAIN` is set correctly.
- Check that GitHub Client ID and Secret are set.

### "kask: command not found"

- Ensure `/usr/local/bin` is in PATH.
- If using Docker image: `docker run ghcr.io/mdz-axo/hkask:kask-main kask --help`

### Database errors

- Check that `/var/lib/hkask/` exists and is writable by the `hkask` user.
- If migrating from a previous version, run: `kask migrate --data-dir /var/lib/hkask`

### Sidecar health check fails

- Ensure Docker daemon is running.
- `cd ~/.config/hkask/sidecar && docker compose logs` for detailed logs.
- Conduit may take 15-30 seconds to initialize its database on first start.

---

## Related Documents

- [Deployment & Multi-User Plan](../plans/deployment-and-backup.md) — Full architecture and design decisions
- [PRINCIPLES.md](../architecture/core/PRINCIPLES.md) — Magna Carta P1–P12
- [Matrix Integration Architecture](../architecture/matrix-integration-architecture.md) — Conduit sidecar details
