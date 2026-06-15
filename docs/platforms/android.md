# Android (InputMethodService)

For the shared platform workflow, read [`docs/platforms/README.md`](README.md).

## Adapter

- Crate: `adapters/android-ime`
- Language: Kotlin + Rust (JNI bridge)
- Runtime boundary: `KhmerInputHandler` (Kotlin) ↔ `KhmerImeSession` (JNI) ↔ `ImeSession` (Rust)

## Architecture

```
KhmerInputMethodService   ← Android InputMethodService subclass
  └── KhmerInputHandler   ← roman buffer + commit logic (pure Kotlin, unit-testable)
        ├── TextProxy      ← interface (InputConnectionProxy on device, MockTextProxy in tests)
        └── KhmerImeSession ← JNI wrapper around the Rust ImeSession
              └── khmerime_android_ime.so  ← Rust crate compiled via cargo-ndk
```

The session is the single source of truth (same `ImeSession` used by iOS and macOS). The
Kotlin layer owns speculative roman insertion and bulk-delete-then-commit on Enter — the
same roman buffer pattern as the iOS keyboard.

## Lifecycle Mapping

| Android callback | Handler call |
| --- | --- |
| `onStartInput` | `handler.focusIn()` |
| `onFinishInput` | `handler.focusOut()` |
| Key button tap | `handler.sendChar/sendBackspace/sendSpace/sendReturn()` |
| Candidate tap | direct `InputConnectionProxy.insertText` + `handler.focusIn()` |

## Testing (no device required)

Unit tests run on the host JVM using the native dylib built for the host machine:

```bash
make platform-test-android
```

This runs `cargo build -p khmerime_android_ime` (produces the host dylib), then
`./gradlew :app:testDebugUnitTest` which loads it via `java.library.path`.

To run Gradle directly:

```bash
cargo build -p khmerime_android_ime
cd adapters/android-ime
JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" \
  PATH="$JAVA_HOME/bin:$PATH" \
  ./gradlew :app:testDebugUnitTest
```

## Building for a Device

### One-time setup

**1. Install the NDK via Android Studio**

Open Android Studio and go to:
**Android Studio → Settings → Languages & Frameworks → Android SDK → SDK Tools tab**

Tick **NDK (Side by side)** and click **Apply**. Wait for the download to finish, then confirm with:

```bash
ls ~/Library/Android/sdk/ndk/
# should print something like: 27.0.12077973
```

**2. Add Android SDK environment variables**

Add this block to your `~/.zshrc` (replace the NDK version number with what `ls` showed above):

```bash
# Android SDK
export ANDROID_HOME="$HOME/Library/Android/sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/30.0.14904198"
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
```

Then reload your shell:

```bash
source ~/.zshrc
```

Verify it works:

```bash
echo $ANDROID_NDK_HOME
# /Users/<you>/Library/Android/sdk/ndk/30.0.14904198

adb version
# Android Debug Bridge version ...
```

**3. Install cargo-ndk and add Rust Android targets**

```bash
cargo install cargo-ndk
rustup target add aarch64-linux-android   # physical ARM64 device
rustup target add x86_64-linux-android    # x86_64 emulator
```

### Build

```bash
make platform-build-android                        # arm64-v8a (default — physical device or Apple Silicon emulator)
make platform-build-android ANDROID_ABI=x86_64    # x86_64 emulator on Intel Mac
```

> **Apple Silicon (M1/M2/M3) note:** the Android emulator runs `arm64-v8a` natively on
> Apple Silicon, so use the default `arm64-v8a` ABI even for the emulator.

This cross-compiles the Rust crate with cargo-ndk, places the `.so` in
`app/src/main/jniLibs/<ABI>/`, and runs `./gradlew :app:assembleDebug`.

## Installing on a Device

Connect a device with USB debugging enabled (or start an emulator), then:

```bash
make platform-install-android
```

This installs the APK and runs `adb shell ime enable`. Complete activation in:
**Settings → General management → Keyboard → On-screen keyboards → enable KhmerIME**

For fast iteration after code changes:

```bash
make platform-reinstall-android
```

## Official References

- Create an input method: <https://developer.android.com/develop/ui/views/touch-and-input/creating-input-method>
- `InputMethodService` API: <https://developer.android.com/reference/android/inputmethodservice/InputMethodService>
