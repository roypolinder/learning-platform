#!/usr/bin/env bash
# Runs the API, restarting on changes to Rust code or course files when cargo-watch is available.
set -euo pipefail
cd "$(dirname "$0")/.."
if command -v cargo-watch >/dev/null 2>&1; then
  exec cargo watch -q -c -w apps/api -w crates -w courses -x "run -q -p lp-api --bin learning-platform -- serve"
fi
exec cargo run -q -p lp-api --bin learning-platform -- serve
