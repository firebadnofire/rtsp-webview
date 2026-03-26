#!/usr/bin/env bash

set -euo pipefail

: "${PACKAGE_TARGET:?PACKAGE_TARGET must be set}"
: "${TARBALL_BASENAME:?TARBALL_BASENAME must be set}"

INPUT_TARBALL="/input/dist/linux/${TARBALL_BASENAME}"
ICON_PATH="/input/icon.png"
OUT_DIR="/out"
APP_NAME="rtsp-viewer"
APP_TITLE="RTSP Viewer"
APP_BINARY="rtsp_viewer_tauri"
APP_URL="https://github.com/firebadnofire/rtsp-webview"
APP_DESCRIPTION="Desktop RTSP viewer packaged from the exported Linux tarball. Runtime still requires ffmpeg on PATH and a Secret Service compatible keyring daemon."

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

require_file() {
    local path="$1"

    [[ -f "${path}" ]] || fail "required file not found: ${path}"
}

create_wrapper() {
    local wrapper_path="$1"

    cat > "${wrapper_path}" <<'EOF'
#!/usr/bin/env sh
set -eu
exec /opt/rtsp-viewer/rtsp_viewer_tauri "$@"
EOF
    chmod 0755 "${wrapper_path}"
}

create_desktop_file() {
    local desktop_path="$1"
    local exec_target="$2"

    cat > "${desktop_path}" <<EOF
[Desktop Entry]
Type=Application
Name=${APP_TITLE}
Exec=${exec_target} %U
Icon=${APP_NAME}
Terminal=false
Categories=AudioVideo;Video;
EOF
}

prepare_stage_tree() {
    local stage_root="$1"

    mkdir -p \
        "${stage_root}/opt/${APP_NAME}" \
        "${stage_root}/usr/bin" \
        "${stage_root}/usr/share/applications" \
        "${stage_root}/usr/share/doc/${APP_NAME}" \
        "${stage_root}/usr/share/icons/hicolor/512x512/apps" \
        "${stage_root}/usr/share/licenses/${APP_NAME}"

    install -m 0755 "${package_root}/${APP_BINARY}" "${stage_root}/opt/${APP_NAME}/${APP_BINARY}"
    install -m 0644 "${package_root}/LICENSE" "${stage_root}/usr/share/licenses/${APP_NAME}/LICENSE"
    install -m 0644 "${package_root}/README-linux.txt" "${stage_root}/usr/share/doc/${APP_NAME}/README-linux.txt"
    install -m 0644 "${ICON_PATH}" "${stage_root}/usr/share/icons/hicolor/512x512/apps/${APP_NAME}.png"

    create_wrapper "${stage_root}/usr/bin/${APP_NAME}"
    create_desktop_file "${stage_root}/usr/share/applications/${APP_NAME}.desktop" "${APP_NAME}"
}

prepare_appdir() {
    local appdir="$1"

    mkdir -p \
        "${appdir}/usr/bin" \
        "${appdir}/usr/share/applications" \
        "${appdir}/usr/share/doc/${APP_NAME}" \
        "${appdir}/usr/share/icons/hicolor/512x512/apps"

    install -m 0755 "${package_root}/${APP_BINARY}" "${appdir}/usr/bin/${APP_BINARY}"
    install -m 0644 "${package_root}/LICENSE" "${appdir}/usr/share/doc/${APP_NAME}/LICENSE"
    install -m 0644 "${package_root}/README-linux.txt" "${appdir}/usr/share/doc/${APP_NAME}/README-linux.txt"
    install -m 0644 "${ICON_PATH}" "${appdir}/usr/share/icons/hicolor/512x512/apps/${APP_NAME}.png"

    create_desktop_file "${appdir}/usr/share/applications/${APP_NAME}.desktop" "AppRun"

    cat > "${appdir}/AppRun" <<'EOF'
#!/usr/bin/env sh
set -eu
SELF="$(readlink -f "$0")"
HERE="$(dirname "${SELF}")"
exec "${HERE}/usr/bin/rtsp_viewer_tauri" "$@"
EOF
    chmod 0755 "${appdir}/AppRun"

    cp "${appdir}/usr/share/applications/${APP_NAME}.desktop" "${appdir}/${APP_NAME}.desktop"
    cp "${appdir}/usr/share/icons/hicolor/512x512/apps/${APP_NAME}.png" "${appdir}/${APP_NAME}.png"
    ln -sf "${APP_NAME}.png" "${appdir}/.DirIcon"
}

map_architecture() {
    local arch="$1"

    deb_arch=""
    rpm_arch=""
    pacman_arch=""
    appimage_arch=""

    case "${arch}" in
        x86_64)
            deb_arch="amd64"
            rpm_arch="x86_64"
            pacman_arch="x86_64"
            appimage_arch="x86_64"
            ;;
        aarch64)
            deb_arch="arm64"
            rpm_arch="aarch64"
            pacman_arch="aarch64"
            appimage_arch="aarch64"
            ;;
        armv7|armv7l)
            deb_arch="armhf"
            rpm_arch="armv7hl"
            pacman_arch="armv7h"
            ;;
        ppc64le)
            deb_arch="ppc64el"
            rpm_arch="ppc64le"
            ;;
        s390x)
            deb_arch="s390x"
            rpm_arch="s390x"
            ;;
        *)
            fail "unsupported tarball architecture '${arch}'"
            ;;
    esac
}

require_supported_architecture() {
    local package_label="$1"
    local mapped_arch="$2"

    [[ -n "${mapped_arch}" ]] || fail "${package_label} packaging is not supported for tarball architecture '${linux_arch}'"
}

build_fpm_package() {
    local target_type="$1"
    local target_arch="$2"
    local output_file="$3"
    shift 3

    mkdir -p "${OUT_DIR}"

    fpm \
        -s dir \
        -t "${target_type}" \
        -C "${stage_root}" \
        -n "${APP_NAME}" \
        -v "${version}" \
        --iteration 1 \
        --architecture "${target_arch}" \
        --category "utils" \
        --maintainer "RTSP Viewer Contributors" \
        --vendor "RTSP Viewer Contributors" \
        --license "MIT" \
        --url "${APP_URL}" \
        --description "${APP_DESCRIPTION}" \
        --package "${OUT_DIR}/${output_file}" \
        "$@" \
        .
}

build_appimage() {
    local target_arch="$1"
    local tool_arch=""
    local tool_url=""
    local appdir="${workspace}/AppDir"

    case "$(uname -m)" in
        x86_64|amd64)
            tool_arch="x86_64"
            ;;
        arm64|aarch64)
            tool_arch="aarch64"
            ;;
        *)
            fail "unsupported build architecture '$(uname -m)' for appimagetool"
            ;;
    esac

    case "${tool_arch}" in
        x86_64)
            tool_url="https://github.com/AppImage/appimagetool/releases/download/${APPIMAGETOOL_RELEASE}/appimagetool-x86_64.AppImage"
            ;;
        aarch64)
            tool_url="https://github.com/AppImage/appimagetool/releases/download/${APPIMAGETOOL_RELEASE}/appimagetool-aarch64.AppImage"
            ;;
        *)
            fail "unsupported AppImage architecture '${tool_arch}'"
            ;;
    esac

    prepare_appdir "${appdir}"

    curl -fsSL "${tool_url}" -o /tmp/appimagetool.AppImage
    chmod 0755 /tmp/appimagetool.AppImage

    mkdir -p "${OUT_DIR}"

    ARCH="${target_arch}" /tmp/appimagetool.AppImage \
        --appimage-extract-and-run \
        "${appdir}" \
        "${OUT_DIR}/RTSP-Viewer-${version}-${target_arch}.AppImage"
}

require_file "${INPUT_TARBALL}"
require_file "${ICON_PATH}"

workspace="$(mktemp -d)"
trap 'rm -rf "${workspace}"' EXIT

tar -xzf "${INPUT_TARBALL}" -C "${workspace}"

package_root="$(find "${workspace}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
[[ -n "${package_root}" ]] || fail "failed to extract package root from ${INPUT_TARBALL}"

require_file "${package_root}/${APP_BINARY}"
require_file "${package_root}/LICENSE"
require_file "${package_root}/README-linux.txt"

package_root_name="$(basename "${package_root}")"

case "${package_root_name}" in
    rtsp-viewer-*-linux-*)
        version="${package_root_name#rtsp-viewer-}"
        version="${version%-linux-*}"
        linux_arch="${package_root_name##*-linux-}"
        ;;
    *)
        fail "unexpected tarball directory name '${package_root_name}'"
        ;;
esac

map_architecture "${linux_arch}"

stage_root="${workspace}/stage"
prepare_stage_tree "${stage_root}"

case "${PACKAGE_TARGET}" in
    deb)
        require_supported_architecture '.deb' "${deb_arch}"
        build_fpm_package \
            deb \
            "${deb_arch}" \
            "${APP_NAME}_${version}_${deb_arch}.deb" \
            --depends ffmpeg \
            --depends libgtk-3-0 \
            --depends libwebkit2gtk-4.0-37 \
            --depends libayatana-appindicator3-1
        ;;
    rpm-rhel)
        require_supported_architecture 'RPM' "${rpm_arch}"
        build_fpm_package \
            rpm \
            "${rpm_arch}" \
            "${APP_NAME}-${version}-1.el9.${rpm_arch}.rpm" \
            --rpm-os linux \
            --rpm-dist el9 \
            --depends ffmpeg \
            --depends gtk3 \
            --depends webkit2gtk3 \
            --depends libappindicator-gtk3
        ;;
    rpm-zypper)
        require_supported_architecture 'RPM' "${rpm_arch}"
        build_fpm_package \
            rpm \
            "${rpm_arch}" \
            "${APP_NAME}-${version}-1.opensuse.${rpm_arch}.rpm" \
            --rpm-os linux \
            --rpm-dist opensuse \
            --depends ffmpeg \
            --depends gtk3 \
            --depends webkit2gtk3-soup2 \
            --depends libappindicator3-1
        ;;
    arch)
        require_supported_architecture 'pacman' "${pacman_arch}"
        build_fpm_package \
            pacman \
            "${pacman_arch}" \
            "${APP_NAME}-${version}-1-${pacman_arch}.pkg.tar.zst" \
            --pacman-compression zstd \
            --depends ffmpeg \
            --depends gtk3 \
            --depends webkit2gtk \
            --depends libappindicator-gtk3
        ;;
    appimage)
        require_supported_architecture 'AppImage' "${appimage_arch}"
        build_appimage "${appimage_arch}"
        ;;
    *)
        fail "unsupported package target '${PACKAGE_TARGET}'"
        ;;
esac
