ARG BASE_IMAGE

FROM --platform=$BUILDPLATFORM rust:1 AS build
ARG RELEASE_BUILD=1
ARG TARGETARCH
WORKDIR /build
RUN mkdir -p /build/outputs
COPY . /build
RUN --mount=type=cache,id=platz-backend-cargo-target,target=/build/target,sharing=locked \
    --mount=type=cache,id=platz-backend-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=platz-backend-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    ./scripts/container-build.sh "${TARGETARCH}" "${RELEASE_BUILD}" "/build/outputs"

FROM $BASE_IMAGE
WORKDIR /root/
COPY --from=build /build/outputs/* /root/
