#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOCKERFILE="${ROOT_DIR}/build-helpers/linux-packages.Dockerfile"
ARTIFACT_DIR="${ROOT_DIR}/dist/linux"
OUTPUT_BASE_DIR="${ROOT_DIR}/dist/linux/packages"
BUILDER_NAME="${BUILDER_NAME:-rtsp-webview-linux-builder}"

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

find_latest_tarball() {
    local tarballs=()

    shopt -s nullglob
    tarballs=("${ARTIFACT_DIR}"/rtsp-viewer-*.tar.gz)
    shopt -u nullglob

    if [[ ${#tarballs[@]} -eq 0 ]]; then
        fail "no Linux tarball found in ${ARTIFACT_DIR}; build one first with ./build-helpers/build-linux-tarball.sh"
    fi

    ls -t "${tarballs[@]}" | head -n 1
}

prompt_for_selection() {
    printf 'Using Linux tarball from dist/linux.\n' >&2
    printf 'Alpine/musl packaging is not available for the current Tauri/WebKit dependency stack.\n' >&2
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
        --file "${DOCKERFILE}" \
        --target "${docker_target}" \
        --build-arg "TARBALL_BASENAME=${tarball_basename}" \
        --build-context "linux_artifacts=${ARTIFACT_DIR}" \
        --output "type=local,dest=${output_dir}" \
        "${ROOT_DIR}"
}

build_selected_package() {
    local selection="$1"
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

    local tarball_path
    local tarball_basename
    local output_dir

    tarball_path="$(find_latest_tarball)"
    tarball_basename="$(basename "${tarball_path}")"
    output_dir="$(canonicalize_dir "${OUTPUT_BASE_DIR}/${output_subdir}")"

    command -v docker >/dev/null 2>&1 || fail "docker is required"
    ensure_builder

    printf 'Using tarball: %s\n' "${tarball_path}"
    printf 'Building %s into %s\n' "${label}" "${output_dir}"

    local docker_target=""
    for docker_target in "${docker_targets[@]}"; do
        run_docker_build "${docker_target}" "${tarball_basename}" "${output_dir}"
    done

    printf '%s created in %s\n' "${label}" "${output_dir}"
}

selection="${1:-}"
BUILD_PLATFORM="${BUILD_PLATFORM:-$(default_build_platform)}"

if [[ -z "${selection}" ]]; then
    selection="$(prompt_for_selection)"
fi

build_selected_package "${selection}"
