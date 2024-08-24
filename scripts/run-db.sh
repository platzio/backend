#!/bin/bash

set -eu

LOG_PREFIX="[$(basename $0)]"

export PGHOST="localhost"
export PGUSER="postgres"
export PGPORT="5432"
export PGDATABASE="test"
export PGPASSWORD="postgres"
export DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@127.0.0.1:${PGPORT}/${PGDATABASE}"
DB_CONTAINER="${PGDATABASE}-postgres"

if ps -ef | grep kubectl | grep port-forward | grep "${PGPORT}" &> /dev/null
then
    echo "${LOG_PREFIX} ⛔️ kubectl port-forward is running" >&2
    echo "${LOG_PREFIX}" >&2
    echo "${LOG_PREFIX} An active kubectl port-forward may potentially point to" >&2
    echo "${LOG_PREFIX} a production database." >&2
    echo "${LOG_PREFIX} Please turn it off first, then re-run this script." >&2
    exit 2
fi

if [ ! -z "${RM_DB:-}" ]
then
    echo "${LOG_PREFIX} 🪓 Deleting existing database container" >&2
    docker stop "${DB_CONTAINER}" >&2 || true
    docker rm "${DB_CONTAINER}" >&2 || true
fi

existing=$(docker ps -aq --no-trunc --filter name="${DB_CONTAINER}")
if [ -z "${existing}" ]
then
    echo "${LOG_PREFIX} ✨ Creating database container" >&2
    docker run -d \
           --name "${DB_CONTAINER}" \
           -p "${PGPORT}":"${PGPORT}" \
           -e POSTGRES_PASSWORD="${PGPASSWORD}" \
           -e POSTGRES_DB="${PGDATABASE}" \
           postgres >&2
fi

if [ "$( docker container inspect -f '{{.State.Status}}' ${DB_CONTAINER} )" != "running" ]
then
    echo "${LOG_PREFIX} 🏁 Starting database container" >&2
    docker start "${DB_CONTAINER}" >&2
fi
