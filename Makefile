.PHONY: help web web-release web-phone desktop stats suggest suggest-wfst suggest-shadow shadow-eval data-split data-build data-check visualize-lexicon visualize-lexicon-streamlit fmt test test-golden test-ui platform-check platform-check-linux platform-check-android platform-check-ios platform-check-macos platform-check-windows platform-build-windows platform-install-windows platform-uninstall-windows platform-reinstall-windows platform-smoke-windows-notepad platform-smoke-windows-notepad-python windows-package linux-package ibus-install ibus-uninstall ibus-smoke paper-current paper-current-clean

DX ?= dx
APP_DIR := apps/dioxus-app
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
WINDOWS_TSF_REINSTALL_STAMP := $(or $(WINDOWS_TSF_REINSTALL_STAMP),$(shell powershell -NoProfile -Command Get-Date -Format yyyyMMddHHmmss))
WINDOWS_TSF_DEPLOY_DIR ?= target/windows-tsf-deploy/$(WINDOWS_TSF_REINSTALL_STAMP)
WINDOWS_TSF_DLL := $(WINDOWS_TSF_TARGET_DIR)/$(WINDOWS_TSF_TARGET)/debug/khmerime_windows_tsf.dll
WINDOWS_TSF_DEV_DLL := $(WINDOWS_TSF_DEV_TARGET_DIR)/$(WINDOWS_TSF_TARGET)/debug/khmerime_windows_tsf.dll
WINDOWS_TSF_DEPLOY_DLL := $(WINDOWS_TSF_DEPLOY_DIR)/khmerime_windows_tsf.dll
# Absolute path — regsvr32 runs elevated and its cwd may not be the project root.
WINDOWS_TSF_DLL_ABS := $(subst /,\,$(CURDIR)/$(WINDOWS_TSF_DLL))
WINDOWS_TSF_DEV_DLL_ABS := $(subst /,\,$(CURDIR)/$(WINDOWS_TSF_DEV_DLL))
WINDOWS_TSF_DEPLOY_DLL_ABS := $(subst /,\,$(CURDIR)/$(WINDOWS_TSF_DEPLOY_DLL))
WINDOWS_TSF_SMOKE_DELAY ?= 8

help:
	@printf "%s\n" \
	"khmerime developer commands" \
	"" \
	"  make web                         Run the Dioxus web app" \
	"  make web-release                 Build deployable web artifacts under dist/web-release" \
	"                                   Optional: WEB_BASE_PATH=khmerime-beta (or KHMERIME_BASE_PATH=/khmerime-beta)" \
	"  make web-phone                   Run the web app on a phone-accessible host" \
	"  make desktop                     Run the desktop app" \
	"  make stats                       Print lexicon entry count" \
	"  make suggest QUERY=tver          Print legacy-mode suggestions" \
	"  make suggest-wfst QUERY=tver     Print WFST-mode suggestions" \
	"  make suggest-shadow QUERY=tver   Print shadow-mode suggestions" \
	"  make shadow-eval QUERIES=path/to/queries.txt [MODE=shadow|wfst|hybrid] [OUTPUT=report.txt]" \
	"  make data-split                  Split data/roman_lookup.csv into reviewable chunk CSVs" \
	"  make data-build                  Generate data/roman_lookup.csv from chunk CSVs" \
	"  make data-check                  Validate lexicon chunks and generated runtime data" \
	"  make visualize-lexicon           Generate lightweight lexicon relationship views under dist/" \
	"  make visualize-lexicon-streamlit Launch the optional Streamlit explorer for the generated views" \
	"  make fmt                         Run cargo fmt" \
	"  make test                        Run cargo test" \
	"  make test-golden                 Run the WFST golden snapshot test" \
	"  make test-ui                     Run the browser/UI Python test file" \
	"  make platform-check              Check all native platform adapter crates" \
	"  make platform-check-<platform>   Check one adapter: linux, android, ios, macos, windows" \
	"  make platform-build-windows      Build the Windows TSF DLL target under target/windows-tsf/" \
	"  make platform-install-windows    Build and register the Windows TSF DLL with regsvr32" \
	"  make platform-uninstall-windows  Unregister the Windows TSF DLL with regsvr32 /u" \
	"  make platform-reinstall-windows  Build once, copy to a fresh DLL path, and re-register" \
	"  make platform-smoke-windows-notepad  Launch Notepad and check for TSF crash events" \
	"  make platform-smoke-windows-notepad-python  Python Notepad smoke with clipboard/log output" \
	"  make windows-package            Build the Windows TSF MSI under dist/windows/" \
	"  make linux-package               Build the Linux IBus .deb package under dist/linux/" \
	"  make ibus-install                Build and install KhmerIME IBus engine files (may use sudo)" \
	"  make ibus-uninstall              Remove KhmerIME IBus engine files" \
	"  make ibus-smoke                  Run bridge + IBus discovery smoke checks" \
	"  make paper-current               Build the current implementation paper PDF" \
	"  make paper-current-clean         Remove LaTeX build byproducts from the paper folder" \
	"" \
	"Read docs/development.md for the workflow and command details."

web:
	cd $(APP_DIR) && $(DX) serve

web-release:
	bash scripts/web/build_release.sh

web-phone:
	bash scripts/web/serve_phone.sh

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
