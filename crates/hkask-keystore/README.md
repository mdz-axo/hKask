# hkask-keystore

OS keychain integration and AES-256-GCM encryption for hKask.

## Features

- **OS keychain** — stores secrets in the OS-native keystore (Linux: DBus Secret Service)
- **AES-256-GCM** — authenticated encryption for passphrase-protected secrets
- **Interactive passphrase** — prompts user for master passphrase on first run

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PASSPHRASE` | Master passphrase for database encryption |
