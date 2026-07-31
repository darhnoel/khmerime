# ADR-0017: Desktop Enter consumes the composition instead of running an Editor Action

**Status:** Accepted

## Context

On macOS, pressing Enter to confirm a Khmer **Composition** inside a browser text box
(Facebook's message field) both committed the Khmer *and* sent the message. In Notes
the same keystroke looked fine, because a stray newline in a multiline field is
invisible. The bug is only visible where Return has a side effect.

The mobile adapters solve the equivalent problem with the **Editor Action** concept
(CONTEXT.md): Android reads the field's `imeOptions` action and iOS reads the
return-key type, so the keyboard knows whether Return means Search / Send / Done or a
literal newline, and deliberately performs that action after applying the **Commit
Rules**. That design exists because on mobile the keyboard *owns* the Return key —
nothing else will run the field's action if the keyboard doesn't.

macOS is not the same situation, in two ways:

1. **No API to ask.** An IMKit input method receives an `NSEvent` and answers a single
   question — did I consume this? There is no equivalent of `imeOptions`; an input
   method cannot interrogate the focused field to learn whether Return submits, or
   whether the field is multiline. Any attempt would be app-specific heuristics.
2. **The app already handles Return.** On the desktop the host application (or the web
   page) has its own Return handling. If the input method declines the event, macOS
   routes it onward and the app does the right thing by itself.

So the mobile rule cannot be ported, and does not need to be.

## Decision

On macOS, Enter's behavior is determined solely by whether a **Composition** is active:

- **Composition active** — Enter applies the **Commit Rules**, sends the **Commit
  Text**, and is **fully consumed**. The host application never sees a Return.
- **No composition** — Enter is **not consumed** and passes through. The application
  decides what it means (send the message, insert a newline, trigger a default button).

This is the behavior every mature CJK input method exhibits, and it is what makes a
committing Enter feel distinct from a submitting Enter: the first Enter finishes the
Khmer, the second one sends.

**Editor Action stays a mobile-only term.** The glossary entry continues to describe
the Android/iOS behavior. The desktop rule is recorded as its own term
(**Composition-Consuming Enter**) rather than stretching Editor Action to cover a
platform that has no editor action to read.

## Consequences

- Sending a Khmer message on Facebook takes two Enters: one commits, one sends. This is
  the intended, conventional IME flow, not a regression.
- The input method never fabricates a newline or a submit on the desktop, so it cannot
  desynchronize from a host field's own Return semantics.
- macOS and mobile deliberately diverge on Enter. The divergence is justified by the
  platform capability difference above, and should not be "fixed" by making them match.
- Any future desktop adapter (Windows TSF, Linux IBus) should follow this rule rather
  than the mobile Editor Action rule, for the same reasons.
