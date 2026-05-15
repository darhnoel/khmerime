# KhmerIME Updater Architecture

## 1. Purpose

KhmerIME needs an update architecture that treats software updates and language
data updates as separate concerns.

The IME has two different kinds of change:

- application changes, such as executable code, UI behavior, native platform
  adapters, installers, and platform integration;
- language data changes, such as dictionary entries, roman lookup tables,
  scoring resources, and statistical language data.

Those changes have different risk profiles. Application updates may require an
installer, package manager, elevated privileges, or an application restart. Data
updates should usually be smaller, lower interruption, and rollback-safe because
they affect typing quality directly.

This document describes the proposed updater architecture. It is intentionally
implementation-oriented, but it does not claim that KhmerIME already has this
updater implemented.

## Current Repository State

The repository currently contains:

| Area | Current state |
| --- | --- |
| Shared engine | `crates/core` compiles lexicon and language resources into binary blobs at build time. |
| Session runtime | `crates/session` owns platform-neutral IME state and key behavior. |
| Web/desktop app | `apps/dioxus-app` runs the Dioxus UI. With the `fetch-data` feature on wasm builds, compiled data blobs are copied to `assets/data`. |
| Linux | `adapters/linux-ibus`, `scripts/platforms/linux/ibus`, and `packaging/linux/deb` provide the Linux IBus path and package work. |
| Windows | `adapters/windows-tsf`, `scripts/platforms/windows/tsf`, and `packaging/windows/wix` provide the TSF path and unsigned MSI work. |
| Other platforms | Android, iOS, and macOS adapter scaffolds exist. |
| Updater subsystem | No dedicated update manager, manifest checker, data updater, or rollback manager is currently implemented. |

## Proposed Boundary

The updater should be a product/runtime subsystem, not part of decoder ranking
or keyboard event handling. Shared update validation and manifest parsing should
be reusable across platforms, while final app installation should remain
platform-specific.

Recommended future ownership:

| Component | Responsibility |
| --- | --- |
| Shared update library | Manifest parsing, version checks, checksum verification, downloaded data validation, rollback metadata. |
| Platform adapter/package | App update handoff to `.deb`, MSI, package manager, app store, or native installer. |
| App/runtime shell | User prompts, release notes, scheduling, "restart required" state. |
| Core engine | Loading validated language data through a stable schema contract. |

## 2. Update Types

### App Update

An app update changes KhmerIME software or packaged platform code.

Examples:

| Platform/surface | App update contents |
| --- | --- |
| Desktop app | Dioxus desktop binary, UI code, bundled engine code. |
| Web app | Built web artifacts and static assets. |
| Linux IBus | IBus bridge binary, Python adapter files, `.desktop`/component metadata, Debian package files. |
| Windows TSF | TSF DLL, COM registration behavior, MSI package, native diagnostics. |
| macOS | Future InputMethodKit bundle/package. |
| Android/iOS | Future native keyboard package or app store release. |

App updates may need a restart, logout/login, input-source reload, package
manager transaction, or installer. They should not be treated like ordinary data
file replacement.

### Data Update

A data update changes language resources without changing executable code.

Examples:

| Data type | Examples |
| --- | --- |
| Dictionary/lexicon | `roman,target` entries, proper names, rare vocabulary, corrected mappings. |
| Roman lookup data | Compiled lookup blobs derived from CSV sources. |
| POS/statistics data | khPOS-derived statistics or related language resources. |
| Scoring tables | Future decoder weights, ranking tables, n-gram resources. |
| Keyboard/language resources | Future keymaps or language metadata that can be validated independently of executable code. |

Data updates should be atomic and rollback-safe. A failed data update must not
leave KhmerIME unable to type.

## 3. High-Level Architecture

```text
KhmerIME App
  -> Update Manager
      -> Manifest Checker
      -> App Patch Updater
      -> Data Updater
      -> Verifier
      -> Rollback Manager
```

Expanded view:

```text
                remote update endpoint
                         |
                         v
KhmerIME App -> Update Manager -> Manifest Checker
                         |              |
                         |              v
                         |        version/channel policy
                         |
        +----------------+----------------+
        |                                 |
        v                                 v
App Patch Updater                  Data Updater
        |                                 |
        v                                 v
platform installer/package         temp download
        |                                 |
        v                                 v
restart/reload required            checksum + schema verify
                                          |
                                          v
                                   atomic data swap
                                          |
                                          v
                                   engine reload
                                          |
                                          v
                                   rollback on failure
```

## 4. Manifest Design

KhmerIME should use a JSON manifest that separates app and data versions. The
manifest should be small enough to fetch frequently and strict enough to reject
unsafe updates before downloading large artifacts.

Example proposed manifest:

```json
{
  "schema_version": 1,
  "channel": "stable",
  "generated_at": "2026-05-14T00:00:00Z",
  "app": {
    "app_version": "0.2.0",
    "required": false,
    "minimum_current_app_version": "0.1.0",
    "platforms": {
      "linux-x86_64": {
        "package_type": "deb",
        "url": "https://updates.example.invalid/khmerime/0.2.0/linux/khmerime_0.2.0_amd64.deb",
        "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "size_bytes": 12345678
      },
      "windows-x86_64": {
        "package_type": "msi",
        "url": "https://updates.example.invalid/khmerime/0.2.0/windows/KhmerIME-0.2.0-x64.msi",
        "sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        "size_bytes": 23456789
      }
    },
    "release_notes": [
      "Improve native IME behavior.",
      "Fix package installation issues."
    ]
  },
  "data": {
    "data_version": "2026.05.14.1",
    "data_schema_version": 1,
    "required": false,
    "minimum_app_version": "0.1.0",
    "resources": [
      {
        "name": "roman_lookup",
        "format": "khmerime-lexicon-bin",
        "url": "https://updates.example.invalid/khmerime-data/2026.05.14.1/roman_lookup.lexicon.bin",
        "sha256": "1111111111111111111111111111111111111111111111111111111111111111",
        "size_bytes": 3456789
      },
      {
        "name": "khpos_stats",
        "format": "khmerime-khpos-bin",
        "url": "https://updates.example.invalid/khmerime-data/2026.05.14.1/khpos.stats.bin",
        "sha256": "2222222222222222222222222222222222222222222222222222222222222222",
        "size_bytes": 4567890
      }
    ],
    "release_notes": [
      "Add new dictionary entries.",
      "Improve ranking resources."
    ]
  }
}
```

Required manifest fields:

| Field | Purpose |
| --- | --- |
| `schema_version` | Version of the manifest format itself. |
| `channel` | Stable, beta, nightly, or update-test. |
| `app.app_version` | Version of the available app package. |
| `data.data_version` | Version of the available language data. |
| `data.data_schema_version` | Version of the runtime data format contract. |
| `url` | Download URL for each update artifact. |
| `sha256` | Expected SHA-256 checksum for each artifact. |
| `minimum_app_version` | Minimum KhmerIME app version that can load the data. |
| `minimum_current_app_version` | Minimum installed app version that can apply the app update directly. |
| `required` | Whether the update is mandatory. |
| `release_notes` | User-visible summary. |

The manifest should not be trusted until it passes schema validation. Downloaded
artifacts should not be trusted until checksum verification passes.

## 5. Local File Layout

Use platform-specific base directories, but keep the internal layout consistent.

Proposed Linux-style example:

```text
~/.local/share/khmerime/
  app/
    current/
    pending/
  data/
    current/
      manifest.json
      roman_lookup.lexicon.bin
      khpos.stats.bin
      next_word.stats.bin
    previous/
      manifest.json
      roman_lookup.lexicon.bin
      khpos.stats.bin
      next_word.stats.bin
  downloads/
    app/
    data/
  backups/
    data/
      2026-05-14T120000Z/
  logs/
    updater.log
  state/
    update-state.json
```

Platform-specific notes:

| Platform | Recommended base |
| --- | --- |
| Linux | XDG data/config/cache locations, for example `~/.local/share/khmerime` and `~/.cache/khmerime`. |
| Windows | `%LOCALAPPDATA%\\KhmerIME` for user data/cache; app binaries through MSI install locations. |
| macOS | `~/Library/Application Support/KhmerIME` for data/cache; app updates through package/app bundle mechanisms. |
| Android/iOS | App-private storage managed by platform APIs. |

Data files should have one active `current` directory. New data should be
downloaded into a temporary path, verified, then atomically promoted.

## 6. Data Update Flow

Recommended flow:

```text
1. Fetch update manifest.
2. Compare local app_version, data_version, and data_schema_version.
3. If data update is applicable, download resources to downloads/data/.
4. Verify SHA-256 for every downloaded resource.
5. Validate data schema and resource headers before activation.
6. Build a complete staging directory.
7. Backup the current data directory.
8. Atomically replace the active data pointer or active data directory.
9. Ask the engine/session layer to reload data.
10. If reload succeeds, mark the data update active.
11. If reload fails, restore previous data and log the failure.
```

The data updater must never partially replace active files. Either the full data
set becomes active or the previous data set remains active.

Recommended data activation model:

```text
data/
  versions/
    2026.05.14.1/
    2026.05.20.1/
  current -> versions/2026.05.20.1
  previous -> versions/2026.05.14.1
```

If symlinks are not appropriate on a platform, store the active version in
`state/update-state.json` and open data files by resolved version directory.

Reload behavior:

- The runtime should avoid reloading data while an active composition is being
  committed.
- If the app has an active preedit/session, the data update can be staged and
  activated after the current composition ends.
- If runtime reload is not available for a platform, mark the data update as
  "restart required" instead of forcing immediate reload.

## 7. App Patch Flow

App updates should generally be handed off to an installer or package manager.
The updater should avoid directly overwriting executable files that may be
running or loaded by the operating system.

Recommended app flow:

```text
1. Fetch update manifest.
2. Select platform-specific app artifact.
3. Check current app version against manifest policy.
4. Show release notes and ask for confirmation unless policy requires otherwise.
5. Download the installer/package to downloads/app/.
6. Verify SHA-256.
7. If future signature verification exists, verify the signature.
8. Hand off to platform installer/package manager.
9. Mark restart, logout/login, input-source reload, or app relaunch as required.
10. Record result in update logs.
```

Windows TSF requires special care. TSF DLL files may be loaded by running
applications, the text service framework, or system processes. KhmerIME should
not attempt to directly replace a loaded TSF DLL from inside the running IME.
The Windows app update path should use the MSI/package flow, perform proper COM
registration updates, and schedule replacement through installer semantics when
files are locked.

Linux IBus app updates should similarly prefer `.deb`/package replacement or an
installer script that can update the bridge, Python adapter, component metadata,
and package-owned files consistently.

## 8. Versioning and Compatibility

KhmerIME should maintain separate versions:

| Version | Meaning |
| --- | --- |
| `app_version` | Version of the application, engine code, UI, native adapter, or package. |
| `data_version` | Version of the language data bundle. |
| `data_schema_version` | Compatibility version for the binary/data file format loaded by the runtime. |

Compatibility rules:

- Data may require a minimum app version.
- The app must reject unsupported `data_schema_version` values.
- The app may accept older data schema versions only when the loader explicitly
  supports them.
- A newer app should keep working with the bundled data if an external data
  update fails.
- A failed data activation must rollback to the previous known-good data set.
- A failed app update should leave the current installed app usable.

The current build-time data blobs already use internal magic bytes for compiled
resources. A future updater should preserve that kind of early format check and
add explicit schema/version metadata around updateable data bundles.

## 9. Update Channels

Recommended channels:

| Channel | Purpose | Users |
| --- | --- | --- |
| `stable` | Conservative updates intended for normal users. | Default for releases. |
| `beta` | Earlier access to tested features and language data. | Users who opt in to help validate. |
| `nightly` | Frequent development builds, may be unstable. | Developers and advanced testers. |
| `update-test` | Local or staging endpoint for testing updater behavior. | Maintainers only. |

Channel should be part of the manifest URL or request path and repeated inside
the manifest for validation. KhmerIME should not silently move a user from
stable to beta/nightly.

## 10. Failure Handling

| Failure | Expected behavior |
| --- | --- |
| Failed download | Keep current app/data active, retry later, show low-noise error or log entry. |
| Checksum mismatch | Delete the downloaded artifact, reject the update, log a security-relevant error. |
| Incompatible schema | Reject the data update before activation and keep current data. |
| Interrupted update | On next startup, inspect `update-state.json`, clean incomplete staging files, keep current data. |
| Corrupted data file | Reject during validation or reload, rollback to previous data, log the failing resource. |
| Failed app update | Leave current app installed, report installer/package failure, do not mark update complete. |
| Rollback failure | Stop using the failed data, fall back to bundled data if available, show clear error, log urgently. |

The updater should be restart-safe. Every multi-step update should write enough
state for KhmerIME to determine whether it was idle, downloading, staging,
activated, failed, or rolled back.

## 11. Security

Minimum requirements:

- Fetch manifests and artifacts over HTTPS.
- Validate manifest structure before using it.
- Verify SHA-256 for every downloaded artifact before activation.
- Never execute downloaded files directly from a temporary location.
- Never activate data that fails checksum or schema validation.
- Log checksum mismatch and schema mismatch distinctly.

Future recommended hardening:

- Signed manifests.
- Signed app packages where platform tooling supports it.
- Optional detached signatures for data bundles.
- Public-key pinning for manifest verification.

This repository currently has package work for Linux and Windows, but this
document does not claim that update signing or updater signature verification is
already implemented.

## 12. User Experience

Recommended UX:

- Data updates can be silent or low-interruption when they are compatible and
  rollback-safe.
- App updates should ask for confirmation unless the platform package manager
  owns the prompt.
- Show release notes for app updates and meaningful data updates.
- Avoid interrupting active typing. Do not reload language data in the middle of
  an active preedit/commit.
- Show clear error messages when a user action is needed.
- Keep detailed errors in logs for debugging.
- If an update requires restart, input-source reload, logout/login, or app
  relaunch, say that directly.

Suggested states:

| State | User-facing behavior |
| --- | --- |
| Data update available | Optional notification or silent background update depending on settings. |
| Data update installed | No interruption; new data applies after safe reload. |
| App update available | Prompt with version and release notes. |
| App update installed | Prompt for restart/reload if required. |
| Update failed | Brief explanation plus log location for diagnostics. |

## 13. Implementation Plan

### Phase 1: Manifest and Version Model

- Define manifest schema.
- Define `app_version`, `data_version`, and `data_schema_version`.
- Add parsing and validation tests.
- Document current bundled data version behavior.

### Phase 2: Data Update Only

- Implement manifest fetch/check behind an explicit setting or developer flag.
- Download data artifacts to a temporary directory.
- Verify checksums.
- Validate data schema and resource headers.
- Add a runtime data loading contract that can choose bundled data or external
  validated data.

### Phase 3: Rollback and Logging

- Add updater state file.
- Add backup/previous-version tracking.
- Add atomic activation.
- Add rollback on failed reload.
- Add logs with enough detail for user support.

### Phase 4: App Update Through Installer/Package

- Add platform-specific app update handoff.
- Linux: hand off to `.deb`/package install flow or a package-manager-aware
  updater path.
- Windows: hand off to MSI; do not directly replace loaded TSF DLLs.
- macOS/iOS/Android: defer to the appropriate package/app-store mechanism when
  those platform packages become real.

### Phase 5: Channels and Optional Signature Verification

- Add stable, beta, nightly, and update-test channel selection.
- Add signed manifest support.
- Add package/data signature verification where appropriate.
- Add CI checks for generated manifests and update artifact integrity.

## 14. Acceptance Criteria

- KhmerIME can detect app and data updates separately.
- KhmerIME can compare local and remote `app_version` independently from
  `data_version`.
- KhmerIME rejects data updates that require a newer app version.
- KhmerIME rejects unsupported `data_schema_version` values.
- Data updates are downloaded to a temporary location before activation.
- Data update artifacts are verified by SHA-256 before use.
- Data activation is atomic and rollback-safe.
- A failed data reload restores the previous known-good data.
- App updates do not directly replace loaded Windows TSF DLLs.
- App updates are handed off to installer/package mechanisms.
- Update logs are written for debugging.
- User-facing prompts distinguish app updates from data updates.
- Documentation clearly distinguishes current repository state from proposed
  updater design.

## Open Questions

- What should the first real `data_version` format be: semantic version,
  date-based version, or monotonically increasing build number?
- Should external data updates be supported by all platforms, or only desktop
  platforms first?
- Should web builds use the same manifest as native builds or a web-specific
  manifest served with static assets?
- Where should the first shared update library live: `crates/update`, inside
  `crates/core`, or as part of the app/platform layer?
- What is the minimum supported rollback behavior on mobile platforms?
- Which signing scheme should be used when update signing is introduced?
