---
title: "hKask Admin Install Guide"
audience: [server admins, DevOps]
last_updated: 2026-06-17
version: "0.27.0"
status: "Draft — Planning Phase"
domain: "Technology"
mds_categories: [lifecycle]
---

# hKask Admin Install Guide

**Purpose:** Step-by-step instructions for deploying a hKask cloud server. After completing this guide, you will have a running hKask server accessible at your domain, with OAuth sign-in and multi-user support.

**Prerequisite documents:**
- [`docs/plans/deployment-and-backup.md`](../plans/deployment-and-backup.md) — deployment architecture and design decisions
- [`docs/architecture/hKask-architecture-master.md`](../architecture/hKask-architecture-master.md) — full architecture reference

---

## 1. Overview

After setup, your server will look like this:

```
https://hkask.your-domain.com
  │
  ├── /login       — OAuth sign-in (GitHub / Google)
  ├── /terminal    — xterm.js terminal with tiled/tabbed layout
  │                   (1/2/4 tiles, up to 6 tabs per tile, max 24 terminals)
  ├── /api/v1/*    — REST API (chat, pods, backup, wallet, etc.)
  │
  └── Caddy (Docker) handles TLS + reverse proxy
      Conduit (Docker) handles Matrix federation
```

Users visit the URL, sign in, and get a terminal — already logged in. Open additional tabs or tiles without re-authenticating. No install. No SSH. No binary.

---

## 2. Prerequisites

### 2.1 Server

| Requirement | Minimum |
|-------------|---------|
| **OS** | Ubuntu 22.04+ (or any Linux with Docker) |
| **RAM** | 2 GB (1 GB for hKask + agents, ~200 MB for Caddy + Conduit) |
| **Disk** | 20 GB (user data, agent memory, backups) |
| **Network** | Public IP, ports 80 and 443 open |

### 2.2 Domain

A domain name pointed at your server's public IP. You can use:

- A domain you already own (e.g., `your-domain.com`)
- A subdomain (e.g., `hkask.your-domain.com`)
- A domain purchased from any registrar (Namecheap, Cloudflare, Google Domains, etc.)

**Before starting:** Create a DNS A record:

```
hkask.your-domain.com  →  A  →  <your-server-public-IP>
```

Wait for DNS propagation (usually 60 seconds to a few hours). Verify with:

```bash
dig +short hkask.your-domain.com
# Should return your server's IP
```

### 2.3 OAuth Providers

You need at least one OAuth provider configured so users can sign in. hKask supports GitHub and Google.

---

## 3. Create OAuth Apps

You must do this manually in each provider's developer console. hKask will tell you the exact callback URL during init, but here's what you'll need for each.

### 3.1 GitHub OAuth

1. Go to [github.com/settings/developers](https://github.com/settings/developers)
2. Click **New OAuth App**
3. Fill in:

| Field | Value |
|-------|-------|
| Application name | `hKask` (or whatever you prefer) |
| Homepage URL | `https://hkask.your-domain.com` |
| Authorization callback URL | `https://hkask.your-domain.com/auth/callback` |

4. Click **Register application**
5. Click **Generate a new client secret**
6. Save the **Client ID** and **Client Secret** — you'll add them to `providers.env` in step 5.

### 3.2 Google OAuth (optional)

1. Go to [console.cloud.google.com](https://console.cloud.google.com)
2. Create a project (or use an existing one)
3. Navigate to **APIs & Services → Credentials**
4. Configure the OAuth consent screen:
   - User type: **External**
   - App name: `hKask`
   - Authorized domains: `your-domain.com`
   - Scopes: `email`, `profile` (default)
5. Click **Create Credentials → OAuth client ID**
   - Application type: **Web application**
   - Authorized redirect URIs: `https://hkask.your-domain.com/auth/callback`
6. Save the **Client ID** and **Client Secret**.

---

## 4. Install Docker

hKask uses Docker for the Caddy (TLS) and Conduit (Matrix) sidecars.

```bash
# Ubuntu
sudo apt update
sudo apt install -y docker.io docker-compose-v2
sudo systemctl enable --now docker

# Verify
docker --version
docker compose version
```

---

## 5. Install the kask Binary

### Option A: Download from GitHub Releases (recommended)

```bash
# Replace with actual release URL once published
curl -fsSL https://github.com/org/hkask/releases/latest/download/kask-linux-x86_64 \
  -o /usr/local/bin/kask
chmod +x /usr/local/bin/kask

# Verify
kask --version
```

### Option B: Build from Source

```bash
# Prerequisites: Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build
git clone https://github.com/org/hkask.git
cd hkask
cargo build --release
cp target/release/kask /usr/local/bin/kask
```

---

## 6. Prepare Secrets — providers.env

The `providers.env` file holds all your secrets in one place. After loading them into the OS keychain, the file is shredded.

```bash
cp providers.env.example providers.env
```

Edit `providers.env` and fill in your keys:

```bash
# ── Inference Providers (at least one required) ───────────────────
DI_API_KEY=sk-...                 # DeepInfra
TOGETHER_API_KEY=...               # Together AI

# FA_API_KEY=...                  # fal.ai (optional)
# RUNPOD_API_KEY=...             # RunPod (optional)
# BASETEN_API_KEY=...            # Baseten (optional)

# ── OAuth Providers (at least one required) ───────────────────────
OAUTH_GITHUB_CLIENT_ID=Iv23li...
OAUTH_GITHUB_CLIENT_SECRET=abc123...
# OAUTH_GOOGLE_CLIENT_ID=...      # Google (optional)
# OAUTH_GOOGLE_CLIENT_SECRET=...  # Google (optional)
```

**Keep this file safe until shredded.** It contains plaintext secrets. After `kask keystore load --shred`, the file is permanently destroyed.

---

## 7. Initialize the Server

```bash
kask init --profile server
```

The init process walks through five steps:

### Step 1: Master Passphrase

```
Create a master passphrase for this hKask server.
This passphrase encrypts all data at rest and derives
internal signing keys. Choose a strong passphrase.
```

This passphrase is stored in the OS keychain. It is never written to disk in plaintext. All databases (user data, agent memory, wallet) are encrypted with keys derived from it via Argon2id → HKDF-SHA256.

### Step 2: Domain

```
What domain will this server be reachable at?
  https://hkask.your-domain.com
```

hKask validates that the domain resolves to this machine's public IP. If DNS isn't pointed yet, it tells you exactly what record to create and which IP to use, then exits.

### Step 3: Inference API Keys

```
Load inference provider keys?
  [1] Load from providers.env (then shred the file)
  [2] Enter keys directly (masked input)
  [3] Skip — configure later with 'kask keystore load'
```

Choose option 1 if you completed step 6 above. hKask parses the file, shows which keys it found, asks for shredding consent, loads keys into the OS keychain, and securely deletes the plaintext file.

### Step 4: OAuth Provider Credentials

```
OAuth callback URLs:
  GitHub:  https://hkask.your-domain.com/auth/callback
  Google:  https://hkask.your-domain.com/auth/callback

Load OAuth credentials?
  [1] Load from providers.env (already loaded in step 3)
  [2] Enter Client ID + Client Secret now
  [3] Skip — configure later

At least one OAuth provider must be configured before users can sign in.
```

Option 1 works if you put OAuth keys in the same providers.env file. The `keystore load` command handles all key prefixes.

### Step 5: Sidecar Deployment

```
Deploy Caddy + Conduit sidecars?
  Domain: hkask.your-domain.com
  Caddy:  TLS termination (auto Let's Encrypt)
  Conduit: Matrix homeserver (agent communication)

  [1] Deploy now — generates Docker config, then runs docker compose up
  [2] Generate config only — you run docker compose up later
```

Option 1 generates `docker-compose.yml`, `Caddyfile`, and `conduit.toml` in `~/.config/hkask/sidecar/`, then runs `docker compose up -d`. Caddy auto-obtains a Let's Encrypt TLS certificate. Wait ~30 seconds for the certificate to be issued.

Option 2 generates the files but doesn't start the containers. Run later with:

```bash
cd ~/.config/hkask/sidecar
docker compose up -d
```

### Init Complete

```
Server is ready.

Visit: https://hkask.your-domain.com
Sign in with GitHub or Google.
You are the first admin.

Invite members: kask invite <email>
```

---

## 8. Verify the Deployment

### 8.1 Check Caddy TLS

```bash
curl -I https://hkask.your-domain.com
# Should return HTTP/2 200 with a valid Let's Encrypt certificate
```

### 8.2 Check Conduit

```bash
kask matrix status-sidecar
# Should show:
#   Caddy:    UP (healthy, TLS serving)
#   Conduit:  UP (healthy, API responding)
```

### 8.3 First Sign-In

Open `https://hkask.your-domain.com` in a browser. Click **Sign in with GitHub** (or Google). Complete the OAuth flow. You should see a terminal with the `kask>` prompt.

---

## 9. Invite Members

From the terminal (or via API):

```bash
kask invite alice@example.com
# → Invitation sent. Alice can sign in at https://hkask.your-domain.com
```

The invitee signs in with the same OAuth provider. Their account is created with the **Member** role. They can see their own settings but not server config.

```bash
kask invite bob@example.com
# → Make Bob an admin?
#    [1] Admin — can manage server config and invite others
#    [2] Member — user access only

kask members list
#   alice@example.com  —  Member  —  last active: 2026-06-17
#   bob@example.com    —  Admin   —  last active: 2026-06-17
```

---

## 10. Backup

### Manual Export

```bash
kask backup export --passphrase "your-backup-passphrase"
# → archive.db created. Download via scp or the API.
```

### Scheduled Auto-Export

```bash
kask config set backup.auto-export.frequency daily
kask config set backup.auto-export.retention 7
# → Server generates an encrypted backup daily, keeps the last 7.
# → Available at: /var/lib/hkask/exports/{webid}/
```

**To migrate to a new server:** Download the archive, run `kask backup upload --server https://new-server.example` on the new server. See the migration section in the deployment plan.

---

## 11. Troubleshooting

| Symptom | Check |
|---------|-------|
| "DNS not pointing here" during init | Verify `dig +short hkask.your-domain.com` returns your server IP. Wait for propagation. |
| Caddy can't get TLS certificate | Ports 80 and 443 must be open. Check firewall. `docker logs hkask-caddy` |
| OAuth callback fails | Verify callback URL in GitHub/Google console exactly matches `https://hkask.your-domain.com/auth/callback` |
| "No OAuth provider configured" | Run `kask keystore list \| grep OAUTH` to verify credentials are in the keychain |
| Conduit not responding | `docker logs hkask-conduit`. Check `~/.config/hkask/sidecar/conduit.toml` |
| Terminal WebSocket fails | Check browser console for WebSocket errors. Verify session cookie is set after OAuth. |
