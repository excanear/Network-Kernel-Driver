#!/usr/bin/env bash
# Provisions PostgreSQL 17 + TimescaleDB inside the WSL2 Ubuntu distro for
# Network Observatory Phase F. Idempotent-ish: safe to re-run.
#
# Pinned to PostgreSQL 17 deliberately: this distro's default (Ubuntu
# "resolute" ships PostgreSQL 18) is newer than TimescaleDB currently
# supports ("TimescaleDB only supports PostgreSQL 14, 15, 16 and 17" as of
# the 2.17.x series) — so we install 17 from the official PGDG apt repo
# side-by-side rather than fighting the distro default.
set -euo pipefail

PG_VERSION=17

echo "== apt update/install base packages =="
sudo apt-get update -y
sudo apt-get install -y curl ca-certificates gnupg build-essential cmake git libssl-dev pkg-config

if [ ! -f /etc/apt/sources.list.d/pgdg.list ]; then
  echo "== Adding PGDG apt repo for PostgreSQL ${PG_VERSION} =="
  sudo install -d /usr/share/postgresql-common/pgdg
  sudo curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc \
    -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc
  . /etc/os-release
  echo "deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt ${VERSION_CODENAME}-pgdg main" | \
    sudo tee /etc/apt/sources.list.d/pgdg.list
  sudo apt-get update -y
fi

sudo apt-get install -y "postgresql-${PG_VERSION}" "postgresql-server-dev-${PG_VERSION}"

# The PGDG package doesn't always auto-create a cluster the way the
# distro-native postgresql package does — create it if pg_lsclusters
# doesn't already show one.
if ! sudo pg_lsclusters | awk -v v="$PG_VERSION" '$1==v {found=1} END{exit !found}'; then
  sudo pg_createcluster "$PG_VERSION" main --start
fi

PG_CONFIG="/usr/lib/postgresql/${PG_VERSION}/bin/pg_config"

echo "== Building TimescaleDB from source against PostgreSQL ${PG_VERSION} =="
if [ ! -d /tmp/timescaledb-src ]; then
  git clone --branch 2.17.2 --depth 1 https://github.com/timescale/timescaledb.git /tmp/timescaledb-src
fi
cd /tmp/timescaledb-src
rm -rf build
PG_CONFIG="$PG_CONFIG" ./bootstrap -DREGRESS_CHECKS=OFF -DPG_CONFIG="$PG_CONFIG"
cd build
make -j"$(nproc)"
sudo make install

echo "== Configuring postgresql.conf / pg_hba.conf for PostgreSQL ${PG_VERSION} =="
PG_CONF_DIR="/etc/postgresql/${PG_VERSION}/main"
sudo sed -i "s/^#listen_addresses.*/listen_addresses = '*'/" "${PG_CONF_DIR}/postgresql.conf"
if ! grep -q "shared_preload_libraries.*timescaledb" "${PG_CONF_DIR}/postgresql.conf"; then
  echo "shared_preload_libraries = 'timescaledb'" | sudo tee -a "${PG_CONF_DIR}/postgresql.conf"
fi
if ! sudo grep -q "host all all 0.0.0.0/0 md5" "${PG_CONF_DIR}/pg_hba.conf"; then
  echo "host all all 0.0.0.0/0 md5" | sudo tee -a "${PG_CONF_DIR}/pg_hba.conf"
fi

sudo pg_ctlcluster "${PG_VERSION}" main restart
sleep 3

PG_PORT=$(sudo pg_lsclusters | awk -v v="$PG_VERSION" '$1==v {print $3}')
echo "== PostgreSQL ${PG_VERSION} cluster is listening on port ${PG_PORT} =="

echo "== Creating database/user/extension =="
sudo -u postgres psql -p "$PG_PORT" -v ON_ERROR_STOP=0 <<'SQL'
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

sudo -u postgres psql -p "$PG_PORT" -d network_observatory -c "CREATE EXTENSION IF NOT EXISTS timescaledb;" || \
  echo "WARNING: timescaledb extension could not be created"

echo "== Done. Connection string: postgres://netobs:netobs_dev_password@localhost:${PG_PORT}/network_observatory =="
