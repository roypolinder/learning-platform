#!/usr/bin/env bash
# Manages the local PostgreSQL used in development.
#
# Prefers a system PostgreSQL service if one is already accepting connections on
# PGPORT. Otherwise it controls the user-space cluster created by
# scripts/setup-postgres.sh (no root needed), which lives outside the repo.
set -euo pipefail

PG_HOME="${PG_HOME:-$HOME/.local/postgresql}"
PG_BIN="${PG_BIN:-$PG_HOME/usr/lib/postgresql/14/bin}"
PGDATA="${PGDATA:-$HOME/.local/share/postgresql/14/data}"
PGPORT="${PGPORT:-5432}"
LOG="$PG_HOME/log/postgresql.log"

ready() {
  if command -v pg_isready >/dev/null 2>&1; then pg_isready -q -h localhost -p "$PGPORT"
  elif [ -x "$PG_BIN/pg_isready" ]; then "$PG_BIN/pg_isready" -q -h localhost -p "$PGPORT"
  else (exec 3<>"/dev/tcp/127.0.0.1/$PGPORT") 2>/dev/null; fi
}

case "${1:-status}" in
  start)
    if ready; then echo "PostgreSQL is running on port $PGPORT"; exit 0; fi
    if [ ! -d "$PGDATA" ]; then
      echo "No PostgreSQL found. Run scripts/setup-postgres.sh or install the postgresql package." >&2
      exit 1
    fi
    "$PG_BIN/pg_ctl" -D "$PGDATA" -l "$LOG" -w start >/dev/null
    echo "PostgreSQL started on port $PGPORT"
    ;;
  stop)
    [ -d "$PGDATA" ] && "$PG_BIN/pg_ctl" -D "$PGDATA" -m fast stop || true
    ;;
  status)
    if ready; then echo "PostgreSQL is accepting connections on port $PGPORT"; else echo "PostgreSQL is not running"; exit 1; fi
    ;;
  *)
    echo "usage: $0 start|stop|status" >&2; exit 2 ;;
esac
