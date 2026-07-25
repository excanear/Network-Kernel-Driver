#!/usr/bin/env bash
# Provisions PostgreSQL + TimescaleDB inside the WSL2 Ubuntu distro for
# Network Observatory Phase F. Idempotent-ish: safe to re-run.
set -euo pipefail

echo "== apt update/install base packages =="
sudo apt-get update -y
sudo apt-get install -y postgresql postgresql-contrib build-essential \
  postgresql-server-dev-all git cmake libssl-dev pkg-config curl

PG_VERSION=$(psql --version | grep -oP '\d+' | head -1)
echo "== Detected PostgreSQL major version: ${PG_VERSION} =="

echo "== Attempting TimescaleDB via PGDG/Timescale apt repo =="
TIMESCALE_OK=0
if ! dpkg -l | grep -q timescaledb-2-postgresql; then
  (
    echo "deb https://packagecloud.io/timescale/timescaledb/ubuntu/ noble main" | \
      sudo tee /etc/apt/sources.list.d/timescaledb.list
    curl -fsSL https://packagecloud.io/timescale/timescaledb/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/timescaledb.gpg 2>/dev/null || true
    sudo apt-get update -y
    sudo apt-get install -y "timescaledb-2-postgresql-${PG_VERSION}" && TIMESCALE_OK=1
  ) || true
fi

if ! dpkg -l | grep -q timescaledb-2-postgresql; then
  echo "== apt package unavailable for this Ubuntu release, building TimescaleDB from source =="
  rm -rf /tmp/timescaledb-src
  git clone --branch 2.17.2 --depth 1 https://github.com/timescale/timescaledb.git /tmp/timescaledb-src
  cd /tmp/timescaledb-src
  ./bootstrap -DREGRESS_CHECKS=OFF
  cd build && make -j"$(nproc)" && sudo make install
fi

echo "== Configuring postgresql.conf / pg_hba.conf =="
PG_CONF_DIR="/etc/postgresql/${PG_VERSION}/main"
sudo sed -i "s/^#listen_addresses.*/listen_addresses = '*'/" "${PG_CONF_DIR}/postgresql.conf"
if ! grep -q "shared_preload_libraries.*timescaledb" "${PG_CONF_DIR}/postgresql.conf"; then
  echo "shared_preload_libraries = 'timescaledb'" | sudo tee -a "${PG_CONF_DIR}/postgresql.conf"
fi
if ! grep -q "host all all 0.0.0.0/0 md5" "${PG_CONF_DIR}/pg_hba.conf"; then
  echo "host all all 0.0.0.0/0 md5" | sudo tee -a "${PG_CONF_DIR}/pg_hba.conf"
fi

sudo service postgresql restart
sleep 3

echo "== Creating database/user/extension =="
sudo -u postgres psql -v ON_ERROR_STOP=0 <<'SQL'
DO $$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'netobs') THEN
    CREATE ROLE netobs LOGIN PASSWORD 'netobs_dev_password';
  END IF;
END
$$;
SELECT 'CREATE DATABASE network_observatory OWNER netobs'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'network_observatory')\gexec
SQL

sudo -u postgres psql -d network_observatory -c "CREATE EXTENSION IF NOT EXISTS timescaledb;" || \
  echo "WARNING: timescaledb extension could not be created — see docs/phase1-slice-design.md limitations note"

echo "== Done. Connection string: postgres://netobs:netobs_dev_password@localhost:5432/network_observatory =="
