# Mobile App Intro Flow — Implementation Plan

## Overview

The KhmerIME host app (iOS `KhmerIME` target, Android `app` module) currently shows nothing when launched. This plan builds a 4-phase intro flow:

```
Tap icon → [Brand Landing] → [Setup Guide] → [Dashboard: Settings | Tips | Support]
                      ↑ one-time only        ↑ auto-dismisses when keyboard enabled
```

---

## Pre-work: Shared assets

Export from the existing CSS glass card logo (`site/download/styles.css:131-215`). Place outputs in `assets/icons/`.

| Asset | Size | Used by |
|-------|------|---------|
| `khmerime-logo-152.png` | 152×152 px (1×) | iOS brand landing @1× |
| `khmerime-logo-304.png` | 304×304 (2×) | iOS brand landing @2× |
| `khmerime-logo-456.png` | 456×456 (3×) | iOS brand landing @3× |
| `khmerime-logo-mdpi.png` | 152×152 / mdpi | Android brand landing |
| `khmerime-logo-hdpi.png` | 228×228 / hdpi | Android brand landing |
| `khmerime-logo-xhdpi.png` | 304×304 / xhdpi | Android brand landing |
| `khmerime-logo-xxhdpi.png` | 456×456 / xxhdpi | Android brand landing |
| `khmerime-logo-xxxhdpi.png` | 608×608 / xxxhdpi | Android brand landing |
| `khmerime-icon-1024.png` | 1024×1024 | Source for all app icon sizes |
| `brand.json` | — | `{ "ink": "#14101b", "amber": "#e98a4e", "ivory": "#f4ece2", "teal": "#38c6c0" }` |

---

## Iteration 1: Brand Landing

Replace the blank screen with the Silk Veil welcome page. One-time only; persisted with `has_seen_welcome` flag.

### iOS

**Files to create:**

| File | Location | What it does |
|------|----------|-------------|
| `WelcomeViewController.swift` | `adapters/ios-keyboard/swift/KhmerIME/Views/` | Full-screen `UIViewController`. Deep-ink `#14101b` background. Centered stack: `UIImageView(logo-304)` → `UILabel("KhmerIME", weight: 800, color: ivory)` → `UILabel("roman → ខ្មែរ", weight: 600, color: amber)` → `UILabel("Type Khmer using the Latin alphabet", color: ivory-dim)` → `UIButton("Get Started", .amber)` → `UILabel("Already enabled? Open keyboard →")`. Button sets `UserDefaults.has_seen_welcome = true` and pushes `SetupGuideViewController`. |
| `Assets.xcassets/` | `KhmerIME/` | New asset catalog. `AppIcon.appiconset` (all sizes from `khmerime-icon-1024.png`). `LogoCard.imageset` (1×/2×/3× from `khmerime-logo-*.png`). |

**Files to modify:**

| File | Change |
|------|--------|
| `SceneDelegate.swift` | Replace `window?.rootViewController = UIViewController()` with: check `UserDefaults.standard.bool(forKey: "has_seen_welcome")`. If `false` → `WelcomeViewController()`. If `true` → check keyboard enabled (poll `UITextInputMode.activeInputModes` for `"com.khmerime.KhmerIME.Keyboard"`) → `SetupGuideViewController` or `DashboardTabController`. |
| `Info.plist` | `UILaunchScreen`: change empty dict to `{ "UIColorName": "" }` and set `LaunchScreen.backgroundColor` via plist. Or add `LaunchScreen.storyboard` with deep-ink background. Simplest: set `UIApplicationLaunchScreenBackgroundColor` to `#14101b`. |
| `project.yml` | Add `KhmerIME/Assets.xcassets` to the `KhmerIME` target sources. |

### Android

**Files to create:**

| File | Location | What it does |
|------|----------|-------------|
| `WelcomeActivity.kt` | `.../java/com/khmerime/WelcomeActivity.kt` | `AppCompatActivity`. `setContentView(R.layout.activity_welcome)`. Button click → `SharedPreferences.edit { putBoolean("has_seen_welcome", true) }` → `startActivity(SetupGuideActivity)`. |
| `activity_welcome.xml` | `res/layout/` | `ConstraintLayout` with `background="#14101b"`. `ImageView` (logo, `app:layout_constraintTop_toTopOf="parent"` at 33% top margin), `TextView` wordmark pair, `TextView` tagline, `MaterialButton` (ember-amber background via custom style `Widget.KhmerIME.Button.Primary`), secondary `TextView` "Already enabled? Open keyboard →". |
| `res/drawable/ic_logo_card.xml` | `res/drawable/` | NOT needed — use PNG from shared assets in `res/drawable-nodpi/khmerime_logo.webp` (converted from PNG). |
| `res/values/colors-brand.xml` | `res/values/` | Brand colors: `ink #14101b`, `amber #e98a4e`, `ivory #f4ece2`, `teal #38c6c0`. |

**Files to modify:**

| File | Change |
|------|--------|
| `AndroidManifest.xml` | Add `<activity android:name=".WelcomeActivity" android:exported="true" android:theme="@style/Theme.Khmerime">` with `<intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter>`. This makes WelcomeActivity the launcher — currently there is no launcher activity. |
| `res/mipmap-*/ic_launcher.webp` | Replace placeholder files with KhmerIME icon resized to each density. |
| `res/values/themes.xml` | Ensure `Theme.Khmerime` has deep-ink background by default. |

---

## Iteration 2: Setup Guide

Platform-specific keyboard enablement steps. Auto-detects when keyboard is enabled and auto-dismisses to dashboard.

### iOS

**Files to create:**

| File | What it does |
|------|-------------|
| `SetupGuideViewController.swift` | `UIViewController`. `UIPageViewController` with 3 child view controllers (one per step). Each child has: numbered amber badge `UILabel`, instruction `UILabel`, optional `UIImageView` (screenshot of Settings pane). "Open Settings" `UIButton` at bottom calls `UIApplication.shared.open(URL(string: UIApplication.openSettingsURLString)!)`. |

**Three steps (iOS):**

| Step | Instruction |
|------|-------------|
| 1 | "Go to **Settings → General → Keyboard → Keyboards**" |
| 2 | "Tap **Add New Keyboard** → select **KhmerIME**" |
| 3 | "Tap the keyboard name → enable **Allow Full Access**" |

A `Timer` fires every 1 second checking `UITextInputMode.activeInputModes`. When `"com.khmerime.KhmerIME.Keyboard"` is found → `UserDefaults.set(true, forKey: "keyboard_enabled")` → dismiss to `DashboardTabController`.

### Android

**Files to create:**

| File | What it does |
|------|-------------|
| `SetupGuideActivity.kt` | `AppCompatActivity`. `ViewPager2` with 2 step fragments. "Open Settings" button fires `Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)`. |

**Two steps (Android):**

| Step | Instruction |
|------|-------------|
| 1 | "Go to **Settings → System → Languages & input → Virtual keyboard → Manage keyboards**" |
| 2 | "Toggle **KhmerIME** on" |

`InputMethodManager` polling: `(getSystemService(INPUT_METHOD_SERVICE) as InputMethodManager).enabledInputMethodList` contains component name `"com.khmerime/.service.KhmerInputMethodService"` → `SharedPreferences.edit { putBoolean("keyboard_enabled", true) }` → launch `DashboardActivity` and finish.

---

## Iteration 3: Settings Dashboard

Tabbed home screen that replaces the setup guide once the keyboard is enabled.

### iOS

**Files to create:**

| File | What it does |
|------|-------------|
| `DashboardTabController.swift` | `UITabBarController`. 3 tabs: Settings, Tips, Support. Tab bar appearance: `UITabBarAppearance` with `backgroundEffect: .systemUltraThinMaterialDark`, `tintColor: #e98a4e`. |
| `SettingsViewController.swift` | `UITableViewController` (grouped, `.insetGrouped`). Sections: |
| | **Section 1 — Prediction:** "Next Word Prediction" with `UISwitch`. Stores to `UserDefaults.standard.bool(forKey: "next_word_prediction")`. Wiring: `KeyboardSession.swift` reads this flag on init and passes to `ImeSessionOptions`. |
| | **Section 2 — Custom Words:** One cell "Custom Words" with `accessoryType = .disclosureIndicator`, `textLabel.textColor = .gray`. Tapping shows `UIAlertController` title "Coming Soon" message "Manage custom roman→Khmer pairs from the candidate list while typing." |
| | **Section 3 — Data:** "Clear Learned History" cell, `textLabel.textColor = .systemRed`. Tapping shows destructive `UIAlertController`. On confirm: calls `imeSession.reset_history()`. |
| | **Section 4 — About:** "Version 1.0 (build XX)" cell, gray, no interaction. Reads from `Bundle.main.infoDictionary`. |

**Files to modify:**

| File | Change |
|------|--------|
| `KeyboardSession.swift` | In `init()`, read `UserDefaults.standard.bool(forKey: "next_word_prediction")` and pass to `ImeSessionOptions { next_word_prediction = ... }`. When the toggle changes in Settings, rebuild session or update the flag via a new Rust API. |
| `Rust bridge` (`khmerime_ios_keyboard/src/lib.rs`) | Read `next_word_prediction` from `ImeSessionOptions` and store in session. In `suggest()`, gate the `next_word_suggestions()` call on this flag. |

### Android

**Files to create:**

| File | What it does |
|------|-------------|
| `DashboardActivity.kt` | `AppCompatActivity`. `BottomNavigationView` + `FragmentContainerView`. 3 tabs. |
| `SettingsFragment.kt` | `Fragment`. Custom layout (not `PreferenceFragmentCompat` — keeps it simple). Rows built as `LinearLayout` children. Each row: `TextView` (label) + end widget (switch / chevron / button). |
| | **Prediction row:** `SwitchCompat`. Persisted to `SharedPreferences`. Wiring: `KhmerImeSession` reads this in JNI init. |
| | **Custom Words row:** Disabled, amber chevron. Tap → `AlertDialog` "Coming soon". |
| | **Clear Learned History row:** Red text. Tap → `AlertDialog` confirm. Calls JNI `resetHistory()`. |
| | **Version row:** `context.packageManager.getPackageInfo(...)`. |
| `res/layout/fragment_settings.xml` | Scrollable `LinearLayout` with card-style rows. |

**Files to modify:**

| File | Change |
|------|--------|
| `KhmerImeSession.kt` | Read `SharedPreferences` in `create()` / `init()`. Pass `next_word_prediction` into Rust JNI call. |
| `lib.rs` (android-ime) | Same gate as iOS in `suggest()`. |

---

## Iteration 4: Tips Tab

| | Tip |
|---|-----|
| 1 | "**Type roman, get Khmer.** Just type `somreach` and KhmerIME suggests `សម្រាក`. Tap a candidate or press Enter to commit." |
| 2 | "**Split long words.** For long compositions, press `s'` between parts — KhmerIME shows segmented previews you can navigate with Left/Right." |
| 3 | "**Names and loanwords.** Press ✦ to enter CharPick mode. Type a roman letter to see all matching Khmer characters. Tap one to insert it." |
| 4 | "**English when you need it.** Press EN to toggle English mode — all keys pass through to the host app. Press EN again to return to Khmer." |

### iOS

`TipsViewController.swift`: `UIPageViewController` with 4 child VCs. Each has `UIImageView` (SF Symbol: `"keyboard"`, `"textformat.alt"`, `"character"`, `"globe"`) + headline + body. Page dots at bottom.

### Android

`TipsFragment.kt`: `ViewPager2` with 4 pages. Each page: `ImageView` (Material icon: `ic_keyboard`, `ic_space_bar`, `ic_person`, `ic_language`) + `TextView` pair.

---

## Iteration 5: Support Tab

### iOS

`SupportViewController.swift`: Glass card (`UIView` with translucent white background, blur, white border, corner radius). "Support KhmerIME" heading. Body text: "KhmerIME is an open-source project. Your support helps keep development, testing, packaging, and future improvements going." `UIImageView` with ABA QR code (`site/download/assets/my-aba-cropped.png`). Caption "ABA Mobile".

### Android

`SupportFragment.kt`: Same layout as iOS. `ImageView` with ABA QR from drawable. Body text in `TextView`.

---

## Iteration 6: App Icon + Launch Screen Polish

### iOS

| File | Change |
|------|--------|
| `Assets.xcassets/AppIcon.appiconset/Contents.json` | All sizes: 20pt (1×/2×/3×), 29pt, 40pt, 60pt, 76pt, 83.5pt, 1024pt. All pointing to properly cropped PNGs from `khmerime-icon-1024.png`. |
| `Info.plist` | Add `CFBundleIcons` and `CFBundleIcons~ipad` dictionaries referencing the asset catalog. |

### Android

| File | Change |
|------|--------|
| `res/mipmap-anydpi-v26/ic_launcher.xml` | Point `foreground` to `@drawable/ic_launcher_foreground` (KhmerIME "ខ" on glass) and `background` to `@color/ink`. |
| `res/drawable/ic_launcher_foreground.xml` | Vector drawable of "ខ" character centered, in warm ivory, with the glass-card border/halo. Simpler than the full card — just the letter on a rounded square with a subtle rim. |

---

## Rust changes (shared across both platforms)

| File | Change |
|------|--------|
| `crates/session/src/adapter_contract.rs` | Add `next_word_prediction: bool` field to `ImeSessionOptions` (default `true`). |
| `crates/session/src/ime_session.rs` | Add `next_word_prediction: bool` field to `ImeSession`. On `ImeSession::suggest()`, if `!self.next_word_prediction`, skip `transliterator.next_word_suggestions()`. Add `pub fn set_next_word_prediction(&mut self, enabled: bool)`. Add `pub fn reset_history(&mut self)` that clears `self.history` and sets `history_changed = true`. |
| `crates/session/src/lib.rs` | Export `set_next_word_prediction` and `reset_history` if needed by uniffi/JNI. |

---

## Navigation state machine (pseudocode)

```
onAppLaunch:
  if !hasSeenWelcome:
    show(BrandLanding)
  else if !keyboardEnabled:
    show(SetupGuide)
  else:
    show(Dashboard)

onBrandLandingGetStarted:
  hasSeenWelcome = true
  navigateTo(SetupGuide)

onSetupGuideKeyboardDetected:
  navigateTo(Dashboard)

onDashboardTabSelected(tab):
  if tab == 0: show(Settings)
  if tab == 1: show(Tips)
  if tab == 2: show(Support)
```

---

## File inventory: all new files by platform

### iOS (9 new files)

```
swift/KhmerIME/
  Views/
    WelcomeViewController.swift       # Iteration 1
    SetupGuideViewController.swift     # Iteration 2
    DashboardTabController.swift       # Iteration 3
    SettingsViewController.swift       # Iteration 3
    TipsViewController.swift           # Iteration 4
    SupportViewController.swift        # Iteration 5
  Assets.xcassets/                     # Iteration 1+6
    Contents.json
    AppIcon.appiconset/
    LogoCard.imageset/
```

### Android (11 new files)

```
app/src/main/
  java/com/khmerime/
    WelcomeActivity.kt                 # Iteration 1
    SetupGuideActivity.kt              # Iteration 2
    DashboardActivity.kt               # Iteration 3
    SettingsFragment.kt                # Iteration 3
    TipsFragment.kt                    # Iteration 4
    SupportFragment.kt                 # Iteration 5
  res/layout/
    activity_welcome.xml               # Iteration 1
    fragment_settings.xml              # Iteration 3
  res/values/
    colors-brand.xml                   # Pre-work
  res/drawable/
    ic_launcher_foreground.xml         # Iteration 6
```

### Rust (1 file touched)

```
crates/session/src/
  adapter_contract.rs   # add next_word_prediction field
  ime_session.rs        # add set_next_word_prediction + reset_history
```

---

## Ordering rationale

Iteration 1 provides the **highest visible impact** — the app icon finally shows something. Iteration 2 makes it *useful* (keyboard enablement). Iteration 3 adds *control* (toggles, history). Iterations 4–5 add *depth* (learning, donating). Iteration 6 is *polish* (icon, launch screen).

Each iteration is independently shippable: you could release after any iteration and the app would be better than the current blank screen.
