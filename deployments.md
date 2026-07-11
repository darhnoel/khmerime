# Deployments — from repo to stores to revenue

The follow-along checklist for shipping KhmerIME to the Play Store and App Store,
then turning on monetization. Ordered by dependency: **nothing earns until the apps
are listed**, and AdMob itself requires a live store listing to verify against.

Detail lives in [docs/signing_pipeline.md](docs/signing_pipeline.md) (per-platform
signing) and [PACKAGING.md](PACKAGING.md) (build/config conventions). This file is
the order of operations.

Legend: **[you]** = account/console/cert steps only you can do · **[code]** = repo
work Claude can do · ✅ done · ⬜ pending

---

## Phase A — Android → Google Play  *(start here: fastest to listed)*

1. ⬜ **[you] Play Console account** — https://play.google.com/console, one-time $25.
   Choose **Personal** account type (no D-U-N-S number needed — that's organizations
   only); verify identity with government ID. Note: personal accounts display your
   **legal name** publicly on the listing. Create the app entry (app name, default
   language, free app).
2. ✅ **[code] Upload keystore + gradle signing** — done: upload keystore at
   `~/.khmerime-signing/khmerime-upload.jks` (alias `khmerime`, valid to 2053),
   `keystore.properties` filled, `signingConfig` wired in
   `adapters/android-ime/app/build.gradle.kts` (absent file → unsigned build).
3. ⬜ **[you] Keep the keystore safe** — back up `~/.khmerime-signing/khmerime-upload.jks`
   **and** the passwords in `adapters/android-ime/keystore.properties` (password
   manager). Losing the upload key is recoverable via Play support but painful.
4. ✅ **[code] Build the signed bundle** — release builds pass the Product Version:
   `KHMERIME_PACKAGE_VERSION=1.0.0-rc.N KHMERIME_ANDROID_VERSION_CODE=<build#> make android-package`
   → `dist/android/KhmerIME-1.0.0-rc.2-android.aab` (verified: signed with the
   upload key, and the bundle's internal `versionName` matches — Play reads the
   internals, not the filename). Bump `KHMERIME_ANDROID_VERSION_CODE` on every
   Play upload; tag `v1.0.0-rc.N` on main to match what you upload.
5. ⬜ **[you] Play Console setup forms** — Data Safety (no data collected — the
   keyboard is offline; say so), content rating questionnaire, privacy policy URL
   (host a page on the Download Landing Page domain), app category (Tools),
   store listing text + screenshots (grab from emulator).
6. ⬜ **[you] Internal testing track** — upload the `.aab`, add your own account as
   tester, install from Play, sanity-check the keyboard end-to-end.
7. ⬜ **[you] Closed testing (required for new personal accounts)** — recruit ~12+
   testers (friends/family/Khmer community, Gmail accounts) who opt in and keep the
   app for **14 consecutive days**; the Play Console shows the authoritative current
   tester count. Start this track as early as possible — it gates production.
8. ⬜ **[you] Apply for production access, then promote** — answer Google's questions
   about your testing; first review typically takes a few days. **→ Android is listed.**

## Phase B — iOS → App Store  *(start prerequisites in parallel with Phase A)*

1. ⬜ **[you] Apple prerequisites** (Apple Developer account exists, team `9289LTXAT7`):
   - **Apple Distribution** certificate (Xcode → Settings → Accounts → Manage
     Certificates → `+`). You have only `Apple Development` today.
   - **App IDs + App Store provisioning profiles for BOTH** the container app and
     the keyboard appex (two bundle IDs — the appex profile is the classic failure point).
   - **App Store Connect**: create the app record; generate an **API key**
     (`.p8` + Key ID + Issuer ID) for CLI uploads.
2. ⬜ **[code] Export + upload wiring** — copy `ExportOptions.example.plist` →
   real (git-ignored); extend the iOS package flow with `xcodebuild -exportArchive`
   → signed `.ipa`, and a gated upload step (`xcrun altool --upload-app` with the
   API key). *Scoped in signing_pipeline.md Phase 2.*
3. ⬜ **[you] App Store Connect forms** — privacy nutrition label (no data
   collected), review notes explaining the keyboard works offline and does NOT
   request Full Access (this materially helps keyboard review), screenshots,
   description.
4. ⬜ **[you] TestFlight** — upload build, install on your iPhone, verify live
   (the Phrase Wheel end-to-end run we still owe).
5. ⬜ **[you] Submit for review** — keyboards get extra scrutiny; the no-Full-Access,
   fully-offline story is the strongest possible position. **→ iOS is listed.**

## Phase C — Monetization  *(only possible after A/B; ship as v1.1)*

Decisions already made (to be ADR'd when built): ads live in the **Companion App
only** (never the keyboard — extension memory limits + AdMob/Play policy); primary
format = **rewarded ads unlocking Lexicon Packs**; Dashboard banner is secondary;
IAP later as the ad-free path.

1. ⬜ **[you] AdMob account** — https://admob.google.com, register both apps, **link
   them to their live store listings** (this is why C waits for A/B).
2. ⬜ **[you] `app-ads.txt`** — publish AdMob's line on the Download Landing Page
   domain; add the domain as the developer website on both listings.
3. ⬜ **[code] Consent + SDK plumbing** — Google UMP consent flow (GDPR) on both;
   **ATT prompt on iOS** (required before personalized ads); AdMob SDK in the two
   Companion Apps only. Privacy labels/Data Safety updated to declare ads.
4. ⬜ **[code] Rewarded ads → Lexicon Packs** — "watch an ad → unlock a curated pack"
   flow in the Dashboard, on top of the existing pack system (packs are already
   versioned + registry-ready). This is the feature that actually earns.
5. ⬜ **[code] Dashboard banner** — one banner on Tips/CharacterTable surfaces.
6. ⬜ **[you] Payment profiles** — AdMob payout details; watch fill rates for the KH
   region before projecting revenue.

## Standing rules

- Ship **v1 with no ads** — clean first review, monetize in v1.1.
- Every signing config: committed `*.example`, git-ignored real file, absent →
  unsigned build (repo convention, see PACKAGING.md).
- Release flow: work on `dev` → PR to `main` → tag `vX.Y.Z-rc.N` → the
  `release-candidate.yml` workflow builds all platforms (signed artifacts still
  come from the local Mac until CI signing is set up).
