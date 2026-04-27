# Platform IME Workflow

This is the shared workflow for developing KhmerIME native packages across Linux, Android, iOS, macOS, and Windows.

The goal is to let each platform move independently without cluttering the repo or duplicating IME behavior.

## Core Rule

Platform adapters own native integration. Shared IME behavior stays in shared crates.

```text
platform adapter
  -> crates/session::ImeSession
  -> crates/core::Transliterator
```

Do not reimplement roman normalization, segmentation, ranking, candidate cycling, or commit behavior in a platform folder.

## Folder Ownership

| Platform | Adapter crate | Platform doc | Package/dev scripts | Current status |
| --- | --- | --- | --- | --- |
| Linux | `adapters/linux-ibus` | `docs/platforms/linux.md`, `docs/ubuntu_ibus.md` | `scripts/platforms/linux/ibus/`, `packaging/linux/deb/` | Working developer-local IBus path and `.deb` package |
| Android | `adapters/android-ime` | `docs/platforms/android.md` | add only when real Android build/install scripts exist | Scaffold |
| iOS | `adapters/ios-keyboard` | `docs/platforms/ios.md` | add only when real Xcode/build scripts exist | Scaffold |
| macOS | `adapters/macos-imk` | `docs/platforms/macos.md` | add only when real IMK build/install scripts exist | Scaffold |
| Windows | `adapters/windows-tsf` | `docs/platforms/windows.md` | add only when real TSF register/package scripts exist | Scaffold |

Use this layout rule:

```text
adapters/<platform>/
  Native runtime adapter code and adapter-local tests.

scripts/platforms/<platform>/<backend>/
  Developer-local install, uninstall, smoke, or package commands that actually run today.

docs/platforms/<platform>.md
  Platform-specific development notes and manual smoke checklist.

packaging/<platform>/
  Future release packaging files only after a real package format exists.
```

Do not add empty packaging folders or placeholder scripts. Add a folder only when it contains a real command, manifest, installer definition, or release note that a developer can use.

## Developer Workflow

For platform-only work:

1. Read this file and the platform doc under `docs/platforms/`.
2. Work inside the platform adapter and its platform script/doc folders.
3. Do not edit `crates/core` or `crates/session` unless the platform exposes a real shared gap.
4. Run the platform check target before committing.
5. Add/update a manual smoke checklist when the platform behavior changes.

For shared session/core work:

1. Update the shared crate first.
2. Add shared tests in `crates/session` or `crates/core`.
3. Run all platform adapter checks because every native IME depends on the shared contract.
4. Update `specs/structure/module-boundaries.md` if ownership changes.
5. Update `specs/structure/verification-surfaces.md` if required checks change.

## Make Targets

Check all native adapter crates:

```bash
make platform-check
```

Check one platform:

```bash
make platform-check-linux
make platform-check-android
make platform-check-ios
make platform-check-macos
make platform-check-windows
```

Linux currently also has developer-local runtime commands:

```bash
make ibus-install
make ibus-smoke
make ibus-uninstall
```

Linux also has the first real package target:

```bash
make linux-package
```

This writes `dist/linux/khmerime_<version>_amd64.deb`.

When another platform grows real package scripts, add matching make targets only after the script works locally. Prefer names like:

```text
make android-package
make ios-package
make macos-package
make windows-package
```

Do not add package targets that only echo TODO text.

## Packaging Principles

A platform package should be reproducible from one command and should not require users to copy files manually.

A package workflow should define:

- build prerequisites,
- package command,
- produced artifact path under `dist/`,
- install command,
- uninstall command,
- smoke test checklist,
- known limitations.

Recommended artifact naming:

```text
dist/linux/khmerime_<version>_<arch>.deb
dist/android/khmerime-<version>.apk
dist/ios/KhmerIME-<version>.xcarchive
dist/macos/KhmerIME-<version>.pkg
dist/windows/KhmerIME-<version>-x64.msi
```

The package should include compiled engine data. General users should not need raw files from `data/` at runtime unless the platform deliberately supports external data updates.

## Non-Interference Rules

- Keep platform-specific code in that platform's adapter crate.
- Keep platform-specific scripts under `scripts/platforms/<platform>/`.
- Keep package/release manifests under `packaging/<platform>/` only when real packaging begins.
- Do not change shared key semantics in `crates/session` for one platform without adding shared tests.
- Do not change decoder/ranking behavior in `crates/core` as part of package plumbing.
- Do not mix installer work with candidate ranking, lexicon, or UI behavior changes in one commit.
- Do not copy adapter code between platforms. Share concepts through `crates/session` contracts instead.

## Commit Shape

Prefer separate commits:

```text
feat(windows): add TSF key event mapping scaffold
chore(windows): add MSI packaging script
docs(windows): document TSF smoke checklist
feat(session): add platform-neutral key enum
```

Avoid mixed commits like:

```text
update windows package and tweak decoder ranking
```

## CI Direction

Current CI checks scaffold adapter crates. As package workflows become real, add one CI job per platform package when feasible:

```text
linux-package      ubuntu-latest
android-package    ubuntu-latest or macos-latest depending on toolchain
ios-package        macos-latest
macos-package      macos-latest
windows-package    windows-latest
```

Each packaging job should upload an artifact and run at least one smoke or structural validation. Keep package CI independent so a Windows installer failure does not block Android-only adapter iteration unless the PR claims full-platform release readiness.
