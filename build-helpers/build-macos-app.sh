#!/usr/bin/env bash

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    printf 'ERROR: build-macos-app.sh only runs on macOS.\n' >&2
    exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UI_DIR="${ROOT_DIR}/ui"
TAURI_CONFIG="${ROOT_DIR}/src-tauri/tauri.conf.json"
ICON_SOURCE="${ROOT_DIR}/src-tauri/icons/icon.png"
OUTPUT_DIR="${ROOT_DIR}/dist/macos"
RELEASE_BIN="${ROOT_DIR}/target/release/rtsp_viewer_tauri"

fail() {
    printf 'ERROR: %s\n' "$*" >&2
    exit 1
}

require_command() {
    local command_name="$1"
    local label="$2"

    command -v "${command_name}" >/dev/null 2>&1 || fail "${label} was not found in PATH."
}

read_tauri_field() {
    local expression="$1"

    node -e '
const fs = require("fs");
const config = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const value = Function("config", "return " + process.argv[2])(config);
if (typeof value !== "string" || value.length === 0) {
  process.exit(1);
}
process.stdout.write(value);
' "${TAURI_CONFIG}" "${expression}" || fail "failed to read ${expression} from ${TAURI_CONFIG}"
}

create_app_icon() {
    local destination_path="$1"
    local temp_dir
    local iconset_dir

    temp_dir="$(mktemp -d)"
    iconset_dir="${temp_dir}/rtsp-viewer.iconset"
    mkdir -p "${iconset_dir}"
    trap 'rm -rf "${temp_dir}"' RETURN

    sips -z 16 16 "${ICON_SOURCE}" --out "${iconset_dir}/icon_16x16.png" >/dev/null
    sips -z 32 32 "${ICON_SOURCE}" --out "${iconset_dir}/icon_16x16@2x.png" >/dev/null
    sips -z 32 32 "${ICON_SOURCE}" --out "${iconset_dir}/icon_32x32.png" >/dev/null
    sips -z 64 64 "${ICON_SOURCE}" --out "${iconset_dir}/icon_32x32@2x.png" >/dev/null
    sips -z 128 128 "${ICON_SOURCE}" --out "${iconset_dir}/icon_128x128.png" >/dev/null
    sips -z 256 256 "${ICON_SOURCE}" --out "${iconset_dir}/icon_128x128@2x.png" >/dev/null
    sips -z 256 256 "${ICON_SOURCE}" --out "${iconset_dir}/icon_256x256.png" >/dev/null
    sips -z 512 512 "${ICON_SOURCE}" --out "${iconset_dir}/icon_256x256@2x.png" >/dev/null
    sips -z 512 512 "${ICON_SOURCE}" --out "${iconset_dir}/icon_512x512.png" >/dev/null
    cp "${ICON_SOURCE}" "${iconset_dir}/icon_512x512@2x.png"

    iconutil -c icns "${iconset_dir}" -o "${destination_path}"
}

require_command node "Node.js"
require_command npm "npm"
require_command cargo "Rust"
require_command rustup "rustup"
require_command codesign "codesign"

[[ -f "${TAURI_CONFIG}" ]] || fail "missing Tauri config at ${TAURI_CONFIG}"
[[ -f "${ICON_SOURCE}" ]] || fail "missing icon source at ${ICON_SOURCE}"

product_name="$(read_tauri_field "config.package.productName")"
bundle_identifier="$(read_tauri_field "config.tauri.bundle.identifier")"
version="$(read_tauri_field "config.package.version")"
bundle_icon_name="rtsp-viewer"
bundle_icon_plist=""

app_dir="${OUTPUT_DIR}/${product_name}.app"
contents_dir="${app_dir}/Contents"
macos_dir="${contents_dir}/MacOS"
resources_dir="${contents_dir}/Resources"
icon_destination="${resources_dir}/${bundle_icon_name}.icns"

echo "[1/5] Checking Rust toolchain..."
rustup default >/dev/null 2>&1 || fail "No default Rust toolchain is configured."

echo "[2/5] Installing frontend dependencies..."
(
    cd "${UI_DIR}"
    npm ci
) || fail "npm ci failed."

echo "[3/5] Building frontend bundle..."
(
    cd "${UI_DIR}"
    npm run build
) || fail "frontend build failed."

echo "[4/5] Building macOS release binary..."
(
    cd "${ROOT_DIR}"
    cargo build --locked --release -p rtsp_viewer_tauri
) || fail "Rust release build failed. Install Xcode Command Line Tools if the linker is missing."

[[ -f "${RELEASE_BIN}" ]] || fail "build finished but ${RELEASE_BIN} was not found"

echo "[5/5] Assembling macOS app bundle..."
mkdir -p "${OUTPUT_DIR}"
rm -rf "${app_dir}"
mkdir -p "${macos_dir}" "${resources_dir}"

cp "${RELEASE_BIN}" "${macos_dir}/rtsp_viewer_tauri"
chmod 0755 "${macos_dir}/rtsp_viewer_tauri"

if command -v sips >/dev/null 2>&1 && command -v iconutil >/dev/null 2>&1; then
    if create_app_icon "${icon_destination}"; then
        bundle_icon_plist="  <key>CFBundleIconFile</key>
  <string>${bundle_icon_name}</string>"
    else
        printf 'WARNING: failed to generate a macOS .icns file; continuing without a custom app icon.\n' >&2
    fi
else
    printf 'WARNING: sips or iconutil was not found; continuing without a custom app icon.\n' >&2
fi

install -m 0644 "${ROOT_DIR}/LICENSE" "${resources_dir}/LICENSE"

cat > "${contents_dir}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${product_name}</string>
  <key>CFBundleExecutable</key>
  <string>rtsp_viewer_tauri</string>
${bundle_icon_plist}
  <key>CFBundleIdentifier</key>
  <string>${bundle_identifier}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${product_name}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${version}</string>
  <key>CFBundleVersion</key>
  <string>${version}</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

printf 'APPL????' > "${contents_dir}/PkgInfo"

codesign --force --deep --sign - "${app_dir}" >/dev/null
codesign --verify --deep --strict "${app_dir}" >/dev/null

echo "Build complete."
echo "Output app bundle:"
echo "  ${app_dir}"
