.PHONY: help web web-release web-phone web-local desktop stats suggest suggest-wfst suggest-shadow shadow-eval data-split data-build data-check lexicon-editor visualize-lexicon visualize-lexicon-streamlit download-page fmt test test-golden test-ui platform-check platform-check-linux platform-check-android platform-check-ios platform-check-macos platform-check-windows platform-ios-assets platform-android-assets platform-build-ios platform-test-ios platform-install-ios-sim platform-reinstall-ios-sim platform-install-ios-device platform-reinstall-ios-device platform-build-macos platform-diagnose-macos platform-install-macos platform-reinstall-macos platform-build-windows platform-install-windows platform-uninstall-windows platform-reinstall-windows platform-smoke-windows-notepad platform-smoke-windows-notepad-python windows-package linux-package macos-package android-package ios-package package ibus-install ibus-uninstall ibus-smoke paper-current paper-current-clean platform-test-android platform-build-android android-adb-device platform-install-android platform-reinstall-android setup-hooks

DX ?= dx
PYTHON ?= python3
APP_DIR := apps/dioxus-app
WEB_LOCAL_PORT ?= 4174
CLI := cargo run -p khmerime_lookup_cli --bin lookup_cli --
QUERY ?= tver
MODE ?= shadow
QUERIES ?=
OUTPUT ?=
PAPER_CURRENT_DIR := papers/current-implementation
PAPER_CURRENT_TEX := khmerime_current_implementation_paper.tex
WINDOWS_TSF_TARGET ?= x86_64-pc-windows-msvc
WINDOWS_TSF_TARGET_DIR ?= target/windows-tsf
WINDOWS_TSF_DEV_TARGET_DIR ?= target/windows-tsf-dev
WINDOWS_TSF_REINSTALL_STAMP ?= $(shell if command -v powershell >/dev/null 2>&1; then powershell -NoProfile -Command "Get-Date -Format yyyyMMddHHmmss"; else date +%Y%m%d%H%M%S; fi)
# Freeze the stamp once per make invocation so all deploy paths match.
WINDOWS_TSF_REINSTALL_STAMP := $(WINDOWS_TSF_REINSTALL_STAMP)
WINDOWS_TSF_DEPLOY_DIR ?= target/windows-tsf-deploy/$(WINDOWS_TSF_REINSTALL_STAMP)
WINDOWS_TSF_DLL := $(WINDOWS_TSF_TARGET_DIR)/$(WINDOWS_TSF_TARGET)/debug/khmerime_windows_tsf.dll
WINDOWS_TSF_DEV_DLL := $(WINDOWS_TSF_DEV_TARGET_DIR)/$(WINDOWS_TSF_TARGET)/debug/khmerime_windows_tsf.dll
WINDOWS_TSF_DEPLOY_DLL := $(WINDOWS_TSF_DEPLOY_DIR)/khmerime_windows_tsf.dll
# Absolute path — regsvr32 runs elevated and its cwd may not be the project root.
WINDOWS_TSF_DLL_ABS := $(subst /,\,$(CURDIR)/$(WINDOWS_TSF_DLL))
WINDOWS_TSF_DEV_DLL_ABS := $(subst /,\,$(CURDIR)/$(WINDOWS_TSF_DEV_DLL))
WINDOWS_TSF_DEPLOY_DLL_ABS := $(subst /,\,$(CURDIR)/$(WINDOWS_TSF_DEPLOY_DLL))
WINDOWS_TSF_SMOKE_DELAY ?= 8

IOS_ADAPTER_DIR     := adapters/ios-keyboard
IOS_TARGET_DEVICE   := aarch64-apple-ios
IOS_TARGET_SIM      := aarch64-apple-ios-sim
IOS_LIB_NAME        := libkhmerime_ios_keyboard.a
IOS_BINDGEN_OUT     := $(IOS_ADAPTER_DIR)/swift/KhmerIMEKeyboard/Generated
IOS_XCFRAMEWORK_OUT := $(IOS_ADAPTER_DIR)/swift/Frameworks/KhmerIME.xcframework
IOS_DERIVED_DATA    ?= /tmp/khmerime-ios-build
IOS_DEVICE_ID       ?=
IOS_DEVICE_APP      := $(IOS_DERIVED_DATA)/Build/Products/Debug-iphoneos/KhmerIME.app
IOS_SIM_ID          ?= $(shell xcrun simctl list devices available | grep 'iPhone' | head -1 | sed 's/.*(\([A-F0-9-]*\)).*/\1/')
IOS_SIM_BUNDLE_ID   := com.khmerime.KhmerIME
IOS_SIM_APP         := $(HOME)/Library/Developer/Xcode/DerivedData/KhmerIME-*/Build/Products/Debug-iphonesimulator/KhmerIME.app

MACOS_ADAPTER_DIR       := adapters/macos-imk
MACOS_TARGET            := aarch64-apple-darwin
MACOS_TARGET_X86        := x86_64-apple-darwin
MACOS_LIB_NAME          := libkhmerime_macos_imk.a
MACOS_BINDGEN_OUT       := $(MACOS_ADAPTER_DIR)/swift/KhmerIMEMacOS/Generated
MACOS_XCFRAMEWORK_OUT   := $(MACOS_ADAPTER_DIR)/swift/Frameworks/KhmerIME.xcframework
MACOS_INPUT_METHODS_DIR := $(HOME)/Library/Input\ Methods
MACOS_BUILD_DIR         := /tmp/khmerime-macos-build
MACOS_BUILD_APP         := $(MACOS_BUILD_DIR)/Build/Products/Release/KhmerIMEMacOS.app
MACOS_ENTITLEMENTS      := $(MACOS_ADAPTER_DIR)/swift/KhmerIMEMacOS/KhmerIMEMacOS.entitlements
# Verified flow for macOS 26 (Tahoe): the input-source scanner silently ignores
# bundles that are not Developer ID signed + hardened + notarized + stapled.
# CODE_SIGN_INJECT_BASE_ENTITLEMENTS=NO is required — without it Xcode injects
# get-task-allow into the Release signature and notarization rejects the app.
# Provide local values in adapters/macos-imk/macos-signing.local.mk, or pass
# MACOS_SIGNING_CONFIG=/path/to/config.mk for another config file.
MACOS_SIGNING_CONFIG    ?= $(MACOS_ADAPTER_DIR)/macos-signing.local.mk
MACOS_CODE_SIGN_IDENTITY ?=
MACOS_INSTALLER_SIGN_IDENTITY ?=
MACOS_TEAM_ID            ?=
MACOS_NOTARY_PROFILE     ?= khmerime-notary
-include $(MACOS_SIGNING_CONFIG)
MACOS_NOTARIZE_ZIP       := /tmp/khmerime-macos-notarize.zip
MACOS_LSREGISTER         := /System/Library/Frameworks/CoreServices.framework/Versions/Current/Frameworks/LaunchServices.framework/Versions/Current/Support/lsregister

ANDROID_ADAPTER_DIR  := adapters/android-ime
ANDROID_ABI         ?= arm64-v8a
ANDROID_JNI_LIBS    := $(ANDROID_ADAPTER_DIR)/app/src/main/jniLibs
ANDROID_APK         := $(ANDROID_ADAPTER_DIR)/app/build/outputs/apk/debug/app-debug.apk
ANDROID_PACKAGE     := com.khmerime.debug
ANDROID_IME_SERVICE := $(ANDROID_PACKAGE)/com.khmerime.service.KhmerInputMethodService
ANDROID_LEGACY_PACKAGE := com.example.khmerime
# Android Studio bundles a JDK at this path on macOS. Override if your JDK is elsewhere.
ANDROID_JAVA_HOME   ?= /Applications/Android Studio.app/Contents/jbr/Contents/Home
MACOS_XCODE_SIGNING_ARGS := CODE_SIGN_STYLE=Manual \
	CODE_SIGN_IDENTITY=- \
	CODE_SIGN_INJECT_BASE_ENTITLEMENTS=NO

help:
	@printf "%s\n" \
	"khmerime developer commands" \
	"" \
	"  make web                         Run the Dioxus web app" \
	"  make web-release                 Build deployable web artifacts under dist/web-release" \
	"                                   Optional: WEB_BASE_PATH=khmerime-beta (or KHMERIME_BASE_PATH=/khmerime-beta)" \
	"  make web-phone                   Run the web app on a phone-accessible host" \
	"  make web-local                   Run the web app on localhost with asset sync" \
	"  make desktop                     Run the desktop app" \
	"  make stats                       Print lexicon entry count" \
	"  make suggest QUERY=tver          Print legacy-mode suggestions" \
	"  make suggest-wfst QUERY=tver     Print weighted-span suggestions (wfst alias)" \
	"  make suggest-shadow QUERY=tver   Print shadow-mode suggestions" \
	"  make shadow-eval QUERIES=path/to/queries.txt [MODE=shadow|weighted-span|wfst|hybrid] [OUTPUT=report.txt]" \
	"  make data-split                  Split data/roman_lookup.csv into reviewable chunk CSVs" \
	"  make data-build                  Generate data/roman_lookup.csv from chunk CSVs" \
	"  make data-check                  Validate lexicon chunks and generated runtime data" \
	"  make lexicon-editor              Run the local lexicon chunk editor" \
	"  make visualize-lexicon           Generate lightweight lexicon relationship views under dist/" \
	"  make visualize-lexicon-streamlit Launch the optional Streamlit explorer for the generated views" \
	"  make fmt                         Run cargo fmt" \
	"  make test                        Run cargo test" \
	"  make test-golden                 Run the weighted-span golden snapshot test" \
	"  make test-ui                     Run the browser/UI Python test file" \
	"  make platform-check              Check all native platform adapter crates" \
	"  make platform-check-<platform>   Check one adapter: linux, android, ios, macos, windows" \
	"  make platform-test-android       Run Android JVM unit tests on the host machine (no device needed)" \
	"  make platform-build-android     Cross-compile Rust via cargo-ndk (ANDROID_ABI=arm64-v8a) + assemble debug APK" \
	"  make platform-install-android   Build and adb install the debug APK, enable the IME on connected device" \
	"  make platform-reinstall-android Fast loop: rebuild Rust + APK, reinstall on connected device" \
	"  make platform-test-ios           Run iOS XCTest suite on the simulator (IOS_SIM_ID=...)" \
	"  make platform-build-ios          Build iOS static libs, generate UniFFI Swift bindings, assemble XCFramework" \
	"  make platform-install-ios-sim    Full build (Rust + app) then install to simulator" \
	"  make platform-reinstall-ios-sim  Fast loop: rebuild Swift app only, reinstall on simulator" \
	"  make platform-install-ios-device Full build (Rust + app) then install to connected iPhone" \
	"  make platform-reinstall-ios-device Fast loop: rebuild Swift app only, install to connected iPhone" \
	"  make platform-build-macos        Build macOS static libs, generate UniFFI Swift bindings, assemble XCFramework" \
	"  make platform-diagnose-macos     Inspect macOS signing, install paths, and Gatekeeper status" \
	"  make platform-install-macos      Full build (Rust + app), notarize, install to ~/Library/Input Methods/" \
	"  make platform-reinstall-macos    Fast loop: rebuild app only, notarize, swap install, rescan (no logout)" \
	"  make platform-build-windows      Build the Windows TSF DLL target under target/windows-tsf/" \
	"  make platform-install-windows    Build and register the Windows TSF DLL with regsvr32" \
	"  make platform-uninstall-windows  Unregister the Windows TSF DLL with regsvr32 /u" \
	"  make platform-reinstall-windows  Build once, copy to a fresh DLL path, and re-register" \
	"  make platform-smoke-windows-notepad  Launch Notepad and check for TSF crash events" \
	"  make platform-smoke-windows-notepad-python  Python Notepad smoke with clipboard/log output" \
	"  make windows-package            Build the Windows TSF MSI under dist/windows/" \
	"  make linux-package               Build the Linux IBus .deb package under dist/linux/" \
	"  make macos-package               Build the macOS .pkg (unsigned) under dist/macos/" \
	"  make android-package             Build the Android release .aab under dist/android/" \
	"  make ios-package                 Build the iOS .xcarchive (unsigned) under dist/ios/" \
	"  make package                     Build every artifact this host can, into dist/ (see PACKAGING.md)" \
	"  make ibus-install                Build and install KhmerIME IBus engine files (may use sudo)" \
	"  make ibus-uninstall              Remove KhmerIME IBus engine files" \
	"  make ibus-smoke                  Run bridge + IBus discovery smoke checks" \
	"  make paper-current               Build the current implementation paper PDF" \
	"  make paper-current-clean         Remove LaTeX build byproducts from the paper folder" \
	"" \
	"  make setup-hooks                 Activate .githooks/ for this clone (run once after git clone)" \
	"" \
	"Read docs/development.md for the workflow and command details."

web:
	cd $(APP_DIR) && $(DX) serve

web-release:
	bash scripts/web/build_release.sh

web-phone:
	bash scripts/web/serve_phone.sh

web-local:
	ADDR=127.0.0.1 PORT=$(WEB_LOCAL_PORT) bash scripts/web/serve_phone.sh

desktop:
	cd $(APP_DIR) && $(DX) serve --platform desktop

stats:
	$(CLI) stats

suggest:
	$(CLI) suggest "$(QUERY)"

suggest-wfst:
	$(CLI) --decoder-mode wfst suggest "$(QUERY)"

suggest-shadow:
	$(CLI) --decoder-mode shadow suggest "$(QUERY)"

shadow-eval:
	@if [ -z "$(QUERIES)" ]; then \
		echo "Set QUERIES=path/to/queries.txt"; \
		exit 2; \
	fi
	@if [ -n "$(OUTPUT)" ]; then \
		$(CLI) --decoder-mode "$(MODE)" --output "$(OUTPUT)" shadow-eval "$(QUERIES)"; \
	else \
		$(CLI) --decoder-mode "$(MODE)" shadow-eval "$(QUERIES)"; \
	fi

data-split:
	python3 scripts/data/lexicon/manage_lexicon_chunks.py split

data-build:
	python3 scripts/data/lexicon/manage_lexicon_chunks.py build

data-check:
	python3 scripts/data/lexicon/manage_lexicon_chunks.py check

lexicon-editor:
	python3 tools/lexicon-editor/server.py

visualize-lexicon:
	python3 scripts/data/lexicon/visualize_roman_lookup.py

visualize-lexicon-streamlit:
	python3 -m streamlit run scripts/data/lexicon/visualize_roman_lookup_streamlit.py

fmt:
	cargo fmt --all

test:
	cargo test

test-golden:
	cargo test --test decoder_golden

test-ui:
	python3 -m pytest tests/test_web_ui.py

platform-check: platform-check-linux platform-check-android platform-check-ios platform-check-macos platform-check-windows

platform-check-linux:
	cargo check -p khmerime_linux_ibus

platform-check-android:
	cargo check -p khmerime_android_ime

platform-check-ios:
	cargo check -p khmerime_ios_keyboard

platform-check-macos:
	cargo check -p khmerime_macos_imk

platform-check-windows:
	cargo check -p khmerime_windows_tsf

# Build the host dylib (used by JVM unit tests) then run Gradle unit tests.
# No Android device or emulator is required.
IOS_XCPROJECT       := $(IOS_ADAPTER_DIR)/swift/KhmerIME.xcodeproj
IOS_TEST_SCHEME     := KhmerIMEKeyboardTests
IOS_APP_SCHEME      := KhmerIME

platform-test-ios:
	xcodebuild test \
		-project $(IOS_XCPROJECT) \
		-scheme $(IOS_TEST_SCHEME) \
		-destination 'platform=iOS Simulator,id=$(IOS_SIM_ID)'

# Full install: build Rust xcframework first, then build + install the app.
platform-install-ios-sim: platform-build-ios
	$(MAKE) platform-reinstall-ios-sim

# Fast loop: rebuild Swift app only (assumes xcframework already built) and install.
platform-reinstall-ios-sim: platform-ios-assets
	xcodegen generate --spec $(IOS_ADAPTER_DIR)/swift/project.yml --project $(IOS_ADAPTER_DIR)/swift
	xcodebuild build \
		-project $(IOS_XCPROJECT) \
		-scheme $(IOS_APP_SCHEME) \
		-destination 'platform=iOS Simulator,id=$(IOS_SIM_ID)'
	xcrun simctl boot $(IOS_SIM_ID) 2>/dev/null || true
	xcrun simctl install $(IOS_SIM_ID) $(IOS_SIM_APP)
	open -a Simulator
	@echo "Installed. In Simulator: Settings → General → Keyboard → Keyboards → Add New Keyboard → KhmerIME"

# Full install to a connected physical device: build Rust xcframework first, then build + install.
platform-install-ios-device: platform-build-ios
	$(MAKE) platform-reinstall-ios-device

# Fast loop: rebuild Swift app only and install to connected physical device.
platform-reinstall-ios-device: platform-ios-assets
	@if [ -z "$(IOS_DEVICE_ID)" ]; then \
		echo "Set IOS_DEVICE_ID to the connected device name, UDID, serial number, or ECID."; \
		echo "Example: make platform-reinstall-ios-device IOS_DEVICE_ID=iPhonak"; \
		exit 2; \
	fi
	xcodegen generate --spec $(IOS_ADAPTER_DIR)/swift/project.yml --project $(IOS_ADAPTER_DIR)/swift
	xcodebuild build \
		-project $(IOS_XCPROJECT) \
		-scheme $(IOS_APP_SCHEME) \
		-destination 'generic/platform=iOS' \
		-derivedDataPath $(IOS_DERIVED_DATA)
	xcrun devicectl device install app --device "$(IOS_DEVICE_ID)" "$(IOS_DEVICE_APP)"
	@echo "Installed. On device: Settings → General → Keyboard → Keyboards → Add New Keyboard → KhmerIME"

platform-test-android: platform-android-assets
	cargo build -p khmerime_android_ime
	cd $(ANDROID_ADAPTER_DIR) && JAVA_HOME="$(ANDROID_JAVA_HOME)" PATH="$(ANDROID_JAVA_HOME)/bin:$(PATH)" ./gradlew :app:testDebugUnitTest

# Cross-compile Rust for the device ABI via cargo-ndk, then assemble the APK.
# Prerequisites: cargo install cargo-ndk && rustup target add aarch64-linux-android
# Override ABI with: make platform-build-android ANDROID_ABI=x86_64
platform-build-android: platform-android-assets
	# --release: an unoptimized (debug) .so makes the decoder ~100x slower on-device
	# (multi-second suggest() on long input). iOS/macOS already build release; Android must too.
	cargo ndk -t $(ANDROID_ABI) -o $(ANDROID_JNI_LIBS) build -p khmerime_android_ime --release
	cd $(ANDROID_ADAPTER_DIR) && JAVA_HOME="$(ANDROID_JAVA_HOME)" PATH="$(ANDROID_JAVA_HOME)/bin:$(PATH)" ./gradlew :app:assembleDebug

android-adb-device:
	@adb get-state >/dev/null 2>&1 || { \
		echo "No Android device/emulator found. Start an emulator or connect a USB-debugging device, then run: adb devices" >&2; \
		exit 1; \
	}

# Build, install on the connected device, and enable the IME.
# Requires: adb in PATH and a connected device/emulator with USB debugging enabled.
platform-install-android: android-adb-device platform-build-android
	-adb uninstall $(ANDROID_LEGACY_PACKAGE) >/dev/null 2>&1
	adb install -r $(ANDROID_APK)
	adb shell ime enable $(ANDROID_IME_SERVICE)
	adb shell ime set $(ANDROID_IME_SERVICE)
	@echo "Installed. Tap any text field on the device to open KhmerIME."

# Fast loop: rebuild Rust + APK and reinstall (assumes cargo-ndk already set up).
platform-reinstall-android: android-adb-device platform-android-assets
	cargo ndk -t $(ANDROID_ABI) -o $(ANDROID_JNI_LIBS) build -p khmerime_android_ime --release
	cd $(ANDROID_ADAPTER_DIR) && JAVA_HOME="$(ANDROID_JAVA_HOME)" PATH="$(ANDROID_JAVA_HOME)/bin:$(PATH)" ./gradlew :app:assembleDebug
	-adb uninstall $(ANDROID_LEGACY_PACKAGE) >/dev/null 2>&1
	adb install -r $(ANDROID_APK)
	adb shell ime enable $(ANDROID_IME_SERVICE)
	adb shell ime set $(ANDROID_IME_SERVICE)

platform-ios-assets:
	$(PYTHON) scripts/dev/render_mobile_logo_assets.py ios

platform-android-assets:
	$(PYTHON) scripts/dev/render_mobile_logo_assets.py android

platform-build-ios: platform-ios-assets
	cargo build -p khmerime_ios_keyboard --target $(IOS_TARGET_DEVICE) --release
	cargo build -p khmerime_ios_keyboard --target $(IOS_TARGET_SIM) --release
	mkdir -p $(IOS_BINDGEN_OUT)
	cargo run -p khmerime_ios_keyboard --bin uniffi-bindgen -- \
		--swift-sources target/$(IOS_TARGET_DEVICE)/release/$(IOS_LIB_NAME) $(IOS_BINDGEN_OUT)
	cargo run -p khmerime_ios_keyboard --bin uniffi-bindgen -- \
		--headers target/$(IOS_TARGET_DEVICE)/release/$(IOS_LIB_NAME) $(IOS_BINDGEN_OUT)
	cargo run -p khmerime_ios_keyboard --bin uniffi-bindgen -- \
		--modulemap target/$(IOS_TARGET_DEVICE)/release/$(IOS_LIB_NAME) $(IOS_BINDGEN_OUT)
	# XCFramework needs module.modulemap (Clang's implicit name) for canImport() to work.
	mkdir -p /tmp/khmerime-xcfw-headers
	cp $(IOS_BINDGEN_OUT)/khmerime_ios_keyboardFFI.h /tmp/khmerime-xcfw-headers/
	cp $(IOS_BINDGEN_OUT)/khmerime_ios_keyboard.modulemap /tmp/khmerime-xcfw-headers/module.modulemap
	# Module name must match the Swift import: khmerime_ios_keyboardFFI (not khmerime_ios_keyboard)
	sed -i '' 's/^module khmerime_ios_keyboard {/module khmerime_ios_keyboardFFI {/' /tmp/khmerime-xcfw-headers/module.modulemap
	mkdir -p $(IOS_ADAPTER_DIR)/swift/Frameworks
	rm -rf $(IOS_XCFRAMEWORK_OUT)
	xcodebuild -create-xcframework \
		-library target/$(IOS_TARGET_DEVICE)/release/$(IOS_LIB_NAME) \
		-headers /tmp/khmerime-xcfw-headers \
		-library target/$(IOS_TARGET_SIM)/release/$(IOS_LIB_NAME) \
		-headers /tmp/khmerime-xcfw-headers \
		-output $(IOS_XCFRAMEWORK_OUT)
	rm -rf /tmp/khmerime-xcfw-headers
	cd $(IOS_ADAPTER_DIR)/swift && xcodegen generate

platform-build-macos:
	cargo build -p khmerime_macos_imk --target $(MACOS_TARGET) --release
	cargo build -p khmerime_macos_imk --target $(MACOS_TARGET_X86) --release
	mkdir -p $(MACOS_BINDGEN_OUT)
	cargo run -p khmerime_macos_imk --bin uniffi-bindgen -- \
		--swift-sources target/$(MACOS_TARGET)/release/$(MACOS_LIB_NAME) $(MACOS_BINDGEN_OUT)
	cargo run -p khmerime_macos_imk --bin uniffi-bindgen -- \
		--headers target/$(MACOS_TARGET)/release/$(MACOS_LIB_NAME) $(MACOS_BINDGEN_OUT)
	cargo run -p khmerime_macos_imk --bin uniffi-bindgen -- \
		--modulemap target/$(MACOS_TARGET)/release/$(MACOS_LIB_NAME) $(MACOS_BINDGEN_OUT)
	# lipo arm64 + x86_64 into a single universal static lib before xcframework
	mkdir -p /tmp/khmerime-macos-universal
	lipo -create \
		target/$(MACOS_TARGET)/release/$(MACOS_LIB_NAME) \
		target/$(MACOS_TARGET_X86)/release/$(MACOS_LIB_NAME) \
		-output /tmp/khmerime-macos-universal/$(MACOS_LIB_NAME)
	mkdir -p /tmp/khmerime-macos-xcfw-headers
	cp $(MACOS_BINDGEN_OUT)/khmerime_macos_imkFFI.h /tmp/khmerime-macos-xcfw-headers/
	cp $(MACOS_BINDGEN_OUT)/khmerime_macos_imk.modulemap /tmp/khmerime-macos-xcfw-headers/module.modulemap
	sed -i '' 's/^module khmerime_macos_imk {/module khmerime_macos_imkFFI {/' /tmp/khmerime-macos-xcfw-headers/module.modulemap
	mkdir -p $(MACOS_ADAPTER_DIR)/swift/Frameworks
	rm -rf $(MACOS_XCFRAMEWORK_OUT)
	xcodebuild -create-xcframework \
		-library /tmp/khmerime-macos-universal/$(MACOS_LIB_NAME) \
		-headers /tmp/khmerime-macos-xcfw-headers \
		-output $(MACOS_XCFRAMEWORK_OUT)
	rm -rf /tmp/khmerime-macos-xcfw-headers /tmp/khmerime-macos-universal
	cd $(MACOS_ADAPTER_DIR)/swift && xcodegen generate

platform-diagnose-macos:
	@echo "== Code signing identities =="
	@security find-identity -v -p codesigning || true
	@echo
	@echo "== Xcode provisioning teams =="
	@defaults read com.apple.dt.Xcode IDEProvisioningTeams 2>/dev/null || echo "No Xcode provisioning teams configured."
	@echo
	@echo "== Installed KhmerIME bundles =="
	@for app in "$(HOME)/Library/Input Methods/KhmerIMEMacOS.app" "/Library/Input Methods/KhmerIMEMacOS.app"; do \
		if [ -d "$$app" ]; then \
			echo "found: $$app"; \
			codesign --verify --deep --strict --verbose=2 "$$app" 2>&1 || true; \
			spctl --assess --verbose "$$app" 2>&1 || true; \
		else \
			echo "missing: $$app"; \
		fi; \
	done

# Full install: rebuild Rust libs + XCFramework first, then the app flow below.
platform-install-macos: platform-build-macos
	$(MAKE) platform-reinstall-macos

# Fast loop for Swift/asset-only changes: rebuild the app (assumes the
# XCFramework from platform-build-macos is already in place), notarize,
# staple, swap the installed copy, and rescan input sources — no logout.
platform-reinstall-macos:
	@if [ -z "$(strip $(MACOS_CODE_SIGN_IDENTITY))" ] || [ -z "$(strip $(MACOS_TEAM_ID))" ]; then \
		echo "error: platform-reinstall-macos requires MACOS_CODE_SIGN_IDENTITY and MACOS_TEAM_ID." >&2; \
		echo "example: make platform-reinstall-macos MACOS_CODE_SIGN_IDENTITY=<sha1> MACOS_TEAM_ID=<team>" >&2; \
		exit 2; \
	elif printf '%s\n%s\n' "$(MACOS_CODE_SIGN_IDENTITY)" "$(MACOS_TEAM_ID)" | grep -q '[<>]'; then \
		echo "error: replace placeholder values in $(MACOS_SIGNING_CONFIG) before installing." >&2; \
		exit 2; \
	fi
	xcodebuild -project $(MACOS_ADAPTER_DIR)/swift/KhmerIMEMacOS.xcodeproj \
		-scheme KhmerIMEMacOS \
		-configuration Release \
		-derivedDataPath $(MACOS_BUILD_DIR) \
		$(MACOS_XCODE_SIGNING_ARGS) \
		build
	codesign --force --deep --options runtime --timestamp \
		--entitlements $(MACOS_ENTITLEMENTS) \
		--sign $(MACOS_CODE_SIGN_IDENTITY) \
		$(MACOS_BUILD_APP)
	codesign --verify --deep --strict --verbose=2 $(MACOS_BUILD_APP)
	rm -f $(MACOS_NOTARIZE_ZIP)
	ditto -c -k --keepParent $(MACOS_BUILD_APP) $(MACOS_NOTARIZE_ZIP)
	xcrun notarytool submit $(MACOS_NOTARIZE_ZIP) --keychain-profile $(MACOS_NOTARY_PROFILE) --wait
	xcrun stapler staple $(MACOS_BUILD_APP)
	xcrun stapler validate $(MACOS_BUILD_APP)
	codesign --verify --deep --strict --verbose=2 $(MACOS_BUILD_APP)
	spctl --assess --type execute --verbose=2 $(MACOS_BUILD_APP)
	killall KhmerIMEMacOS 2>/dev/null || true
	mkdir -p $(MACOS_INPUT_METHODS_DIR)
	rm -rf $(MACOS_INPUT_METHODS_DIR)/KhmerIMEMacOS.app
	ditto $(MACOS_BUILD_APP) $(MACOS_INPUT_METHODS_DIR)/KhmerIMEMacOS.app
	codesign --verify --deep --strict --verbose=2 $(MACOS_INPUT_METHODS_DIR)/KhmerIMEMacOS.app
	$(MACOS_LSREGISTER) -u $(MACOS_BUILD_APP) >/dev/null 2>&1 || true
	$(MACOS_LSREGISTER) -f -R -trusted $(MACOS_INPUT_METHODS_DIR)/KhmerIMEMacOS.app
	rm -f "$$(getconf DARWIN_USER_CACHE_DIR)"com.apple.IntlDataCache.le*
	killall TextInputSwitcher TextInputMenuAgent imklaunchagent 2>/dev/null || true
	swift scripts/platforms/macos/imk/tis_check.swift
	@echo "Reinstalled — no logout needed. Add/keep 'Khmer IME' in System Settings → Keyboard → Input Sources."

platform-build-windows:
	cargo build -p khmerime_windows_tsf --target $(WINDOWS_TSF_TARGET) --target-dir $(WINDOWS_TSF_TARGET_DIR)

platform-install-windows: platform-build-windows
	powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process -FilePath regsvr32.exe -ArgumentList @('$(WINDOWS_TSF_DLL_ABS)') -Verb RunAs -Wait"

platform-uninstall-windows:
	powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process -FilePath regsvr32.exe -ArgumentList @('/u', '$(WINDOWS_TSF_DLL_ABS)') -Verb RunAs -Wait"

# Build in an unregistered stable Cargo target dir for incremental speed, then
# register a copied DLL from a fresh deploy dir so loaded TSF DLLs do not block rebuilds.
platform-reinstall-windows:
	powershell -NoProfile -ExecutionPolicy Bypass -Command "Stop-Process -Name notepad -Force -ErrorAction SilentlyContinue; exit 0"
	cargo build -p khmerime_windows_tsf --target $(WINDOWS_TSF_TARGET) --target-dir $(WINDOWS_TSF_DEV_TARGET_DIR)
	powershell -NoProfile -ExecutionPolicy Bypass -Command "New-Item -ItemType Directory -Force '$(WINDOWS_TSF_DEPLOY_DIR)' | Out-Null; Copy-Item -Force '$(WINDOWS_TSF_DEV_DLL)' '$(WINDOWS_TSF_DEPLOY_DLL)'"
	powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process -FilePath regsvr32.exe -ArgumentList @('$(WINDOWS_TSF_DEPLOY_DLL_ABS)') -Verb RunAs -Wait"
	powershell -NoProfile -ExecutionPolicy Bypass -Command "Stop-Process -Name ctfmon -Force -ErrorAction SilentlyContinue; Start-Process ctfmon.exe"

platform-smoke-windows-notepad:
	powershell -NoProfile -ExecutionPolicy Bypass -File scripts/platforms/windows/tsf/notepad_smoke.ps1

platform-smoke-windows-notepad-python:
	python scripts/platforms/windows/tsf/notepad_smoke.py --delay $(WINDOWS_TSF_SMOKE_DELAY)

windows-package:
	powershell -NoProfile -ExecutionPolicy Bypass -File scripts/platforms/windows/tsf/build_msi.ps1

linux-package:
	bash scripts/platforms/linux/ibus/build_deb.sh

macos-package:
	MACOS_CODE_SIGN_IDENTITY='$(MACOS_CODE_SIGN_IDENTITY)' \
	MACOS_INSTALLER_SIGN_IDENTITY='$(MACOS_INSTALLER_SIGN_IDENTITY)' \
	MACOS_TEAM_ID='$(MACOS_TEAM_ID)' \
	MACOS_NOTARY_PROFILE='$(MACOS_NOTARY_PROFILE)' \
	MACOS_ENTITLEMENTS='$(MACOS_ENTITLEMENTS)' \
	bash scripts/platforms/macos/imk/build_pkg.sh

android-package:
	bash scripts/platforms/android/ime/build_aab.sh

ios-package:
	bash scripts/platforms/ios/keyboard/build_archive.sh

# One command: build every distributable artifact THIS host can produce, into dist/.
# No single host does all five — Apple targets need macOS, .deb needs Linux, .msi needs Windows.
package:
	@case "$$(uname)" in \
	  Darwin) $(MAKE) macos-package android-package ios-package ;; \
	  Linux)  $(MAKE) linux-package ;; \
	  *)      echo "On Windows run: make windows-package" >&2; exit 1 ;; \
	esac
	@echo "==> artifacts:"; ls -1 dist/*/ 2>/dev/null || true

ibus-install:
	bash scripts/platforms/linux/ibus/install_engine.sh

ibus-uninstall:
	bash scripts/platforms/linux/ibus/uninstall_engine.sh

ibus-smoke:
	bash scripts/platforms/linux/ibus/smoke_test.sh

paper-current:
	cd $(PAPER_CURRENT_DIR) && TEXMFVAR=/tmp/texmf-var lualatex -interaction=nonstopmode -halt-on-error $(PAPER_CURRENT_TEX)
	cd $(PAPER_CURRENT_DIR) && TEXMFVAR=/tmp/texmf-var lualatex -interaction=nonstopmode -halt-on-error $(PAPER_CURRENT_TEX)

paper-current-clean:
	rm -f $(PAPER_CURRENT_DIR)/*.aux $(PAPER_CURRENT_DIR)/*.log $(PAPER_CURRENT_DIR)/*.out

# Configure git to use the tracked hooks in .githooks/.
# Run once per clone. The pre-commit hook blocks accidental .so commits.
setup-hooks:
	git config core.hooksPath .githooks
	@echo "Git hooks activated. Pre-commit will now block .so files from being staged."
