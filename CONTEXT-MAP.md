# Context map

This repository has multiple bounded contexts. Each owns its own decisions
(ADRs) and, where useful, its own `CONTEXT.md` glossary. System-wide decisions
that cut across contexts live in the root `docs/adr/`.

| Context | Lives in | Decisions (ADRs) | Glossary |
| --- | --- | --- | --- |
| **System / engine + native adapters** | `crates/`, `adapters/` | `docs/adr/` (0001–…) | `CONTEXT.md` |
| **Dioxus webapp** (the Online Beta editor) | `apps/dioxus-app/` | `apps/dioxus-app/docs/adr/` (0001–…) | — |

Notes:

- The **root `docs/adr/`** sequence (0001–0022 today) is for the engine and the
  native IME adapters (IBus / TSF / IMK / iOS / Android).
- The **webapp** owns its own ADR sequence starting at `0001` under
  `apps/dioxus-app/docs/adr/` — its UI/UX/product decisions are scoped to that
  app and do not share numbers with the system sequence.
- A decision that genuinely spans both (e.g. an engine behavior change that the
  webapp and the adapters both depend on) belongs in the root sequence.
