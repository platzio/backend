#!/usr/bin/env bash

set -eu

HERE="$(realpath $(dirname "$0"))"
LOG_PREFIX="[$(basename $0)]"

OIDC_CONFIG_FILE="${HERE}/dex.config.yaml"
export OIDC_PORT="5556"
export OIDC_SERVER_URL="http://127.0.0.1:${OIDC_PORT}"
export OIDC_CLIENT_ID="$(yq '.staticClients[0].id' "${OIDC_CONFIG_FILE}")"
export OIDC_CLIENT_SECRET="$(yq '.staticClients[0].secret' "${OIDC_CONFIG_FILE}")"
OIDC_CONTAINER="oidc-provider"

existing=$(docker ps -aq --no-trunc --filter name="${OIDC_CONTAINER}")
if [ ! -z "${existing}" ]
then
    docker stop "${OIDC_CONTAINER}" || true
    docker rm "${OIDC_CONTAINER}"
fi

echo "${LOG_PREFIX} ✨ Creating OIDC server" >&2
docker run -d \
        --name "${OIDC_CONTAINER}" \
        -p "${OIDC_PORT}":"${OIDC_PORT}" \
        -v "${OIDC_CONFIG_FILE}:/etc/dex/config.docker.yaml:ro" \
        dexidp/dex >&2
