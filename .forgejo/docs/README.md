# RTSP Viewer Forgejo Release Workflow

Use this checklist when maintaining this repository's `.forgejo/` release
workflow. The detailed workflow behavior is documented in
[`release-packaging-workflow.md`](release-packaging-workflow.md).

## Plan

1. Keep `.forgejo/workflows/release-packaging.yml` aligned with RTSP Viewer build
   helper outputs and artifact names.
2. Confirm the Forgejo runner fleet has the labels used by the matrix:
   `windows-latest`, `macos-latest`, and `ubuntu-22.04`.
3. Confirm each runner has the required native toolchain before pushing a tag.
4. Validate the workflow syntax and at least one local build path before pushing
   a release tag.

## Implementation

### Release Assets

The workflow publishes these deterministic asset names:

```text
rtsp-viewer-windows-amd64.zip
rtsp-viewer-macos-app.zip
rtsp-viewer-linux-x86_64.tar.gz
```

Search workflow files for copied Android or source-project assumptions after
edits:

```bash
rg -n 'APK|apk|Android|android|Launch Pad|launchpad|Gradle|gradlew|KEYSTORE|setup-java|setup-android|actions/checkout|uses:' .forgejo/workflows
```

### Runner Requirements

Windows:

- `bash`, `git`, `base64`, `curl`, `jq`
- `node`, `npm`, `cargo`, `rustup`
- MSVC Rust target `x86_64-pc-windows-msvc`
- Visual Studio Build Tools with Desktop development with C++
- Windows SDK
- PowerShell `Compress-Archive`

macOS:

- `bash`, `git`, `base64`, `curl`, `jq`
- `node`, `npm`, `cargo`, `rustup`
- Xcode Command Line Tools
- `codesign`
- `ditto`

Linux:

- `bash`, `git`, `base64`, `curl`, `jq`
- Docker with Buildx
- permission to create/use the `build-system` Docker network

### Secrets

The workflow uses the Forgejo-provided workflow token:

```text
${{ forgejo.token }}
```

No Android signing secrets, GitHub migration token, keystore, or APK-specific
secret is required.

## Validation

Run these checks in the target repository before pushing a release tag:

```bash
ruby -e 'require "yaml"; YAML.load_file(".forgejo/workflows/release-packaging.yml"); puts "yaml ok"'
rg -n 'APK|apk|Android|android|Launch Pad|launchpad|Gradle|gradlew|KEYSTORE|setup-java|setup-android|actions/checkout|uses:' .forgejo/workflows
npm --prefix ui ci
npm --prefix ui run build
cargo test --locked --workspace
```

The grep command should return no matches.

After validation, push a version tag:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

If release publishing fails with a 404, check `FORGEJO_API_URL`,
`FORGEJO_REPOSITORY`, and whether the workflow token can create or update
releases for the repository.
