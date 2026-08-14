# Making Windows-TSF and Linux-IBus AI-ready

Status: **plan** (grill-resolved 2026-08-14). Builds are intentionally deferred; this
document is the leak-safe wiring plan so both desktop adapters are ready to link the
private AI provider the moment a Windows/Linux build machine exists.

## Goal and non-goals

- **Goal:** Windows-TSF and Linux-IBus become AI-*ready* — the public seam and the
  private link/build targets exist, matching how iOS / Android / macOS-IMK already
  work — **without leaking any model, inference source, or model-runtime dependency
  into the public repo.**
- **Non-goal (deferred):** actually producing the Windows `.dll` and Linux bridge
  binary. candle/ByT5 is notoriously hard to cross-compile from macOS; native builds
  come later on a Windows machine / Linux container. Nothing here requires a build.

## How AI works today (the reference the plan mirrors)

The public engine (`crates/core`) carries a provider seam and **no model**:

- `SpanProposalProvider` trait + `register_span_proposal_provider()` slot
  (`crates/core/src/decoder/span_proposal.rs`).
- `SpanProposalMode::Model` is inert unless a provider is registered.
- Documented in [provider-architecture.md](../provider-architecture.md); the public
  tree has zero references to any model runtime, weights, or architecture.

The private AI crate (`khmerime-lab/runtime/tonle-native`, `khmerime-tonle`)
implements the trait (`TonleProvider`), loads weights, and **self-registers** on an
arming call. It is fused with each platform's public adapter into **one Cargo link
graph** so both share the same `khmerime_core` provider registry. The fusion is a
per-`target_os` **link anchor** — a `#[no_mangle] khmerime_ai_link_anchor()` that
references a stable symbol of the platform's public adapter so the linker keeps the
adapter's object code:

| Platform | Public adapter shape | Anchor references | AI artifact |
| --- | --- | --- | --- |
| iOS | staticlib | `KhmerIMESession::new()` | fused staticlib → paid xcframework |
| macOS-IMK | staticlib | `MacosIMKSession::new()` | fused staticlib → paid xcframework |
| Android | cdylib (`.so`) | `Java_..._nativeCreate` | fused `libkhmerime_android_ime.so` |

The leak boundary (`ai.mk` header): the AI build "writes only inside this AI lab or
into **gitignored paths** in the public tree. The public `project.yml` / `Makefile`
are never modified." Android's drop-ins (`AiModelProvider.kt`, manifest, `.so`) all
land in gitignored paths; iOS uses a gitignored `project-paid.yml`.

## The two new platforms

| Platform | Public adapter shape | Consequence |
| --- | --- | --- |
| **Windows-TSF** | `cdylib` (`rlib` + `cdylib`) — a COM DLL loaded by the OS | Structurally identical to Android. |
| **Linux-IBus** | a **`[[bin]]`** (`khmerime_ibus_bridge`) spawned by the Python IBus engine, plus an existing `[lib]` | The provider must land in the *binary's* link graph, not a lib's. |

## Resolved design (from the grill)

1. **Linking model — fused replacement artifact** on both platforms (mirrors
   Android/iOS/macOS):
   - **Windows:** the AI crate builds one `khmerime_windows_tsf.dll` linking the TSF
     adapter + Tonle, self-registering; swapped into a gitignored path. The COM
     registration (the class the OS loads) is unchanged — the OS just loads the AI DLL.
   - **Linux:** the AI crate builds a **replacement `khmerime_ibus_bridge` binary**
     with the provider statically linked + self-registered. The Python IBus engine is
     unchanged; it spawns the AI bridge from a gitignored path.

2. **Link anchor — a public no-op symbol** added to each public adapter (carries zero
   AI information, exactly like Android's public `nativeCreate`):
   - **Windows-TSF:** add `pub fn khmerime_tsf_link_anchor()` (no-op) to the adapter
     lib; the AI crate's `#[cfg(windows)]` anchor references it.
   - **Linux-IBus:** the crate already has a `[lib]` (`lib.rs`). Add
     `pub fn khmerime_ibus_link_anchor()` (no-op) there; the **fused AI bridge binary**
     calls it, pulling the AI crate + provider registration into the binary's link
     graph. (The free bridge never calls it, so the free build is unaffected.)

3. **Build environment — deferred.** Do the leak-safe wiring now; pick native
   build (Windows VM + Linux Docker) later. No cross-compile attempt today.

4. **Scope now — public readiness on `dev` + lab stubs.** Public repo gets the
   AI-free plumbing; the lab gets `#[cfg]` anchors and `ai-windows-*` / `ai-linux-*`
   targets stubbed so they are present, documented, and fail cleanly until a build
   env exists.

## Work items

### A. Public repo (`khmerime`, on the `dev` branch) — all AI-free, reviewable

1. **Windows-TSF anchor.** In `adapters/windows-tsf/src/lib.rs`, add a stable no-op:
   ```rust
   /// Link anchor for an out-of-tree provider build (mirrors Android's public
   /// nativeCreate). No-op; carries no model information. See docs/handoff/…md.
   #[no_mangle]
   pub extern "C" fn khmerime_tsf_link_anchor() {}
   ```
2. **Linux-IBus anchor.** In `adapters/linux-ibus/src/lib.rs`, add the same:
   ```rust
   #[no_mangle]
   pub extern "C" fn khmerime_ibus_link_anchor() {}
   ```
   (The `[lib]` already exists; no new crate-type needed.)
3. **Gitignore reservations.** Add the gitignored drop-in paths the AI build will
   write to, so the leak boundary is explicit and reserved up front. Candidates
   (final names set when the lab targets are written):
   - `adapters/windows-tsf/**/*.paid.dll` (or a `dist-ai/` dir)
   - `adapters/linux-ibus/**/khmerime_ibus_bridge.ai` (or a `dist-ai/` dir)
   - any Windows/Linux drop-in glue mirroring Android's `AiModelProvider.kt` path.
4. **Readiness doc.** This file, plus a short pointer from
   `docs/provider-architecture.md` noting Windows/Linux now have anchors.
5. **Verify no leak:** `cargo build` (default features) still pulls no model
   runtime; `cargo tree -p khmerime_core` unchanged. The two anchors are no-ops with
   no new deps.

### B. Lab repo (`khmerime-lab/runtime/tonle-native`) — stubs, private

1. **`#[cfg]` link anchors** in `src/lib.rs`, mirroring the existing ios/macos/android
   arms:
   ```rust
   #[cfg(all(target_os = "windows"))]
   #[no_mangle]
   pub extern "C" fn khmerime_ai_link_anchor() {
       khmerime_windows_tsf::khmerime_tsf_link_anchor();
   }
   #[cfg(all(target_os = "linux"))]
   #[no_mangle]
   pub extern "C" fn khmerime_ai_link_anchor() {
       khmerime_linux_ibus::khmerime_ibus_link_anchor();
   }
   ```
   Add the two adapters as `[target.'cfg(windows)'.dependencies]` /
   `[target.'cfg(linux)'.dependencies]` in the lab crate's `Cargo.toml`, and the
   Windows/Linux `crate-type`s (`cdylib` for Windows, `bin`-fusion strategy for Linux).
2. **`ai.mk` targets — stubbed:** `ai-windows`, `ai-windows-install`, `ai-linux`,
   `ai-linux-install`. Each documents the intended steps and **fails cleanly** with a
   message like `"AI Windows build needs a Windows build host — see …md"` until a
   build env is wired. This makes the seam discoverable and turns "produce the
   artifact" into a one-command task later.
3. **Build doc** (`AI_BUILD.md` addendum): the intended native-build recipe per OS
   (Linux via Docker; Windows via VM/PC), and where each fused artifact is dropped.

## Leak-boundary checklist (must all hold)

- [ ] Public repo changes contain **only** no-op anchor symbols, gitignore lines, and
      docs — no model, no inference code, no model-runtime dependency, no weights.
- [ ] `cargo build` with default features pulls no model runtime (unchanged from today).
- [ ] Every AI artifact and drop-in lands in a **gitignored** path or the lab.
- [ ] The public `Makefile` / TSF & IBus build configs are never modified by the AI build.
- [ ] The anchors are `#[no_mangle]` no-ops whose names carry no product/model meaning
      beyond "link anchor" — same disclosure level as Android's public `nativeCreate`.

## Deferred (explicitly out of scope now)

- Producing the actual Windows `.dll` and Linux bridge binary (needs a build host).
- Any candle cross-compilation from macOS.
- Two-level candidate-surface UI parity on these adapters is a **separate** effort —
  see `adapters/linux-ibus/docs/adr/0001-port-two-level-candidate-surface.md`.

## Branch note

This work targets the **`dev`** branch, not `experiments/flick-keyboard` where it was
planned. Move/cherry-pick the public changes onto `dev` before committing.
