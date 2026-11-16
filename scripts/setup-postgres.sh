#!/usr/bin/env bash
# Creates a PostgreSQL 14 cluster in user space from Ubuntu's own packages, plus the
# application role and databases, and writes DATABASE_URL into .env.
#
# Use this when you cannot (or prefer not to) `sudo apt install postgresql`.
# With a system install, create the role/databases yourself and edit .env instead.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PG_HOME="$HOME/.local/postgresql"
PG_BIN="$PG_HOME/usr/lib/postgresql/14/bin"
PGDATA="$HOME/.local/share/postgresql/14/data"

if [ ! -x "$PG_BIN/postgres" ]; then
  tmp="$(mktemp -d)"
  (cd "$tmp" && apt-get download postgresql-14 postgresql-client-14 >/dev/null)
  mkdir -p "$PG_HOME"
  for deb in "$tmp"/*.deb; do dpkg -x "$deb" "$PG_HOME"; done
  rm -rf "$tmp"
fi

mkdir -p "$PG_HOME/run" "$PG_HOME/log" "$(dirname "$PGDATA")"
umask 077
if [ ! -d "$PGDATA" ]; then
  head -c 24 /dev/urandom | base64 | tr -d '/+=' > "$PG_HOME/.superuser_pw"
  "$PG_BIN/initdb" -D "$PGDATA" -U postgres --auth-local=scram-sha-256 --auth-host=scram-sha-256 \
    --pwfile="$PG_HOME/.superuser_pw" -E UTF8 --locale=C.UTF-8 >/dev/null
  cat >> "$PGDATA/postgresql.conf" <<CONF
listen_addresses = 'localhost'
port = 5432
unix_socket_directories = '$PG_HOME/run'
CONF
fi

"$ROOT/scripts/db.sh" start

if [ ! -f "$PG_HOME/.app_pw" ]; then
  head -c 24 /dev/urandom | base64 | tr -d '/+=' > "$PG_HOME/.app_pw"
  PGPASSWORD="$(cat "$PG_HOME/.superuser_pw")" "$PG_BIN/psql" -h localhost -U postgres -q -v ON_ERROR_STOP=1 <<SQL
CREATE ROLE learning_platform LOGIN CREATEDB PASSWORD '$(cat "$PG_HOME/.app_pw")';
CREATE DATABASE learning_platform OWNER learning_platform;
CREATE DATABASE learning_platform_test OWNER learning_platform;
SQL
fi

if [ ! -f "$ROOT/.env" ]; then
  sed "s#CHANGE_ME#$(cat "$PG_HOME/.app_pw")#" "$ROOT/.env.example" > "$ROOT/.env"
  echo "Wrote .env"
fi
echo "PostgreSQL ready."
