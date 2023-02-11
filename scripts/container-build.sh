#!/usr/bin/env bash

set -eu
set -o pipefail

TARGETARCH="$1"
BUILD_DEST="$2"

case "$TARGETARCH" in \
    arm64)
        LINUX_TARGETARCH="aarch64"
        ;;
    amd64)
        LINUX_TARGETARCH="x86_64"
        ;;
    *)
        echo "Unknown arch $TARGETARCH"
        exit 1
    ;;
esac

dpkg --add-architecture "${TARGETARCH}"
apt-get update
apt-get install -y "libpq-dev:${TARGETARCH}"

RUST_TARGET="${LINUX_TARGETARCH}-unknown-linux-gnu"

if [ "${LINUX_TARGETARCH}" != `uname -m` ]
then
    apt-get install -y "gcc-`echo ${LINUX_TARGETARCH} | tr '_' '-'`-linux-gnu"
fi

cat >"${CARGO_HOME}/config.toml" <<EOF
[target.x86_64-unknown-linux-gnu]
linker = "x86_64-linux-gnu-gcc"

[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
EOF

export CC_x86_64_unknown_linux_gnu="x86_64-linux-gnu-gcc"
export CXX_x86_64_unknown_linux_gnu="x86_64-linux-gnu-g++"
export AR_x86_64_unknown_linux_gnu="x86_64-linux-gnu-ar"

export CC_aarch64_unknown_linux_gnu="aarch64-linux-gnu-gcc"
export CXX_aarch64_unknown_linux_gnu="aarch64-linux-gnu-g++"
export AR_aarch64_unknown_linux_gnu="aarch64-linux-gnu-ar"

export LDFLAGS="-L/usr/lib/${LINUX_TARGETARCH}-linux-gnu"

rustup target add "${RUST_TARGET}"
cargo build --release --target="${RUST_TARGET}"

find "target/${RUST_TARGET}/release/" -maxdepth 1 -type f -executable -exec mv -v {} "${BUILD_DEST}/" \;
