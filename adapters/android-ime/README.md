# khmerime Android IME

Android keyboard adapter that bridges `KhmerInputHandler` (Kotlin) to the shared
Rust `ImeSession` via JNI. Works like the iOS keyboard: romanization input builds
a preedit, candidates appear above the keyboard, and Enter commits Khmer text.

## Quick start

```bash
# Run unit tests (host machine, no device needed)
make platform-test-android

# Install on a connected device or emulator
make platform-install-android
```

See [`docs/platforms/android.md`](../../docs/platforms/android.md) for the full
workflow including one-time NDK setup.

## Structure

```
src/lib.rs                          Rust JNI bridge (10 #[no_mangle] exports)
app/src/main/java/com/example/khmerime/
  KhmerImeSession.kt                JNI wrapper, loads native lib, parses JSON render state
  KhmerInputHandler.kt              Roman buffer + commit logic (unit-testable)
  TextProxy.kt                      Interface: insertText / deleteBackward
  InputConnectionProxy.kt           Live TextProxy backed by InputConnection
  KhmerInputMethodService.kt        InputMethodService subclass, wires keys + render
app/src/main/res/
  layout/keyboard.xml               QWERTY rows + preedit bar + candidate strip
  xml/method.xml                    IME subtype declaration (locale: km)
app/src/test/java/com/example/khmerime/
  KhmerInputHandlerTest.kt          5 JVM unit tests (real Rust session + MockTextProxy)
  MockTextProxy.kt                  In-memory TextProxy for tests
```

## Tests

Tests use the real Rust session loaded as a host-platform native library — no mocks,
no emulator. The `java.library.path` is configured in `app/build.gradle.kts` to point
at `target/debug/` so Gradle finds the dylib automatically.

```bash
make platform-test-android
```

## Building for a device

Cross-compilation requires [cargo-ndk](https://github.com/bbqsrc/cargo-ndk):

```bash
cargo install cargo-ndk
rustup target add aarch64-linux-android
make platform-build-android          # places .so in app/src/main/jniLibs/arm64-v8a/
```
