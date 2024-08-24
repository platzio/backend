#!/usr/bin/env bash

set -eu

HERE="$(realpath $(dirname "$0"))"
LOG_PREFIX="[$(basename $0)]"

OIDC_CONFIG_FILE="${HERE}/../.dev/oidc-config.json"
OIDC_USERS_FILE="${HERE}/../.dev/oidc-users.json"
export OIDC_PORT="9000"
export OIDC_SERVER_URL="http://localhost:${OIDC_PORT}"
export OIDC_CLIENT_ID="$(jq -r '.client_config[0].client_id' "${OIDC_CONFIG_FILE}")"
export OIDC_CLIENT_SECRET="$(jq -r '.client_config[0].client_secret' "${OIDC_CONFIG_FILE}")"
OIDC_CONTAINER="oidc-provider"

existing=$(docker ps -aq --no-trunc --filter name="${OIDC_CONTAINER}")
if [ -z "${existing}" ]
then
    echo "${LOG_PREFIX} ✨ Creating OIDC server container" >&2
    docker run -d \
           --name "${OIDC_CONTAINER}" \
           -p "${OIDC_PORT}":"${OIDC_PORT}" \
           -v "${OIDC_CONFIG_FILE}:/etc/oidc-config.json:ro" \
           -v "${OIDC_USERS_FILE}:/etc/oidc-users.json:ro" \
           -e "CONFIG_FILE=/etc/oidc-config.json" \
           -e "USERS_FILE=/etc/oidc-users.json" \
           qlik/simple-oidc-provider >&2
fi

if [ "$( docker container inspect -f '{{.State.Status}}' ${OIDC_CONTAINER} )" != "running" ]
then
    echo "${LOG_PREFIX} 🏁 Starting OIDC server container" >&2
    docker start "${OIDC_CONTAINER}" >&2
fi

i=1
while ! curl "${OIDC_SERVER_URL}" &>/dev/null
do
    echo "${LOG_PREFIX} 🕗 Waiting for OIDC server (${i})..."
    sleep 2
    i=$((i + 1))
done
