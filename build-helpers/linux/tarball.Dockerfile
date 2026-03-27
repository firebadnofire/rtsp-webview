# syntax=docker/dockerfile:1.7

FROM node:20-bullseye AS toolchain

ARG APT_HTTP_PROXY=""

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:${PATH}

RUN set -eux; \
    if [ -n "${APT_HTTP_PROXY}" ]; then \
        printf 'Acquire::http::Proxy "%s";\n' "${APT_HTTP_PROXY}" > /etc/apt/apt.conf.d/01proxy; \
    else \
        rm -f /etc/apt/apt.conf.d/01proxy; \
    fi; \
    apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    file \
    libayatana-appindicator3-dev \
    libgtk-3-dev \
    libssl-dev \
    libwebkit2gtk-4.0-dev \
    librsvg2-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable

WORKDIR /work

FROM toolchain AS build

COPY Cargo.toml /work/Cargo.toml
COPY Cargo.lock /work/Cargo.lock
COPY crates/config/Cargo.toml /work/crates/config/Cargo.toml
COPY crates/core/Cargo.toml /work/crates/core/Cargo.toml
COPY crates/media/Cargo.toml /work/crates/media/Cargo.toml
COPY crates/secrets/Cargo.toml /work/crates/secrets/Cargo.toml
COPY src-tauri/Cargo.toml /work/src-tauri/Cargo.toml
COPY ui/package.json /work/ui/package.json
COPY ui/package-lock.json /work/ui/package-lock.json

RUN --mount=type=cache,target=/root/.npm,sharing=locked \
    cd /work/ui && npm ci

RUN set -eux; \
    mkdir -p \
        /work/crates/config/src \
        /work/crates/core/src \
        /work/crates/media/src \
        /work/crates/secrets/src \
        /work/src-tauri/src; \
    printf 'pub fn placeholder() {}\n' > /work/crates/config/src/lib.rs; \
    printf 'pub fn placeholder() {}\n' > /work/crates/core/src/lib.rs; \
    printf 'pub fn placeholder() {}\n' > /work/crates/media/src/lib.rs; \
    printf 'pub fn placeholder() {}\n' > /work/crates/secrets/src/lib.rs; \
    printf 'fn main() {}\n' > /work/src-tauri/src/main.rs; \
    printf 'fn main() {}\n' > /work/src-tauri/build.rs

RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git/db,sharing=locked \
    set -eux; \
    cargo fetch --locked --manifest-path /work/src-tauri/Cargo.toml; \
    wry_src="$(find /usr/local/cargo/registry/src -maxdepth 2 -type d -name 'wry-*' | head -n 1)"; \
    test -n "${wry_src}"; \
    wry_webkitgtk_mod="${wry_src}/src/webview/webkitgtk/mod.rs"; \
    if ! grep -q 'SettingsExt' "${wry_webkitgtk_mod}"; then \
        sed -i 's/PolicyDecisionType, /PolicyDecisionType, SettingsExt, /' "${wry_webkitgtk_mod}"; \
    fi

COPY . /work

RUN --mount=type=cache,target=/root/.npm,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git/db,sharing=locked \
    --mount=type=cache,target=/work/target,sharing=locked \
    set -eux; \
    cd /work/ui; \
    npm run build; \
    cd /work; \
    cargo build --locked --release -p rtsp_viewer_tauri; \
    install -m 0755 /work/target/release/rtsp_viewer_tauri /tmp/rtsp_viewer_tauri; \
    cargo pkgid -p rtsp_viewer_tauri | sed 's/.*@//' > /tmp/rtsp-viewer-version

FROM debian:bullseye-slim AS package

ARG TARGETARCH
ARG TARGETVARIANT

WORKDIR /out

COPY --from=build /tmp/rtsp-viewer-version /tmp/rtsp-viewer-version
COPY --from=build /work/LICENSE /tmp/LICENSE
COPY --from=build /work/build-helpers/linux/runtime-notes.txt /tmp/README-linux.txt
COPY --from=build /tmp/rtsp_viewer_tauri /tmp/rtsp_viewer_tauri

RUN set -eux; \
    version="$(cat /tmp/rtsp-viewer-version)"; \
    case "${TARGETARCH:-amd64}" in \
        amd64) arch="x86_64" ;; \
        arm64) arch="aarch64" ;; \
        arm) \
            case "${TARGETVARIANT:-}" in \
                v7) arch="armv7" ;; \
                *) arch="${TARGETARCH:-unknown}${TARGETVARIANT:+-${TARGETVARIANT}}" ;; \
            esac \
            ;; \
        ppc64le) arch="ppc64le" ;; \
        s390x) arch="s390x" ;; \
        *) arch="${TARGETARCH:-unknown}${TARGETVARIANT:+-${TARGETVARIANT}}" ;; \
    esac; \
    package_dir="/tmp/rtsp-viewer-${version}-linux-${arch}"; \
    mkdir -p "${package_dir}"; \
    install -m 0755 /tmp/rtsp_viewer_tauri "${package_dir}/rtsp_viewer_tauri"; \
    install -m 0644 /tmp/LICENSE "${package_dir}/LICENSE"; \
    install -m 0644 /tmp/README-linux.txt "${package_dir}/README-linux.txt"; \
    tar -C /tmp -czf "/out/rtsp-viewer-${version}-linux-${arch}.tar.gz" "$(basename "${package_dir}")"

FROM scratch AS export

COPY --from=package /out/ /
