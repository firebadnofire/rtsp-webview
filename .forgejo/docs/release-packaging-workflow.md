# Release Packaging Workflow

This document explains `.forgejo/workflows/release-packaging.yml` for future
maintenance. The workflow builds Windows, macOS, and Linux release assets for
RTSP Viewer when a version tag is pushed, then publishes each asset to the
matching Forgejo release.

## Trigger

The workflow runs on pushed tags matching either lowercase or uppercase version
prefixes:

```yaml
'on':
  push:
    tags:
      - 'v*'
      - 'V*'
```

Examples that trigger the workflow:

- `v1.0.4`
- `V1.0.4`
- `v2.0.0-beta1`

## Runtime Environment

The workflow is a three-entry matrix:

| OS | Runner label | Build helper | Asset |
| --- | --- | --- | --- |
| Windows | `windows-latest` | `build-helpers\windows\build-exe.bat amd64` | `rtsp-viewer-windows-amd64.zip` |
| macOS | `macos-latest` | `./build-helpers/mac/build-app.sh` | `rtsp-viewer-macos-app.zip` |
| Linux | `ubuntu-22.04` | `BUILD_PLATFORM=linux/amd64 ./build-helpers/linux/build-tarball.sh` | `rtsp-viewer-linux-x86_64.tar.gz` |

The runner labels are infrastructure configuration. They must exist on the
Forgejo runner fleet and must map to machines with the native OS listed above.
This repository cannot verify runner labels locally.

## Checkout

The workflow does not use `actions/checkout`. It first prepares the minimal
checkout runtime, then performs checkout with shell commands against:

```text
${FORGEJO_SERVER_URL}/${FORGEJO_REPOSITORY}.git
```

The Linux matrix entry installs `ca-certificates`, `coreutils`, `curl`, `git`,
and `jq` with `apt-get` when any of those tools are missing. The Windows matrix
entry installs Git for Windows through Chocolatey when `git` or `bash` is
missing. The macOS matrix entry expects Xcode Command Line Tools or equivalent
runner provisioning to provide `git`.

The checkout step requires HTTPS and fails for non-HTTPS server URLs. It uses
the Forgejo workflow token as an HTTP extra header when a token is present. This
avoids relying on `actions/*` mirrors or automatic action rewrites in a
self-hosted Forgejo environment.

## Build Outputs

Windows:

1. Runs the repository Windows helper for `amd64`.
2. Verifies the helper output exists at `dist\windows\amd64\rtsp-viewer.exe`.
3. Compresses that executable into
   `dist\releases\rtsp-viewer-windows-amd64.zip`.

macOS:

1. Runs the repository macOS helper.
2. Builds `dist/macos/RTSP Viewer.app`.
3. Creates `dist/releases/rtsp-viewer-macos-app.zip` with `ditto`, preserving app
   bundle metadata.

Linux:

1. Runs the Docker Buildx tarball helper for `linux/amd64`.
2. Finds the generated `rtsp-viewer-*-linux-x86_64.tar.gz` tarball.
3. Copies it to `dist/releases/rtsp-viewer-linux-x86_64.tar.gz`.

Each job validates that its release artifact exists and is non-empty before
publishing.

## Forgejo Release Publishing

Each matrix job publishes its own artifact to the Forgejo release for the tag.
The publish step:

1. Derives the API URL, repository, tag, and token from Forgejo environment
   variables with GitHub-compatible fallbacks.
2. Requires an HTTPS API URL.
3. Looks up the release by tag.
4. Creates the release if it is missing.
5. Patches the release if it already exists.
6. Deletes an existing asset with the same deterministic filename.
7. Uploads the new asset.

The release publishing steps are safe to rerun for the same tag because release
metadata is patched and same-name assets are replaced.

## External Dependencies

Validated from this workspace on April 28, 2026:

| Dependency | Exact reference | Validation |
| --- | --- | --- |
| Forgejo/Gitea API | `https://pubcode.archuser.org/api/v1/version` | Returned `14.0.4+gitea-1.22.0`. |
| Repository HTTPS remote | `https://pubcode.archuser.org/firebadnofire/rtsp-webview.git` | `git ls-remote` returned `HEAD` and tag refs. |
| Dockerfile frontend | `docker.io/docker/dockerfile:1.7` | `docker buildx imagetools inspect` succeeded. |
| Linux build image | `docker.io/library/node:20-bullseye` | `docker manifest inspect` succeeded. |
| Linux package image | `docker.io/library/debian:bullseye-slim` | `docker manifest inspect` succeeded. |
| APT cache probe image | `docker.io/library/busybox:1.36.1` | `docker manifest inspect` succeeded. |
| Rust installer | `https://sh.rustup.rs` | HTTPS HEAD returned `200`. |
| npm registry | `https://registry.npmjs.org/` | HTTPS HEAD returned `200`. |
| crates.io API | `https://crates.io/api/v1/crates/serde` | HTTPS API request returned crate JSON. |
| Debian package repository | `https://deb.debian.org/debian/` | HTTPS HEAD returned `200`. |
| Debian security repository | `https://security.debian.org/debian-security/` | HTTPS HEAD returned `200`. |

Not locally verifiable from this workspace:

- Forgejo runner labels: `windows-latest`, `macos-latest`, `ubuntu-22.04`.
- Tools installed on those target runners.
- The permissions granted to `${{ forgejo.token }}` during workflow execution.

If any of these are unavailable on the target Forgejo instance, update the matrix
runner labels or runner provisioning before pushing a release tag.

## Security Notes

- The workflow requires HTTPS for repository checkout and release publishing.
- `curl` calls restrict protocols to HTTPS and disable deprecated TLS versions
  below TLS 1.2 while allowing TLS 1.3 when the server negotiates it.
- No release credentials are hardcoded.
- No Android keystore, signing password, GitHub migration token, or APK-specific
  secret is used.
- The workflow does not use third-party `uses:` actions, which avoids ambiguous
  Forgejo action mirror behavior.

## Validation

Before pushing a release tag, run:

```bash
ruby -e 'require "yaml"; YAML.load_file(".forgejo/workflows/release-packaging.yml"); puts "yaml ok"'
rg -n 'APK|apk|Android|android|Launch Pad|launchpad|Gradle|gradlew|KEYSTORE|setup-java|setup-android|actions/checkout|uses:' .forgejo/workflows
npm --prefix ui ci
npm --prefix ui run build
cargo test --locked --workspace
```

The grep command should return no matches. The full release matrix still requires
the Forgejo runner fleet because Windows, macOS, and Linux builds are native.
