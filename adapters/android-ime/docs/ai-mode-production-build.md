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

## ⚠️ Current gap: the glue targets DEBUG, not release

`ai-android-glue` copies the manifest overlay to **`app/src/debug/`**, and
`ai-android-apk` runs **`assembleDebug`**. So today the AI build only produces a
**debug APK** — not shippable to Play Store (unsigned/oversized/slow-cold-start).

To ship, the AI glue must also apply to the **release** variant and build a
signed AAB. Steps below; the makefile change is a follow-up (`ai-android-release`).

## Producing a production AAB (manual, until `ai-android-release` exists)

1. **Signing** — create `adapters/android-ime/keystore.properties` (gitignored;
   see `keystore.properties.example`) pointing at your upload keystore. Without
   it, the release variant builds **unsigned**.

2. **Layer the AI assets** into the tree:
   ```bash
   cd khmerime-lab/runtime/tonle-native
   make -f ai.mk ai-android            # .so + assets + glue (debug variant)
   ```

3. **Make the provider manifest apply to release too.** The overlay currently
   lands in `app/src/debug/`. Copy it to the release variant as well:
   ```bash
   cp app/src/debug/AndroidManifest.xml app/src/release/AndroidManifest.xml
   ```
   (This file is gitignored — it's the AI drop-in, not source.)

4. **Build the signed release AAB:**
   ```bash
   cd adapters/android-ime
   JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" \
     ./gradlew :app:bundleRelease
   # -> app/build/outputs/bundle/release/app-release.aab
   ```

5. **Verify** the AAB contains the model + provider:
   ```bash
   unzip -l app/build/outputs/bundle/release/app-release.aab | grep -E 'tonle|libkhmerime_android_ime'
   ```
   Expect `assets/tonle/tonle-model/model.safetensors`, `vocab.trie`, and the
   arm64 `.so`.

6. **Upload** `app-release.aab` to the Play Console.

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
