#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOCKERFILE="${ROOT_DIR}/build-helpers/linux/packages.Dockerfile"
ARTIFACT_DIR="${ROOT_DIR}/dist/linux"
OUTPUT_BASE_DIR="${ROOT_DIR}/dist/linux/packages"
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
        x86_64|amd64)
            printf '%s\n' 'linux/amd64'
            ;;
        *)
            printf '%s\n' 'linux/amd64'
            ;;
    esac
}

default_tarball_architecture() {
    case "$(uname -m)" in
        arm64|aarch64)
            printf '%s\n' 'aarch64'
            ;;
        armv7l|armv7|armhf)
            printf '%s\n' 'armv7'
            ;;
        ppc64le|powerpc64le)
            printf '%s\n' 'ppc64le'
            ;;
        s390x)
            printf '%s\n' 's390x'
            ;;
        x86_64|amd64)
            printf '%s\n' 'x86_64'
            ;;
        *)
            printf '%s\n' 'x86_64'
            ;;
    esac
}

normalize_tarball_architecture() {
    local selection="${1:-}"

    case "${selection}" in
        1|amd64|x64|x86_64|linux/amd64)
            printf '%s\n' 'x86_64'
            ;;
        2|arm64|aarch64|linux/arm64)
            printf '%s\n' 'aarch64'
            ;;
        3|armv7|armv7l|armhf|linux/arm/v7)
            printf '%s\n' 'armv7'
            ;;
        4|ppc64le|linux/ppc64le)
            printf '%s\n' 'ppc64le'
            ;;
        5|s390x|linux/s390x)
            printf '%s\n' 's390x'
            ;;
        6|q|quit|exit)
            printf '%s\n' 'quit'
            ;;
        '')
            fail 'no tarball architecture selection was provided'
            ;;
        *)
            fail "invalid architecture selection '${selection}'"
            ;;
    esac
}

is_architecture_token() {
    case "${1:-}" in
        1|2|3|4|5|6|amd64|x64|x86_64|linux/amd64|arm64|aarch64|linux/arm64|armv7|armv7l|armhf|linux/arm/v7|ppc64le|linux/ppc64le|s390x|linux/s390x|q|quit|exit)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

is_package_token() {
    case "${1:-}" in
        1|2|3|4|5|deb|rpm|rpm-rhel|rhel|rpm-zypper|zypper|opensuse|suse|arch|pacman|appimage|q|quit|exit)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

is_interactive_terminal() {
    [[ -t 0 && -t 2 ]]
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

find_latest_tarball() {
    local tarball_arch="$1"
    local tarballs=()

    shopt -s nullglob
    tarballs=("${ARTIFACT_DIR}"/rtsp-viewer-*-linux-"${tarball_arch}".tar.gz)
    shopt -u nullglob

    if [[ ${#tarballs[@]} -eq 0 ]]; then
        fail "no Linux tarball for architecture '${tarball_arch}' found in ${ARTIFACT_DIR}; build one first with ./build-helpers/linux/build-tarball.sh"
    fi

    ls -t "${tarballs[@]}" | head -n 1
}

prompt_for_architecture() {
    local selection=""

    printf 'Using Linux tarballs from dist/linux.\n' >&2
    printf 'Alpine/musl packaging is not available for the current Tauri/WebKit dependency stack.\n' >&2
    printf '\n' >&2
    printf '1. AMD64 (x86_64)\n' >&2
    printf '2. ARM64 (aarch64)\n' >&2
    printf '3. ARMv7\n' >&2
    printf '4. PPC64LE\n' >&2
    printf '5. s390x\n' >&2
    printf '6. Quit\n' >&2
    printf '\n' >&2

    read -r -p 'Select a tarball architecture: ' selection
    printf '%s' "${selection}"
}

prompt_for_selection() {
    local tarball_arch="$1"

    printf 'Selected tarball architecture: %s\n' "${tarball_arch}" >&2
    printf '\n' >&2
    printf 'Using Linux tarball from dist/linux.\n' >&2
    printf '\n' >&2
    printf '1. Build a .deb\n' >&2
    printf '2. Build RPM packages\n' >&2
    printf '3. Build an Arch package\n' >&2
    printf '4. Build an AppImage\n' >&2
    printf '5. Quit\n' >&2
    printf '\n' >&2

    read -r -p 'Select a package target: ' selection
    printf '%s' "${selection}"
}

run_docker_build() {
    local docker_target="$1"
    local tarball_basename="$2"
    local output_dir="$3"

    docker buildx build \
        --builder "${BUILDER_NAME}" \
        --platform "${BUILD_PLATFORM}" \
        --network "${BUILD_NETWORK_NAME}" \
        --file "${DOCKERFILE}" \
        --target "${docker_target}" \
        --build-arg "APT_HTTP_PROXY=${APT_HTTP_PROXY}" \
        --build-arg "TARBALL_BASENAME=${tarball_basename}" \
        --build-context "linux_artifacts=${ARTIFACT_DIR}" \
        --output "type=local,dest=${output_dir}" \
        "${ROOT_DIR}"
}

package_supported_for_architecture() {
    local selection="$1"
    local tarball_arch="$2"

    case "${selection}" in
        1|deb)
            return 0
            ;;
        2|rpm|rpm-rhel|rhel|rpm-zypper|zypper|opensuse|suse)
            return 0
            ;;
        3|arch|pacman)
            case "${tarball_arch}" in
                x86_64|aarch64|armv7)
                    return 0
                    ;;
                *)
                    return 1
                    ;;
            esac
            ;;
        4|appimage)
            case "${tarball_arch}" in
                x86_64|aarch64)
                    return 0
                    ;;
                *)
                    return 1
                    ;;
            esac
            ;;
        5|q|quit|exit)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

build_selected_package() {
    local selection="$1"
    local tarball_arch="$2"
    local output_subdir=""
    local label=""
    local -a docker_targets=()

    case "${selection}" in
        1|deb)
            docker_targets=("export-deb")
            output_subdir="deb"
            label=".deb package"
            ;;
        2|rpm|rpm-rhel|rhel|rpm-zypper|zypper|opensuse|suse)
            docker_targets=("export-rpm-rhel" "export-rpm-zypper")
            output_subdir="rpm"
            label="RPM packages"
            ;;
        3|arch|pacman)
            docker_targets=("export-arch")
            output_subdir="arch"
            label="Arch package"
            ;;
        4|appimage)
            docker_targets=("export-appimage")
            output_subdir="appimage"
            label="AppImage"
            ;;
        5|q|quit|exit)
            printf 'No package built.\n'
            exit 0
            ;;
        *)
            fail "invalid selection '${selection}'"
            ;;
    esac

    if ! package_supported_for_architecture "${selection}" "${tarball_arch}"; then
        fail "package target '${selection}' is not supported for tarball architecture '${tarball_arch}'"
    fi

    local tarball_path
    local tarball_basename
    local output_dir

    tarball_path="$(find_latest_tarball "${tarball_arch}")"
    tarball_basename="$(basename "${tarball_path}")"
    output_dir="$(canonicalize_dir "${OUTPUT_BASE_DIR}/${output_subdir}")"

    ensure_builder

    printf 'Using tarball: %s\n' "${tarball_path}"
    printf 'Building %s into %s\n' "${label}" "${output_dir}"

    local docker_target=""
    for docker_target in "${docker_targets[@]}"; do
        run_docker_build "${docker_target}" "${tarball_basename}" "${output_dir}"
    done

    printf '%s created in %s\n' "${label}" "${output_dir}"
}

tarball_arch_selection=""
selection=""
BUILD_PLATFORM="${BUILD_PLATFORM:-$(default_build_platform)}"

ensure_docker_command
ensure_build_network
APT_HTTP_PROXY="$(probe_apt_cache)"

if [[ -n "${APT_HTTP_PROXY}" ]]; then
    printf 'Using apt proxy %s\n' "${APT_HTTP_PROXY}"
else
    printf 'No apt proxy detected at %s\n' "${APT_CACHE_URL}"
fi

for arg in "$@"; do
    if [[ -z "${tarball_arch_selection}" ]] && is_architecture_token "${arg}"; then
        tarball_arch_selection="${arg}"
        continue
    fi

    if [[ -z "${selection}" ]] && is_package_token "${arg}"; then
        selection="${arg}"
        continue
    fi

    fail "unrecognized or duplicate argument '${arg}'"
done

if [[ -z "${tarball_arch_selection}" ]]; then
    if is_interactive_terminal; then
        tarball_arch_selection="$(prompt_for_architecture)"
    else
        tarball_arch_selection="$(default_tarball_architecture)"
    fi
fi

tarball_arch="$(normalize_tarball_architecture "${tarball_arch_selection}")"
if [[ "${tarball_arch}" == 'quit' ]]; then
    printf 'No package built.\n'
    exit 0
fi

if [[ -z "${selection}" ]]; then
    if is_interactive_terminal; then
        selection="$(prompt_for_selection "${tarball_arch}")"
    else
        fail "no package target was provided in non-interactive mode; pass deb, rpm, arch, or appimage"
    fi
fi

trap cleanup_builder EXIT
build_selected_package "${selection}" "${tarball_arch}"
