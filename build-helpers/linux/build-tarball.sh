#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOCKERFILE="${ROOT_DIR}/build-helpers/linux/tarball.Dockerfile"
STATE_DIR="${ROOT_DIR}/.build-helpers-state"
OUTPUT_REGISTRY="${STATE_DIR}/linux-tarball-output-dirs"
OUTPUT_DIR="${OUTPUT_DIR:-${ROOT_DIR}/dist/linux}"
BUILDER_NAME="${BUILDER_NAME:-rtsp-webview-linux-builder}"
BUILD_NETWORK_NAME="${BUILD_NETWORK_NAME:-build-system}"
APT_CACHE_URL="${APT_CACHE_URL:-http://apt-cacher-ng:3142}"
APT_HTTP_PROXY=""

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

canonicalize_dir() {
    local dir="$1"

    mkdir -p "${dir}"
    (
        cd "${dir}"
        pwd
    )
}

default_build_platform() {
    case "$(uname -m)" in
        arm64|aarch64)
            printf '%s\n' 'linux/arm64'
            ;;
        armv7l|armv7|armhf)
            printf '%s\n' 'linux/arm/v7'
            ;;
        ppc64le|powerpc64le)
            printf '%s\n' 'linux/ppc64le'
            ;;
        s390x)
            printf '%s\n' 'linux/s390x'
            ;;
        x86_64|amd64)
            printf '%s\n' 'linux/amd64'
            ;;
        *)
            printf '%s\n' 'linux/amd64'
            ;;
    esac
}

normalize_build_platform() {
    local selection="${1:-}"

    case "${selection}" in
        1|amd64|x86_64|linux/amd64)
            printf '%s\n' 'linux/amd64'
            ;;
        2|arm64|aarch64|linux/arm64)
            printf '%s\n' 'linux/arm64'
            ;;
        3|armv7|armhf|linux/arm/v7)
            printf '%s\n' 'linux/arm/v7'
            ;;
        4|ppc64le|linux/ppc64le)
            printf '%s\n' 'linux/ppc64le'
            ;;
        5|s390x|linux/s390x)
            printf '%s\n' 'linux/s390x'
            ;;
        6|q|quit|exit)
            printf '%s\n' 'quit'
            ;;
        '')
            fail 'no Linux architecture selection was provided'
            ;;
        *)
            fail "unsupported Linux build architecture '${selection}'"
            ;;
    esac
}

prompt_for_build_platform() {
    local selection=""

    printf 'Select a Linux build architecture:\n' >&2
    printf '\n' >&2
    printf '1. AMD64 (linux/amd64, x86_64)\n' >&2
    printf '2. ARM64 (linux/arm64, aarch64)\n' >&2
    printf '3. ARMv7 (linux/arm/v7)\n' >&2
    printf '4. PPC64LE (linux/ppc64le)\n' >&2
    printf '5. s390x (linux/s390x)\n' >&2
    printf '6. Quit\n' >&2
    printf '\n' >&2

    read -r -p 'Select a build architecture: ' selection
    normalize_build_platform "${selection}"
}

select_build_platform() {
    if [[ -n "${BUILD_PLATFORM:-}" ]]; then
        normalize_build_platform "${BUILD_PLATFORM}"
        return
    fi

    if [[ -t 0 && -t 2 ]]; then
        prompt_for_build_platform
        return
    fi

    default_build_platform
}

ensure_docker_command() {
    command -v docker >/dev/null 2>&1 || fail 'docker is required'
}

ensure_build_network() {
    if ! docker network inspect "${BUILD_NETWORK_NAME}" >/dev/null 2>&1; then
        docker network create "${BUILD_NETWORK_NAME}" >/dev/null
    fi
}

probe_apt_cache() {
    if docker run --rm --network "${BUILD_NETWORK_NAME}" busybox:1.36.1 \
        wget -q -T 2 -O /dev/null "${APT_CACHE_URL}" >/dev/null 2>&1; then
        printf '%s\n' "${APT_CACHE_URL}"
        return
    fi

    printf '%s\n' ''
}

cleanup_builder() {
    if command -v docker >/dev/null 2>&1 && docker buildx inspect "${BUILDER_NAME}" >/dev/null 2>&1; then
        docker buildx rm --force "${BUILDER_NAME}" >/dev/null 2>&1 || true
    fi
}

ensure_builder() {
    if docker buildx inspect "${BUILDER_NAME}" >/dev/null 2>&1; then
        if ! docker buildx inspect "${BUILDER_NAME}" 2>/dev/null | grep -Fq "network=${BUILD_NETWORK_NAME}"; then
            docker buildx rm --force "${BUILDER_NAME}" >/dev/null 2>&1 || true
        fi
    fi

    if ! docker buildx inspect "${BUILDER_NAME}" >/dev/null 2>&1; then
        docker buildx create \
            --name "${BUILDER_NAME}" \
            --driver docker-container \
            --driver-opt "network=${BUILD_NETWORK_NAME}" \
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
ensure_docker_command
ensure_build_network
APT_HTTP_PROXY="$(probe_apt_cache)"
BUILD_PLATFORM="$(select_build_platform)"

if [[ "${BUILD_PLATFORM}" == "quit" ]]; then
    printf 'No Linux tarball built.\n'
    exit 0
fi

trap cleanup_builder EXIT
ensure_builder

if [[ -n "${APT_HTTP_PROXY}" ]]; then
    printf 'Using apt proxy %s\n' "${APT_HTTP_PROXY}"
else
    printf 'No apt proxy detected at %s\n' "${APT_CACHE_URL}"
fi

printf 'Building Linux tarball for %s into %s\n' "${BUILD_PLATFORM}" "${OUTPUT_DIR}"

docker buildx build \
    --builder "${BUILDER_NAME}" \
    --platform "${BUILD_PLATFORM}" \
    --network "${BUILD_NETWORK_NAME}" \
    --file "${DOCKERFILE}" \
    --target export \
    --build-arg "APT_HTTP_PROXY=${APT_HTTP_PROXY}" \
    --output "type=local,dest=${OUTPUT_DIR}" \
    "$@" \
    "${ROOT_DIR}"

record_output_dir "${OUTPUT_DIR}"

echo "Linux tarball exported to ${OUTPUT_DIR}"
