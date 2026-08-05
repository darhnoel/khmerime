# Building the AI-mode Android app for Play Store (production)

The Smart-mode neural model (TonleLbah) is a **closed asset**: it lives in the
private `khmerime-lab` workspace and is layered into the public Android adapter
tree by `khmerime-lab/runtime/tonle-native/ai.mk` at build time. None of it is
committed to the public repo (`.so`, `model.safetensors`, `vocab.trie`, the
manifest overlay, and the Kotlin glue all land in gitignored paths).

This doc covers how to produce a **signed, optimized AAB** with Smart mode
built in, for Play Store upload.

## What the AI build layers in (all gitignored)

`make -f ai.mk ai-android` runs, in order:

| step | drops into | what |
|---|---|---|
| `ai-android-provider` | `app/src/main/jniLibs/…/libkhmerime_android_ime.so` | combined provider `.so`, built `cargo ndk … --release` |
| `ai-android-assets`   | `app/src/main/assets/tonle/…` | `model.safetensors`, `config.json`, `vocab.trie` |
| `ai-android-glue`     | `app/src/<variant>/AndroidManifest.xml` + Kotlin glue | the self-initializing `AiModelInitializer` provider |

The provider is inert without the `.so` symbol, so the public (OSS) build stays
Standard-only and never references any of this.

## One-command signed AAB: `ai-android-release`

`ai.mk` has a dedicated release target that wires the provider into the
**release** variant, builds a signed bundle, and **verifies** the model +
arming provider are actually in it (so you can never ship a dormant build).

### Prerequisites (one-time)
- `cargo install cargo-ndk` and `rustup target add aarch64-linux-android`
- `adapters/android-ime/keystore.properties` (gitignored) with your **upload**
  keystore — Play App Signing re-signs with the real key on upload. See
  `keystore.properties.example`. Generate a key with:
  ```bash
  keytool -genkey -v -keystore khmerime-upload.jks -keyalg RSA -keysize 2048 \
          -validity 10000 -alias khmerime
  ```

### Build
```bash
cd khmerime-lab/runtime/tonle-native
make -f ai.mk ai-android                                  # stage .so + model + glue (once)
make -f ai.mk ai-android-release \
  KHMERIME_PACKAGE_VERSION=1.0.0 \
  KHMERIME_ANDROID_VERSION_CODE=1                          # bump the CODE every upload
```
Output: `adapters/android-ime/app/build/outputs/bundle/release/app-release.aab`,
already signed and verified to contain `tonle-model/model.safetensors` and the
`AiModelInitializer` provider.

**Do NOT** use the public `build_aab.sh` / `android-package` — those rebuild the
FREE `.so` over the provider-armed one and ship a bundle with the model present
but no provider (dormant Smart mode). Always use `ai-android-release`.

- `KHMERIME_PACKAGE_VERSION` → `versionName` (the Play "app version").
- `KHMERIME_ANDROID_VERSION_CODE` → `versionCode`, **must be unique and
  increasing on every Play upload**.

## Runtime notes for production
- **Cold start**: the model (~18.6 MB) is extracted to `filesDir` on first launch
  (candle needs a real file path). This runs on a background thread; the keyboard
  opens instantly in Standard/lookup mode and Smart mode arms shortly after. See
  `cold-start-latency-fix.md`.
- **Release `.so` is mandatory** — a debug `.so` makes the decoder ~100× slower
  (`ai.mk` already builds `--release`; do not regress this).
- **App size**: the AAB carries ~23 MB of model assets. Consider Play Asset
  Delivery or an int8 model (~5 MB) later to shrink the download.

## Follow-ups
- Add an `ai-android-release` target to `ai.mk` that applies the glue to the
  release variant and runs `bundleRelease` — so the manual steps 3–4 become one
  command.
- Shrink the model (fp32 → int8) for download size and the iOS 77 MB cap.
