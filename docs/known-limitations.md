# Known limitations

Behaviors that look like input-method bugs but are not fixable from the IME.

## Safari web chat apps send on Enter mid-composition (Messenger, and others)

**Symptom.** Typing Khmer in Messenger (or a similar web chat) inside Safari, a single
Enter both commits the Khmer *and* sends the message, instead of only committing.

**Not an IME bug.** The macOS input method consumes that Return correctly. Captured from
a live session (`handle(_:client:)` return value plus the requesting app):

```
key=0xff0d consumed=true  app=com.apple.Safari      ← consumed, page sent anyway
key=0xff0d consumed=true  app=com.microsoft.VSCode  ← consumed, committed only
key=0xff0d consumed=false app=com.microsoft.VSCode  ← passthrough, native app acts
```

Native clients (VS Code, Notes) honor the consumption and show the intended two-Enter
flow from [ADR-0017](adr/0017-desktop-enter-consumes-the-composition.md): the first
Enter commits, the second sends. Safari delivers a `keydown` to the page regardless.

**Root cause (browser + web app).** WebKit fires `keydown` and `compositionend` in the
opposite order from Chromium and reports the commit keydown as `which === Enter` with
`isComposing: true`; Chromium reports `keyCode 229`, which most handlers skip. A web app
is expected to guard its send handler with
`event.isComposing || event.keyCode === 229`. Messenger does not (or trips the WebKit
ordering bug), so its handler fires while a composition is still active.

This is a well-known CJK-input failure mode, not specific to Khmer — the same reports
exist for paperless-ngx, OpenAI Codex, Discourse, and others, and it is catalogued in
the [CJK Failure Corpus](https://greymoth-jp.github.io/cjk-failure-corpus/).

**Confirmed by direct comparison (2026-07-31).** The same Messenger conversation, same
Khmer input, same single Enter: **Chrome commits only (correct); Safari commits and
sends.** Nothing about the input method differs between the two — this is the clearest
possible demonstration that the behavior is browser-side.

**Workarounds.** Use Chrome for web chat — verified working (Chromium's `keyCode 229`
means the page's send handler never fires mid-composition). Alternatively, finish the
Khmer with Space so no composition is active when Enter is pressed.

**Do not "fix" this in the adapter.** Suppressing the send would require emitting or
withholding events the input method does not control, and would break the correct
behavior that native apps already show.
