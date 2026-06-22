#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
ICON_SOURCE="${ROOT_DIR}/src-tauri/icons/icon.png"
DESKTOP_SOURCE="${ROOT_DIR}/build-helpers/linux/flatpak/org.archuser.rtspviewer.desktop"
METAINFO_SOURCE="${ROOT_DIR}/build-helpers/linux/flatpak/org.archuser.rtspviewer.metainfo.xml"
WRAPPER_SOURCE="${ROOT_DIR}/build-helpers/linux/flatpak/rtsp-viewer"
OUTPUT_DIR="${ROOT_DIR}/dist/flatpak"
BUILD_DIR="${OUTPUT_DIR}/build-dir"
REPO_DIR="${OUTPUT_DIR}/repo"
TARBALL_OUTPUT_DIR="${OUTPUT_DIR}/tarball-source"
FFMPEG_OUTPUT_DIR="${OUTPUT_DIR}/ffmpeg-source"
BRANCH="${BRANCH:-stable}"
APP_ID="org.archuser.rtspviewer"
APP_BINARY="rtsp_viewer_tauri"
APP_LAUNCHER="rtsp-viewer"
APP_TITLE="RTSP Viewer"
RUNTIME="org.gnome.Platform"
SDK="org.gnome.Sdk"
RUNTIME_BRANCH="${RUNTIME_BRANCH:-50}"

fail() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

require_command() {
    local command_name="$1"
    local label="$2"

    command -v "${command_name}" >/dev/null 2>&1 || fail "${label} was not found in PATH."
}

ensure_flatpak_ref() {
    local ref_name="$1"
    local branch="$2"

    if flatpak info "${ref_name}//${branch}" >/dev/null 2>&1; then
        return
    fi

    printf 'Installing missing Flatpak runtime %s//%s\n' "${ref_name}" "${branch}"
    flatpak install --user -y flathub "${ref_name}//${branch}"
}

map_flatpak_architecture() {
    case "${1:-}" in
        x86_64)
            tarball_arch="x86_64"
            ;;
        aarch64)
            tarball_arch="aarch64"
            ;;
        *)
            fail "unsupported Flatpak architecture '${1:-}'"
            ;;
    esac
}

extract_tarball_root() {
    local tarball_path="$1"
    local destination="$2"

    rm -rf "${destination}"
    mkdir -p "${destination}"
    tar -xzf "${tarball_path}" -C "${destination}"

    package_root="$(find "${destination}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    [[ -n "${package_root}" ]] || fail "failed to extract package root from ${tarball_path}"
}

require_command flatpak "Flatpak"
require_command docker "Docker"
require_command magick "ImageMagick"
require_command tar "tar"

[[ -f "${ICON_SOURCE}" ]] || fail "missing icon source at ${ICON_SOURCE}"
[[ -f "${DESKTOP_SOURCE}" ]] || fail "missing desktop file at ${DESKTOP_SOURCE}"
[[ -f "${METAINFO_SOURCE}" ]] || fail "missing metainfo file at ${METAINFO_SOURCE}"
[[ -f "${WRAPPER_SOURCE}" ]] || fail "missing launcher wrapper at ${WRAPPER_SOURCE}"

flatpak_arch="$(flatpak --default-arch)"
map_flatpak_architecture "${flatpak_arch}"

ensure_flatpak_ref "${RUNTIME}" "${RUNTIME_BRANCH}"
ensure_flatpak_ref "${SDK}" "${RUNTIME_BRANCH}"

echo "[1/6] Building Linux tarball in Docker..."
rm -rf "${TARBALL_OUTPUT_DIR}"
mkdir -p "${TARBALL_OUTPUT_DIR}"
DOCKER_BUILDKIT=1 docker build \
    --file "${ROOT_DIR}/build-helpers/linux/tarball.Dockerfile" \
    --target export \
    --output "type=local,dest=${TARBALL_OUTPUT_DIR}" \
    "${ROOT_DIR}"

latest_tarball="$(find "${TARBALL_OUTPUT_DIR}" -maxdepth 1 -type f -name "rtsp-viewer-*-linux-${tarball_arch}.tar.gz" | sort | tail -n 1)"
[[ -n "${latest_tarball}" ]] || fail "no tarball was produced for ${tarball_arch}"

rm -rf "${FFMPEG_OUTPUT_DIR}"
mkdir -p "${FFMPEG_OUTPUT_DIR}"
DOCKER_BUILDKIT=1 docker build \
    --file "${ROOT_DIR}/build-helpers/linux/tarball.Dockerfile" \
    --target flatpak-ffmpeg-export \
    --output "type=local,dest=${FFMPEG_OUTPUT_DIR}" \
    "${ROOT_DIR}"

workspace="$(mktemp -d)"
trap 'rm -rf "${workspace}"' EXIT

extract_tarball_root "${latest_tarball}" "${workspace}/package"
release_bin="${package_root}/${APP_BINARY}"
license_file="${package_root}/LICENSE"
readme_file="${package_root}/README-linux.txt"
resized_icon="${workspace}/org.archuser.rtspviewer-512.png"

[[ -f "${release_bin}" ]] || fail "tarball did not contain ${APP_BINARY}"
[[ -f "${license_file}" ]] || fail "tarball did not contain LICENSE"
[[ -f "${readme_file}" ]] || fail "tarball did not contain README-linux.txt"
[[ -x "${FFMPEG_OUTPUT_DIR}/ffmpeg/bin/ffmpeg" ]] || fail "Flatpak ffmpeg bundle did not contain ffmpeg"

magick "${ICON_SOURCE}" -resize 512x512 "${resized_icon}"
[[ -f "${resized_icon}" ]] || fail "failed to create resized Flatpak icon"

package_root_name="$(basename "${package_root}")"
case "${package_root_name}" in
    rtsp-viewer-*-linux-*)
        version="${package_root_name#rtsp-viewer-}"
        version="${version%-linux-*}"
        ;;
    *)
        fail "unexpected tarball directory name '${package_root_name}'"
        ;;
esac

bundle_path="${OUTPUT_DIR}/RTSP-Viewer-${version}-${flatpak_arch}.flatpak"

echo "[2/6] Preparing Flatpak filesystem..."
rm -rf "${BUILD_DIR}" "${REPO_DIR}"
mkdir -p "${OUTPUT_DIR}"

flatpak build-init \
    --arch="${flatpak_arch}" \
    "${BUILD_DIR}" \
    "${APP_ID}" \
    "${SDK}" \
    "${RUNTIME}" \
    "${RUNTIME_BRANCH}"

install -Dm0755 "${release_bin}" "${BUILD_DIR}/files/bin/${APP_BINARY}"
install -Dm0755 "${WRAPPER_SOURCE}" "${BUILD_DIR}/files/bin/${APP_LAUNCHER}"
install -Dm0644 "${DESKTOP_SOURCE}" "${BUILD_DIR}/files/share/applications/${APP_ID}.desktop"
install -Dm0644 "${METAINFO_SOURCE}" "${BUILD_DIR}/files/share/metainfo/${APP_ID}.metainfo.xml"
install -Dm0644 "${resized_icon}" "${BUILD_DIR}/files/share/icons/hicolor/512x512/apps/${APP_ID}.png"
install -Dm0644 "${license_file}" "${BUILD_DIR}/files/share/licenses/${APP_ID}/LICENSE"
install -Dm0644 "${readme_file}" "${BUILD_DIR}/files/share/doc/${APP_ID}/runtime-notes.txt"
mkdir -p "${BUILD_DIR}/files/libexec/ffmpeg"
cp -a "${FFMPEG_OUTPUT_DIR}/ffmpeg/." "${BUILD_DIR}/files/libexec/ffmpeg/"

echo "[3/6] Finalizing Flatpak metadata..."
flatpak build-finish \
    --command="${APP_LAUNCHER}" \
    --share=ipc \
    --share=network \
    --socket=fallback-x11 \
    --device=dri \
    --talk-name=org.freedesktop.secrets \
    "${BUILD_DIR}"

echo "[4/6] Exporting repository and bundle..."
mkdir -p "${REPO_DIR}"
flatpak build-export \
    --arch="${flatpak_arch}" \
    "${REPO_DIR}" \
    "${BUILD_DIR}" \
    "${BRANCH}"

rm -f "${bundle_path}"
flatpak build-bundle "${REPO_DIR}" "${bundle_path}" "${APP_ID}" "${BRANCH}"

echo "[5/6] Flatpak bundle ready."
echo "Product: ${APP_TITLE}"
echo "Bundle:"
echo "  ${bundle_path}"
echo
echo "[6/6] Install locally with:"
echo "  flatpak uninstall --user ${APP_ID}"
echo "  flatpak install --user ${bundle_path}"
