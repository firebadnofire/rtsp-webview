#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOCKERFILE="${ROOT_DIR}/build-helpers/linux/tarball.Dockerfile"
STATE_DIR="${ROOT_DIR}/.build-helpers-state"
OUTPUT_REGISTRY="${STATE_DIR}/linux-tarball-output-dirs"
BUILD_PLATFORM="${BUILD_PLATFORM:-linux/amd64}"
OUTPUT_DIR="${OUTPUT_DIR:-${ROOT_DIR}/dist/linux}"
BUILDER_NAME="${BUILDER_NAME:-rtsp-webview-linux-builder}"

canonicalize_dir() {
    local dir="$1"

    mkdir -p "${dir}"
    (
        cd "${dir}"
        pwd
    )
}

ensure_builder() {
    if ! docker buildx inspect "${BUILDER_NAME}" >/dev/null 2>&1; then
        docker buildx create \
            --name "${BUILDER_NAME}" \
            --driver docker-container \
            >/dev/null
    fi

    docker buildx inspect \
        --builder "${BUILDER_NAME}" \
        --bootstrap \
        >/dev/null
}

record_output_dir() {
    local output_dir="$1"

    mkdir -p "${STATE_DIR}"
    touch "${OUTPUT_REGISTRY}"

    if ! grep -Fqx "${output_dir}" "${OUTPUT_REGISTRY}"; then
        printf '%s\n' "${output_dir}" >> "${OUTPUT_REGISTRY}"
    fi
}

if [[ $# -gt 0 && "${1}" != -* ]]; then
    OUTPUT_DIR="$1"
    shift
fi

OUTPUT_DIR="$(canonicalize_dir "${OUTPUT_DIR}")"

ensure_builder

docker buildx build \
    --builder "${BUILDER_NAME}" \
    --platform "${BUILD_PLATFORM}" \
    --file "${DOCKERFILE}" \
    --target export \
    --output "type=local,dest=${OUTPUT_DIR}" \
    "$@" \
    "${ROOT_DIR}"

record_output_dir "${OUTPUT_DIR}"

echo "Linux tarball exported to ${OUTPUT_DIR}"
