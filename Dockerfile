ARG BASE_IMAGE

FROM --platform=$BUILDPLATFORM rust:1-bullseye AS build
ARG TARGETARCH
WORKDIR /build
RUN mkdir -p /build/outputs
COPY . /build
RUN --mount=type=cache,id=platz-backend-cargo-target,target=/build/target,sharing=locked \
    --mount=type=cache,id=platz-backend-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=platz-backend-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    ./scripts/container-build.sh "$TARGETARCH" "/build/outputs"

FROM $BASE_IMAGE
WORKDIR /root/
COPY --from=build /build/outputs/* /root/
