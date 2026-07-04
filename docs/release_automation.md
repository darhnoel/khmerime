# Release Automation Plan

This plan gets KhmerIME to a professional 1.0.0 release without requiring the maintainer to switch between local machines. The maintainer drives the release from macOS; GitHub Actions supplies the platform-specific build machines.

## Target Shape

Public releases use one **Product Version** across KhmerIME:

```text
v1.0.0
KhmerIME 1.0.0
```

Each packaged artifact may also carry a platform build number, but user-facing release notes and support language use the product version.

## Phase 1: Unsigned Draft Release

Goal: prove release automation and artifact collection before handling credentials.

1. Add a manual GitHub Actions workflow named `Release Candidate`.
2. Build artifacts on native runners:
   - `ubuntu-latest`: `make linux-package`
   - `windows-latest`: `make windows-package`
   - `macos-latest`: `make macos-package`, `make ios-package`
   - `ubuntu-latest` or `macos-latest`: `make android-package`
3. Upload each platform artifact as a workflow artifact.
4. A final release job downloads all artifacts, writes `SHA256SUMS`, and creates a draft GitHub Release.
5. The release remains a draft until manually inspected and published.

Start with release-candidate tags:

```text
v1.0.0-rc.1
v1.0.0-rc.2
```

Only use `v1.0.0` after signing, notarization, and smoke testing are ready.

## Phase 2: Low-Risk Signing

Goal: add signing steps that do not depend on complex native store flows.

1. Linux:
   - Keep publishing `.deb` for direct download.
   - Sign `SHA256SUMS` with GPG.
   - Later, publish through a signed PPA or APT repository.
2. Android:
   - Store the upload keystore in GitHub Secrets.
   - Build a Play-uploadable signed `.aab`.

## Phase 3: Apple Signing

Goal: make Apple artifacts acceptable for real users.

1. macOS:
   - Sign the `.app` with Developer ID Application.
   - Notarize with Apple.
   - Staple the notarization ticket.
   - Build/sign the installer package.
2. iOS:
   - Archive with distribution signing.
   - Export/upload to App Store Connect.
   - Treat `.xcarchive` as a CI artifact, not the public download.

## Phase 4: Windows Signing

Goal: sign Windows artifacts without owning a Windows PC.

1. Build on `windows-latest`.
2. Sign `khmerime_windows_tsf.dll`.
3. Build the MSI.
4. Sign the MSI.
5. Verify signatures with `signtool verify`.

Prefer Azure Trusted Signing / Artifact Signing if available. Otherwise use a CA-issued code-signing certificate configured in GitHub Actions.

## Local Trial Script

Use the local helper to rehearse artifact collection without publishing:

```bash
scripts/release/prepare_draft_release.sh --tag v1.0.0-rc.1
```

To also build whatever this host can build first:

```bash
scripts/release/prepare_draft_release.sh --tag v1.0.0-rc.1 --build
```

The script writes a release staging directory:

```text
dist/release/v1.0.0-rc.1/
```

It prints the exact `gh release create --draft ...` command. Add `--create` only when you want it to call GitHub:

```bash
scripts/release/prepare_draft_release.sh --tag v1.0.0-rc.1 --create
```

## Guardrails

- Do not publish unsigned desktop installers as the final `v1.0.0`.
- Keep draft release creation separate from public release publication.
- Keep mobile store upload separate from GitHub desktop downloads.
- Do not put private certificates, keystores, or passwords in the repository.
- Add signing one platform at a time, after unsigned package generation works in CI.
