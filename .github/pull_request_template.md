## Summary

- What changed:
- Why:

## Scope

- In scope:
- Out of scope:

## AI Disclosure (Required)

- AI-Assistance: none | used (describe tools and affected parts)
- Human-Verification: describe what you personally reviewed and validated

## Golden Snapshot Metadata (Required)

- Golden-Changed: no | yes
- Golden-Discussion: N/A | <issue-or-pr-url>
- Golden-Rationale: N/A | <intended behavior change and why>
- Golden-Approval: N/A | requested

## Verification

- [ ] `cargo fmt --all`
- [ ] `cargo test`
- [ ] `cargo test --test decoder_golden` (if decoder/golden surface changed)
- [ ] `cargo test -p khmerime_session` (if session/native path changed)
- [ ] `cargo test -p khmerime_linux_ibus --test ibus_bridge_protocol` (if native path changed)
- [ ] `python3 -m pytest tests/test_web_ui.py` (if browser UI changed)

## Notes for Reviewers

- Risk areas:
- Follow-up items:
