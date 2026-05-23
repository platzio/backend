# Multi-stage build using cargo-chef for dep-only layer caching and per-arch
# cache mounts so amd64 and arm64 don't fight over the same target dir.
#
# Drives the workspace into a static musl binary so the runtime image (alpine-
# based platzio/base) doesn't need a libc. Architecture is selected via Docker
# Buildx' automatic TARGETARCH build arg — set platforms in the build invocation,
# not here.

ARG BASE_IMAGE
ARG RUST_IMAGE=rust:1-trixie

# ---------------------------------------------------------------------------
# 1. chef — Rust toolchain + musl tooling + cargo-chef. Shared by planner and
# builder so both layers reuse the same toolchain image.
# ---------------------------------------------------------------------------
FROM ${RUST_IMAGE} AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
        musl \
        musl-dev \
        musl-tools \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked --version ^0.1
WORKDIR /build

# ---------------------------------------------------------------------------
# 2. planner — strip the workspace down to a "recipe" describing the dep graph.
# This stage is invalidated by *any* source change, but it's cheap (no compile).
# ---------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------------------------------------------------------------------------
# 3. builder — cook deps from the recipe, then build the workspace. The cooked
# deps live in a buildkit cache mount keyed by TARGETARCH, so each architecture
# keeps its own warm target dir across CI runs.
# ---------------------------------------------------------------------------
FROM chef AS builder
ARG RELEASE_BUILD=1
ARG TARGETARCH

RUN set -eux; \
    case "${TARGETARCH}" in \
        amd64) target=x86_64-unknown-linux-musl ;; \
        arm64) target=aarch64-unknown-linux-musl ;; \
        *) echo "Unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    echo "${target}" > /target.txt; \
    rustup target add "${target}"

COPY --from=planner /build/recipe.json recipe.json
RUN --mount=type=cache,id=platz-cargo-target-${TARGETARCH},target=/build/target,sharing=locked \
    --mount=type=cache,id=platz-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=platz-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    set -eux; \
    target="$(cat /target.txt)"; \
    if [ "${RELEASE_BUILD}" = "1" ]; then \
        cargo chef cook --release --target "${target}" --recipe-path recipe.json; \
    else \
        cargo chef cook --target "${target}" --recipe-path recipe.json; \
    fi

COPY . .
RUN --mount=type=cache,id=platz-cargo-target-${TARGETARCH},target=/build/target,sharing=locked \
    --mount=type=cache,id=platz-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=platz-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    set -eux; \
    target="$(cat /target.txt)"; \
    if [ "${RELEASE_BUILD}" = "1" ]; then \
        cargo build --release --target "${target}"; \
        out_dir="target/${target}/release"; \
    else \
        cargo build --target "${target}"; \
        out_dir="target/${target}/debug"; \
    fi; \
    mkdir -p /out; \
    find "${out_dir}" -maxdepth 1 -type f -executable -exec cp -v {} /out/ \;

# ---------------------------------------------------------------------------
# 4a. dev — debug-mode build kept in the Rust toolchain image so Tilt's
# live_update can run `cargo build` *inside* the container after syncing
# source. No cache mount on the build step: the warm /build/target dir
# survives into the resulting image and incremental rebuilds in the running
# container reuse it. Dynamic-linked debian runtime (libpq5) — the musl-
# static release path isn't useful when we're recompiling at runtime.
#
# Selected by `--target=dev` (the Tiltfile in platzio/dev sets this). Not
# referenced by any other stage, so default builds skip it.
# ---------------------------------------------------------------------------
FROM ${RUST_IMAGE} AS dev
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libpq5 \
        libpq-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY . .
RUN cargo build --workspace --bins \
    && mkdir -p /root \
    && cp target/debug/platz-api             /root/platz-api \
    && cp target/debug/platz-k8s-agent       /root/platz-k8s-agent \
    && cp target/debug/platz-chart-discovery /root/platz-chart-discovery \
    && cp target/debug/platz-status-updates  /root/platz-status-updates \
    && cp target/debug/platz-resource-sync   /root/platz-resource-sync
WORKDIR /root

# ---------------------------------------------------------------------------
# 4. runtime — small base image carrying just the static musl binaries.
# ---------------------------------------------------------------------------
FROM ${BASE_IMAGE}
WORKDIR /root/
COPY --from=builder /out/* /root/
