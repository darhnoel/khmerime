# Packaging

One command per platform. Each emits a distributable artifact under `dist/<os>/`.

```
make package            # everything THIS host can build (see the matrix)
make <os>-package       # just one platform
```

No single host builds all five — Apple targets need macOS, `.deb` needs Linux, `.msi` needs Windows.

## Matrix

| `make …`          | artifact                         | build host | needs                                  | reproducible unsigned? |
|-------------------|----------------------------------|------------|----------------------------------------|------------------------|
| `macos-package`   | `dist/macos/*.pkg`               | macOS      | Xcode, xcodegen                        | yes                    |
| `ios-package`     | `dist/ios/*.xcarchive`           | macOS      | Xcode, xcodegen                        | yes                    |
| `android-package` | `dist/android/*.aab`             | macOS/Linux| Android SDK+NDK, JDK, `cargo-ndk`      | yes                    |
| `linux-package`   | `dist/linux/*.deb`               | Linux      | `dpkg-deb`                             | yes                    |
| `windows-package` | `dist/windows/*.msi`             | Windows    | WiX (`wix` on PATH)                    | yes                    |

Version comes from the root `Cargo.toml`; override with `KHMERIME_PACKAGE_VERSION=…`.

## Signing — opt-in via a per-platform config file

The `*-package` targets build **unsigned by default** (reproducible, credential-free). To get a
**professional signed** artifact, drop in that platform's git-ignored signing config — copy the
committed `.example`, fill your credentials. Same rule everywhere: **`*.example` is committed, the
real file is git-ignored.** Each config is in the platform's *native* form (no custom format):

| platform | copy this `.example` → real (git-ignored) | holds | consumed by |
|---|---|---|---|
| **macOS** | `adapters/macos-imk/macos-signing.example.mk` → `…local.mk` ✅ filled | Developer ID, Team, notary profile | `codesign`+`notarytool` (`make platform-reinstall-macos`) |
| **Android** | `adapters/android-ime/keystore.properties.example` → `keystore.properties` | upload keystore + passwords | gradle `signingConfig` |
| **iOS** | `adapters/ios-keyboard/ExportOptions.example.plist` → `ExportOptions.plist` | distribution method + Team | `xcodebuild -exportArchive` |
| **Windows** | `adapters/windows-tsf/signing.example.ps1` → `signing.local.ps1` | cert thumbprint or `.pfx` | `signtool` |

Per-platform notes:
- **macOS** — config is filled; Tahoe Gatekeeper *requires* sign+notarize+staple for input methods.
- **Android** — for **Google Play** you only need an **upload** key (Play re-signs). The `.aab` is
  unsigned today because there's no `signingConfig` yet.
- **iOS** — needs a provisioning profile for **both** the container app *and* the keyboard appex
  (in your keychain/account); the plist only picks the method. `KHMERIME_IOS_SIGN=1 make ios-package`.
- **Windows** — sign the `.msi` post-build with `signtool` using the thumbprint/pfx in the config.

> Wiring status: the **config files + `.gitignore` exist**, but the *consumers* (gradle
> `signingConfig`, the `exportArchive` step, the `signtool` step) are **not wired yet** — adding
> them is the next step, kept gated so an absent config still yields today's unsigned build.

## Adding a platform

Mirror the pattern, nothing new to learn:
1. `scripts/platforms/<os>/<adapter>/build_<artifact>.sh` — reuse `make platform-build-<os>` for
   the build half, then wrap the output into `dist/<os>/`.
2. Add a `<os>-package` target in the `Makefile` next to the others (and to `package`'s `case`).

## Adding a platform

Mirror the pattern, nothing new to learn:
1. `scripts/platforms/<os>/<adapter>/build_<artifact>.sh` — reuse `make platform-build-<os>` for
   the build half, then wrap the output into `dist/<os>/`.
2. Add a `<os>-package` target in the `Makefile` next to the others (and to `package`'s `case`).
