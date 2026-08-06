# RTSP Viewer Forgejo Release Workflow

Use this checklist when maintaining this repository's `.forgejo/` release
workflow. The detailed workflow behavior is documented in
[`release-packaging-workflow.md`](release-packaging-workflow.md).

## Plan

1. Keep `.forgejo/workflows/release-packaging.yaml` aligned with RTSP Viewer build
   helper outputs and artifact names.
2. Confirm the Forgejo runner fleet has the labels used by the matrix:
   `macos-latest` and `ubuntu-22.04`.
3. Confirm each runner has the required native or cross toolchain before pushing
   a tag.
4. Confirm the `GH_KEY` secret can push to and create releases in
   `github.com/firebadnofire/rtsp-webview`.
5. Confirm `CI_KEY` contains the private release-signing key and
   `CI_KEY_PASSPHRASE` contains its passphrase.
6. Validate the workflow syntax and at least one local build path before pushing
   a release tag.

## Implementation

### Release Assets

The workflow publishes these deterministic asset names:

```text
rtsp-viewer-windows-amd64.zip
rtsp-viewer-macos-app.zip (universal Intel/Apple Silicon app)
rtsp-viewer-linux-x86_64.tar.gz
```

Search workflow files for copied Android or source-project assumptions after
edits:

```bash
rg -n 'APK|apk|Android|android|Launch Pad|launchpad|Gradle|gradlew|KEYSTORE|setup-java|setup-android|actions/checkout|uses:' .forgejo/workflows
```

### Runner Requirements

macOS:

- `bash`, `git`, `base64`, `curl`, `jq`
- GnuPG as `gpg` (installed through Homebrew by the workflow when needed, with
  prompts and automatic updates disabled for the CI invocation)
- Node.js 24 LTS as `node`, plus `npm`, `cargo`, `rustup`
- Xcode Command Line Tools
- `lipo`
- `codesign`
- `ditto`
- host-mode runner service PATH must expose Rust and Homebrew tools, or the
  workflow must be able to find them in `~/.cargo/bin`, `/opt/homebrew/bin`, or
  `/usr/local/bin`

Linux:

- `bash`, `git`, `base64`, `curl`, `jq`
- GnuPG as `gpg` (installed from the Ubuntu `gnupg` package)
- `apt-get` access to install native Tauri/WebKit build dependencies
- `mingw-w64` and `zip` from Ubuntu packages for the Windows cross-build
- `x86_64-w64-mingw32-gcc` from `mingw-w64`
- Rust target `x86_64-pc-windows-gnu`
- outbound HTTPS access to NodeSource and rustup if Node 24 or Rust is missing

### Secrets

The workflow uses the Forgejo-provided workflow token for checkout and Forgejo
release publishing:

```text
${{ forgejo.token }}
```

It also requires these repository or organization secrets:

```text
GH_KEY
CI_KEY
CI_KEY_PASSPHRASE
```

`GH_KEY` must be able to push refs to `github.com/firebadnofire/rtsp-webview`,
create the repository if it is missing, and create/update GitHub releases and
release assets. `CI_KEY` may contain the ASCII-armored private key or its base64
encoding. `CI_KEY_PASSPHRASE` unlocks that key for noninteractive detached
signature creation. The workflow rejects a `CI_KEY` whose keyring does not
contain fingerprint `7D6EF134D851C8DA0862D97494F31AF374E2EE3C`.

### Verify Release Signatures

Each artifact is published with an ASCII-armored detached signature named by
appending `.asc` to the artifact filename. Recover the public key using one of
these methods:

```bash
gpg --keyserver hkps://keys.openpgp.org --recv-keys 7D6EF134D851C8DA0862D97494F31AF374E2EE3C
# Or:
gpg --keyserver hkps://keyserver.ubuntu.com --recv-keys 7D6EF134D851C8DA0862D97494F31AF374E2EE3C
# Or:
curl --proto '=https' --tlsv1.2 -fsSLo william.asc https://archuser.org/gpg/william.asc
gpg --import william.asc
```

Confirm the full fingerprint and then verify the downloaded artifact:

```bash
gpg --fingerprint 7D6EF134D851C8DA0862D97494F31AF374E2EE3C
gpg --verify <artifact>.asc <artifact>
```

The expected fingerprint is
`7D6E F134 D851 C8DA 0862 D974 94F3 1AF3 74E2 EE3C`.

## Validation

Run these checks in the target repository before pushing a release tag:

```bash
ruby --disable-gems -e 'require "yaml"; YAML.load_file(".forgejo/workflows/release-packaging.yaml"); puts "yaml ok"'
rg -n 'APK|apk|Android|android|Launch Pad|launchpad|Gradle|gradlew|KEYSTORE|setup-java|setup-android|actions/checkout|uses:' .forgejo/workflows
npm --prefix ui ci
npm --prefix ui run build
cargo test --locked --workspace
rustup target add x86_64-pc-windows-gnu
cargo check --locked --release --target x86_64-pc-windows-gnu -p rtsp_viewer_tauri
```

The grep command should return no matches.

After validation, push a version tag:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

If Forgejo release publishing fails with a 404, check `FORGEJO_API_URL`,
`FORGEJO_REPOSITORY`, and whether the workflow token can create or update
releases for the repository. If GitHub mirroring fails with a 404 or 403, check
`GH_KEY` permissions and the `GITHUB_MIRROR_REPOSITORY` value in the workflow.
