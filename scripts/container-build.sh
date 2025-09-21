#!/usr/bin/env bash

set -euo pipefail

RELEASE_BUILD="$1"
BUILD_DEST="$2"

export CARGO_BUILD_TARGET="$(arch)-unknown-linux-musl"
rustup target add "${CARGO_BUILD_TARGET}"

if [ "${RELEASE_BUILD}" = "1" ]
then
    CARGO_FLAGS="--release"
    CARGO_TARGET_DIR="target/${CARGO_BUILD_TARGET}/release/"
else
    CARGO_FLAGS=""
    CARGO_TARGET_DIR="target/${CARGO_BUILD_TARGET}/debug/"
fi

cargo build ${CARGO_FLAGS}
find "${CARGO_TARGET_DIR}" -maxdepth 1 -type f -executable -exec cp -v {} "${BUILD_DEST}/" \;
