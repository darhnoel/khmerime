.PHONY: help web web-phone desktop stats suggest suggest-wfst suggest-shadow shadow-eval fmt test test-golden test-ui paper-current paper-current-clean

DX ?= dx
CLI := cargo run --bin lookup_cli --
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
	"  make web-phone                   Run the web app on a phone-accessible host" \
	"  make desktop                     Run the desktop app" \
	"  make stats                       Print lexicon entry count" \
	"  make suggest QUERY=tver          Print legacy-mode suggestions" \
	"  make suggest-wfst QUERY=tver     Print WFST-mode suggestions" \
	"  make suggest-shadow QUERY=tver   Print shadow-mode suggestions" \
	"  make shadow-eval QUERIES=path/to/queries.txt [MODE=shadow|wfst|hybrid] [OUTPUT=report.txt]" \
	"  make fmt                         Run cargo fmt" \
	"  make test                        Run cargo test" \
	"  make test-golden                 Run the WFST golden snapshot test" \
	"  make test-ui                     Run the browser/UI Python test file" \
	"  make paper-current               Build the current implementation paper PDF" \
	"  make paper-current-clean         Remove LaTeX build byproducts from the paper folder" \
	"" \
	"Read docs/development.md for the workflow and command details."

web:
	$(DX) serve

web-phone:
	bash scripts/serve_web_phone.sh

desktop:
	$(DX) serve --platform desktop

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

fmt:
	cargo fmt --all

test:
	cargo test

test-golden:
	cargo test --test decoder_golden

test-ui:
	python3 -m pytest tests/test_web_ui.py

paper-current:
	cd $(PAPER_CURRENT_DIR) && TEXMFVAR=/tmp/texmf-var lualatex -interaction=nonstopmode -halt-on-error $(PAPER_CURRENT_TEX)
	cd $(PAPER_CURRENT_DIR) && TEXMFVAR=/tmp/texmf-var lualatex -interaction=nonstopmode -halt-on-error $(PAPER_CURRENT_TEX)

paper-current-clean:
	rm -f $(PAPER_CURRENT_DIR)/*.aux $(PAPER_CURRENT_DIR)/*.log $(PAPER_CURRENT_DIR)/*.out
