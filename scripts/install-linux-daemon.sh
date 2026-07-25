#!/usr/bin/env bash
# Installs network-observatoryd as a systemd service (Phase I).
# Run with sudo. Builds a release binary first if missing.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT_DIR/target/release/network-observatoryd"

if [ ! -x "$BIN" ]; then
  echo "Release binary not found, building..."
  (cd "$ROOT_DIR" && cargo build --release -p service)
fi

id -u netobs &>/dev/null || useradd --system --no-create-home --shell /usr/sbin/nologin netobs

install -Dm755 "$BIN" /usr/local/bin/network-observatoryd
install -d -o netobs -g netobs /var/lib/network-observatory
install -Dm644 "$ROOT_DIR/scripts/network-observatoryd.service" /etc/systemd/system/network-observatoryd.service

systemctl daemon-reload
systemctl enable --now network-observatoryd

echo "Done. Check status with: systemctl status network-observatoryd"
