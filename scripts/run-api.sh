#!/usr/bin/env bash

set -eu

HERE="$(dirname "$0")"
SCRIPT="$(basename "$0")"
export API_HOST="127.0.0.1"
export PLATZ_FRONTEND_PORT="5173"

if [ -z "${DATABASE_URL:-}" ]
then
    source "${HERE}/run-db.sh"
fi

if [ -z "${OIDC_SERVER_URL:-}" ]
then
    source "${HERE}/run-oidc.sh"
fi

if ! which cargo-watch &>/dev/null
then
    echo "[${SCRIPT}] 🦀 Installing cargo watch"
    cargo install cargo-watch
fi

export RUST_LOG="debug"
export RUST_BACKTRACE="1"
export PLATZ_OWN_URL="http://localhost:${PLATZ_FRONTEND_PORT}"
# From oidc-users.json
export ADMIN_EMAILS="admin@example.com"

echo "[${SCRIPT}] 🚀 Running API server"
args=("$@")
exec cargo watch -x "run --bin=platz-api -- run ${args[*]}"
