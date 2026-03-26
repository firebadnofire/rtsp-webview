#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STATE_DIR="${ROOT_DIR}/.build-helpers-state"
OUTPUT_REGISTRY="${STATE_DIR}/linux-tarball-output-dirs"
BUILDER_NAME="${BUILDER_NAME:-rtsp-webview-linux-builder}"
BUILD_NETWORK_NAME="${BUILD_NETWORK_NAME:-build-system}"

static_paths_to_clean=(
    "${ROOT_DIR}/target"
    "${ROOT_DIR}/dist"
    "${ROOT_DIR}/ui/dist"
    "${ROOT_DIR}/ui/node_modules"
    "${ROOT_DIR}/ui/.vite"
    "${ROOT_DIR}/coverage"
    "${ROOT_DIR}/ui/coverage"
    "${ROOT_DIR}/vendor"
    "${STATE_DIR}"
)

path_size_kib() {
    local path="$1"

    if [[ ! -e "${path}" ]]; then
        echo 0
        return
    fi

    du -sk "${path}" 2>/dev/null | awk '{ print $1 }'
}

format_kib() {
    local kib="$1"

    awk -v kib="${kib}" '
        BEGIN {
            split("B KB MB GB TB PB", units, " ")
            size = kib * 1024
            unit = 1

            while (size >= 1024 && unit < length(units)) {
                size /= 1024
                unit++
            }

            if (unit == 1) {
                printf "%d %s", size, units[unit]
            } else {
                printf "%.1f %s", size, units[unit]
            }
        }
    '
}

human_size_to_kib() {
    local size="$1"
    local value
    local unit

    size="$(printf '%s' "${size}" | tr -d '[:space:]')"

    if [[ -z "${size}" || "${size}" == "0" ]]; then
        echo 0
        return
    fi

    value="$(printf '%s' "${size}" | sed -E 's/^([0-9.]+)(B|KB|MB|GB|TB|PB)$/\1/')"
    unit="$(printf '%s' "${size}" | sed -E 's/^([0-9.]+)(B|KB|MB|GB|TB|PB)$/\2/')"

    if [[ -z "${value}" || -z "${unit}" || "${value}" == "${size}" || "${unit}" == "${size}" ]]; then
        echo 0
        return
    fi

    awk -v value="${value}" -v unit="${unit}" '
        BEGIN {
            if (unit == "B") {
                print int((value + 1023) / 1024)
            } else if (unit == "KB") {
                print int(value + 0.999999)
            } else if (unit == "MB") {
                print int((value * 1024) + 0.999999)
            } else if (unit == "GB") {
                print int((value * 1024 * 1024) + 0.999999)
            } else if (unit == "TB") {
                print int((value * 1024 * 1024 * 1024) + 0.999999)
            } else if (unit == "PB") {
                print int((value * 1024 * 1024 * 1024 * 1024) + 0.999999)
            } else {
                print 0
            }
        }
    '
}

docker_builder_size_kib() {
    if ! command -v docker >/dev/null 2>&1; then
        echo 0
        return
    fi

    if ! docker buildx inspect "${BUILDER_NAME}" >/dev/null 2>&1; then
        echo 0
        return
    fi

    local reclaimable

    reclaimable="$(
        docker buildx du --builder "${BUILDER_NAME}" 2>/dev/null \
            | awk '
                /^Reclaimable:/ {
                    print $2
                    exit
                }
            '
    )"

    human_size_to_kib "${reclaimable:-0}"
}

is_safe_to_remove() {
    local path="$1"

    [[ -n "${path}" ]] || return 1
    [[ "${path}" != "/" ]] || return 1
    [[ "${path}" != "${HOME:-}" ]] || return 1
}

gather_paths_to_clean() {
    local path

    for path in "${static_paths_to_clean[@]}"; do
        printf '%s\n' "${path}"
    done

    if [[ -f "${OUTPUT_REGISTRY}" ]]; then
        while IFS= read -r path; do
            [[ -n "${path}" ]] || continue
            printf '%s\n' "${path}"
        done < "${OUTPUT_REGISTRY}"
    fi
}

total_kib=0
declare -A seen_paths=()
paths_to_clean=()

while IFS= read -r path; do
    [[ -n "${path}" ]] || continue
    if [[ -n "${seen_paths[${path}]+x}" ]]; then
        continue
    fi

    seen_paths["${path}"]=1
    paths_to_clean+=("${path}")
done < <(gather_paths_to_clean)

for path in "${paths_to_clean[@]}"; do
    total_kib=$((total_kib + $(path_size_kib "${path}")))
done

docker_kib="$(docker_builder_size_kib)"
total_kib=$((total_kib + docker_kib))

for path in "${paths_to_clean[@]}"; do
    if is_safe_to_remove "${path}"; then
        rm -rf "${path}"
    fi
done

if command -v docker >/dev/null 2>&1 && docker buildx inspect "${BUILDER_NAME}" >/dev/null 2>&1; then
    docker buildx prune --builder "${BUILDER_NAME}" --all --force >/dev/null 2>&1 || true
    docker buildx rm --force "${BUILDER_NAME}" >/dev/null 2>&1 || true
fi

if command -v docker >/dev/null 2>&1 && docker network inspect "${BUILD_NETWORK_NAME}" >/dev/null 2>&1; then
    docker network rm "${BUILD_NETWORK_NAME}" >/dev/null 2>&1 || true
fi

echo "$(format_kib "${total_kib}") removed"
