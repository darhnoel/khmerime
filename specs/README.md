# Specs

This directory holds durable maintenance specs for the repository.

Use specs for information that should stay true across many edits:

- module boundaries
- regression surfaces
- stable contracts for maintained subsystems

Do not duplicate material that already belongs in:

- `README.md` for product summary and entrypoints
- `docs/development.md` for operational commands
- `docs/architecture.md` for deeper architectural explanation
- `PLANS.md` for temporary change planning

## Current Layout

- `rules/`
  Task-specific coding constraints or review rules.
- `structure/`
  Durable repository structure and regression guidance.
- `tools/`
  Durable contracts for repository maintenance tools.
- `templates/`
  Minimal starting points for new specs.

## Current Structural Specs

- `structure/module-boundaries.md`
- `structure/verification-surfaces.md`
- `tools/lexicon-editor.md`

## When To Add A New Spec

Add one only when at least one of these is true:

- future maintainers would otherwise repeat the same architecture decision
- a high-value behavior or boundary needs an explicit contract
- review quality depends on having a stable checklist for a subsystem

If a fact is already documented elsewhere, link to it instead of copying it.
