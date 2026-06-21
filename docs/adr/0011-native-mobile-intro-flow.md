# ADR-0011: Native platform UI for mobile app intro flow

**Status:** Accepted

## Context

The KhmerIME keyboard extension ships with a minimal host app on both iOS (blank UIViewController) and Android (no Activity at all) — the container exists only to host the extension. This means tapping the app icon shows nothing (iOS) or nothing at all. We need an intro flow (brand landing → setup guide → settings dashboard with tips/support tabs) to help users enable the keyboard in system settings and configure the IME.

The core question: should each platform build its own native Swift/Kotlin UI, or use a shared cross-platform approach?

## Decision

Build the intro flow as **native platform UI** on each platform — Swift with UIKit/UIWindow on iOS, Kotlin with standard Android Views/Activity on Android.

The Silk Veil brand tokens (`$ink: #14101b`, `$amber: #e98a4e`, `$ivory: #f4ece2`) are shared as constants; the logo card and app icon are exported as static PNG assets from the existing CSS glass card (`site/download/styles.css`). Every view is a thin native layer: a `UIViewController` or `Activity` stacking a logo image, labels, and a button.

We considered and rejected:

- **Shared WebView** — loading embedded HTML/CSS/JS in both platforms. Rejected because every page except the brand landing needs access to native Settings APIs (`UIApplication.openSettingsURLString`, `Settings.ACTION_INPUT_METHOD_SETTINGS`), which requires a JS bridge with platform-specific message handlers — eroding the cross-platform benefit while introducing WebView latency and a second styling surface to maintain.

- **Cross-platform Rust UI (Dioxus with a native renderer)** — attractive in theory since the project already uses Rust for the IME engine. Rejected because the mobile platform adapters do not have a Dioxus native renderer integrated (the existing Dioxus app targets web/desktop), and shoehorning one in for four static screens is disproportionate to the work. The engine already links per-platform (UniFFI for iOS, JNI for Android); native views are the simplest terminal for those existing bridges.

Each screen is trivially simple: a centered logo, a wordmark, a button. The cost of maintaining two implementations is small, and the benefit — no WebView overhead, full access to platform Settings intents, native-feel animations — is immediate.

## Consequences

- The two implementations will drift over time if new screens are added; periodic alignment reviews are needed.
- Brand changes (new logo, new color) require updating both native codebases plus the exported PNG assets — no single CSS change propagates to mobile.
- A future migration to a shared UI layer (if one emerges) would replace these screens entirely; the cost of rewriting four native screens is low.
- The Settings tab's "Clear Learned History" and "Next Word Prediction" toggle wire directly through the existing Rust bridge (UniFFI / JNI) with no intermediate abstraction — the simplest path, and hard to get wrong.
