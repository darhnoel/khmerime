.PHONY: help web web-release web-phone desktop stats suggest suggest-wfst suggest-shadow shadow-eval visualize-lexicon visualize-lexicon-streamlit fmt test test-golden test-ui ibus-install ibus-uninstall ibus-smoke paper-current paper-current-clean

DX ?= dx
APP_DIR := apps/dioxus-app
CLI := cargo run -p khmerime_lookup_cli --bin lookup_cli --
QUERY ?= tver
MODE ?= shadow
QUERIES ?=
OUTPUT ?=
PAPER_CURRENT_DIR := papers/current-implementation
PAPER_CURRENT_TEX := khmerime_current_implementation_paper.tex

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
	"  make visualize-lexicon           Generate lightweight lexicon relationship views under dist/" \
	"  make visualize-lexicon-streamlit Launch the optional Streamlit explorer for the generated views" \
	"  make fmt                         Run cargo fmt" \
	"  make test                        Run cargo test" \
	"  make test-golden                 Run the WFST golden snapshot test" \
	"  make test-ui                     Run the browser/UI Python test file" \
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
