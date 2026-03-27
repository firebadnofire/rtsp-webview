# syntax=docker/dockerfile:1.7

FROM debian:bookworm-slim AS package-common

ARG APT_HTTP_PROXY=""

ENV DEBIAN_FRONTEND=noninteractive

RUN set -eux; \
    if [ -n "${APT_HTTP_PROXY}" ]; then \
        printf 'Acquire::http::Proxy "%s";\n' "${APT_HTTP_PROXY}" > /etc/apt/apt.conf.d/01proxy; \
    else \
        rm -f /etc/apt/apt.conf.d/01proxy; \
    fi; \
    apt-get update && apt-get install -y --no-install-recommends \
    bash \
    binutils \
    ca-certificates \
    coreutils \
    cpio \
    curl \
    file \
    gzip \
    libarchive-tools \
    rpm \
    ruby \
    tar \
    xz-utils \
    zstd \
    && rm -rf /var/lib/apt/lists/*

RUN gem install --no-document fpm -v 1.16.0

COPY build-helpers/linux/packaging /input/packaging
COPY src-tauri/icons/icon.png /input/icon.png
COPY --from=linux_artifacts . /input/dist/linux/

ARG TARBALL_BASENAME
ARG APPIMAGETOOL_RELEASE=continuous

ENV TARBALL_BASENAME="${TARBALL_BASENAME}" \
    APPIMAGETOOL_RELEASE="${APPIMAGETOOL_RELEASE}"

FROM package-common AS package-deb
ENV PACKAGE_TARGET=deb
RUN /input/packaging/build-package-from-tarball.sh

FROM package-common AS package-rpm-rhel
ENV PACKAGE_TARGET=rpm-rhel
RUN /input/packaging/build-package-from-tarball.sh

FROM package-common AS package-rpm-zypper
ENV PACKAGE_TARGET=rpm-zypper
RUN /input/packaging/build-package-from-tarball.sh

FROM package-common AS package-arch
ENV PACKAGE_TARGET=arch
RUN /input/packaging/build-package-from-tarball.sh

FROM package-common AS package-appimage
RUN apt-get update && apt-get install -y --no-install-recommends \
    squashfs-tools \
    && rm -rf /var/lib/apt/lists/*

ENV PACKAGE_TARGET=appimage
RUN /input/packaging/build-package-from-tarball.sh

FROM scratch AS export-deb
COPY --from=package-deb /out/ /

FROM scratch AS export-rpm-rhel
COPY --from=package-rpm-rhel /out/ /

FROM scratch AS export-rpm-zypper
COPY --from=package-rpm-zypper /out/ /

FROM scratch AS export-arch
COPY --from=package-arch /out/ /

FROM scratch AS export-appimage
COPY --from=package-appimage /out/ /
