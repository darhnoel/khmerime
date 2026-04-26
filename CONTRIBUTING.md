# Contributing to KhmerIME

Thank you for contributing. This project is an open-source Khmer IME with a locked regression surface for decoder behavior and native input flows.

## Quick Start

1. Fork and create a focused branch.
2. Keep PRs scoped to one concern.
3. Run required checks before opening/updating PR.
4. Use the PR template fully, including AI and golden metadata.

For architecture and command details:
- [docs/development.md](docs/development.md)
- [specs/structure/module-boundaries.md](specs/structure/module-boundaries.md)
- [specs/structure/verification-surfaces.md](specs/structure/verification-surfaces.md)

## Rust Contribution Rules

- Run `cargo fmt --all` after Rust changes.
- Keep formatting/style consistent with `rustfmt.toml` (`max_width = 120`).
- Do not mix unrelated refactors with feature or bug-fix work.
- Prefer extending existing abstractions over duplicating logic.

## Required Verification

Choose checks based on touched surface (see verification spec for full matrix).

Baseline:
- `cargo fmt --all`

Core/session/decoder/editor changes:
- `cargo test`

Decoder ranking/segmentation/phrase-output changes:
- `cargo test --test decoder_golden`

Native IBus/session path changes:
- `cargo test -p khmerime_session`
- `cargo test -p khmerime_linux_ibus --test ibus_bridge_protocol`

Browser-facing UI changes:
- `python3 -m pytest tests/test_web_ui.py`

## Golden Snapshot Governance

Golden snapshots are locked by default.

For any PR that updates `tests/golden/decoder_wfst_suggest.txt` (or other golden artifacts), you must include:

1. `Golden-Discussion:` URL to prior discussion (issue or PR thread).
2. `Golden-Rationale:` concise explanation of intended behavior change.
3. `Golden-Approval: requested` in PR body before review.

Rules:
- No silent bless/auto-bless workflow.
- No snapshot updates without explicit discussion context.
- Reviewer/maintainer approval is required before merge.

## AI-Assisted Contributions Policy

AI agents/tools are allowed.

If AI was used, PR must include:
- `AI-Assistance:` what tool was used and where.
- `Human-Verification:` what you personally verified (tests + manual checks).

AI-generated changes are held to the same quality and review standards.
AI-assisted golden updates must follow the same golden discussion/approval flow.

## Pull Request Expectations

- PR description must be complete and factual.
- Include verification commands and outcomes.
- Link relevant issues/specs/discussions.
- Keep commits and diff reviewable.

## Review and Merge Notes

Maintainers may request:
- tighter scope,
- stronger tests,
- clearer golden rationale,
- split PRs for unrelated changes.

Repeated policy bypass (missing disclosure, missing golden metadata, or unverifiable changes) may block merge.
