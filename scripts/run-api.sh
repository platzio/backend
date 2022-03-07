#!/usr/bin/env bash

set -e

SCRIPT=`basename $0`

if [ -z "${DATABASE_URL}" ]
then
    echo "error: Please define DATABASE_URL manually or run ./run-db.sh"
    exit 1
fi

if ! which cargo-watch &>/dev/null
then
    echo "[${SCRIPT}] 🦀 Installing cargo watch"
    cargo install cargo-watch
fi

export RUST_BACKTRACE="1"
export OIDC_SERVER_URL="https://accounts.google.com"
export OIDC_CLIENT_ID="id"
export OIDC_CLIENT_SECRET="secret"

echo "[${SCRIPT}] 🚀 Running API server"
args=("$@")
exec cargo watch -x "run --bin=platz-api -- --debug ${args[*]}"
