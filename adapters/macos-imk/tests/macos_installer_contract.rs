use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists")
}

/// Reads the boolean value of `key` from an XML plist body, relying on the
/// `<key>…</key>` element being immediately followed by `<true/>`/`<false/>`.
/// Returns `None` when the key is absent.
fn plist_bool(plist: &str, key: &str) -> Option<bool> {
    let marker = format!("<key>{key}</key>");
    let after = &plist[plist.find(&marker)? + marker.len()..];
    let after = after.trim_start();
    if after.starts_with("<true/>") {
        Some(true)
    } else if after.starts_with("<false/>") {
        Some(false)
    } else {
        None
    }
}

#[test]
fn info_plist_allows_candidate_panel_display() {
    // The candidate panel is an NSPanel. A background-only process has no
    // window-server access and cannot show panels, so LSBackgroundOnly must
    // not be true — while LSUIElement must stay true to keep the IME an
    // accessory app (no Dock icon, no menu bar) that can still float a panel.
    let plist = std::fs::read_to_string(
        repo_root().join("adapters/macos-imk/swift/KhmerIMEMacOS/Info.plist"),
    )
    .expect("Info.plist exists");

    assert_ne!(
        plist_bool(&plist, "LSBackgroundOnly"),
        Some(true),
        "LSBackgroundOnly=true suppresses the candidate NSPanel; remove it or set it false"
    );
    assert_eq!(
        plist_bool(&plist, "LSUIElement"),
        Some(true),
        "LSUIElement must stay true so the IME is an accessory app that can still show a panel"
    );
}

#[test]
fn installer_fails_before_build_when_signing_config_is_missing() {
    let output = Command::new("make")
        .args([
            "platform-reinstall-macos",
            "MACOS_TEAM_ID=",
            "MACOS_CODE_SIGN_IDENTITY=",
        ])
        .current_dir(repo_root())
        .output()
        .expect("make can be executed");

    assert!(
        !output.status.success(),
        "installer must fail before xcodebuild when signing config is missing"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires MACOS_CODE_SIGN_IDENTITY and MACOS_TEAM_ID"),
        "installer should explain the missing signing config\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.contains("xcodebuild -project"),
        "installer must not build an ad-hoc app before signing config is provided\nstdout:\n{stdout}"
    );
}

#[test]
fn installer_fails_before_build_when_signing_config_has_placeholders() {
    let output = Command::new("make")
        .args([
            "platform-reinstall-macos",
            "MACOS_SIGNING_CONFIG=adapters/macos-imk/macos-signing.example.mk",
        ])
        .current_dir(repo_root())
        .output()
        .expect("make can be executed");

    assert!(
        !output.status.success(),
        "installer must fail before xcodebuild when signing config still has placeholders"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("replace placeholder values"),
        "installer should explain placeholder config values\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.contains("xcodebuild -project"),
        "installer must not build an ad-hoc app with placeholder signing config\nstdout:\n{stdout}"
    );
}

#[test]
fn installer_dry_run_reads_explicit_signing_config_file() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let config_path = std::env::temp_dir().join(format!("khmerime-macos-signing-{nonce}.mk"));
    std::fs::write(
        &config_path,
        "MACOS_CODE_SIGN_IDENTITY := CONFIGIDENTITY\nMACOS_TEAM_ID := CONFIGTEAM\nMACOS_NOTARY_PROFILE := config-notary\n",
    )
    .expect("test can write temporary signing config");

    let output = Command::new("make")
        .args([
            "-n",
            "platform-reinstall-macos",
            &format!("MACOS_SIGNING_CONFIG={}", config_path.display()),
        ])
        .current_dir(repo_root())
        .output()
        .expect("make dry-run can be executed");
    let _ = std::fs::remove_file(&config_path);

    assert!(
        output.status.success(),
        "make dry-run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--sign CONFIGIDENTITY"),
        "installer should sign final app with identity from explicit config\n{stdout}"
    );
    assert!(
        stdout.contains("--keychain-profile config-notary"),
        "installer should read notary profile from explicit config\n{stdout}"
    );
}

#[test]
fn installer_dry_run_verifies_gatekeeper_before_copying_app() {
    let output = Command::new("make")
        .args([
            "-n",
            "platform-install-macos",
            "MACOS_TEAM_ID=TESTTEAM",
            "MACOS_CODE_SIGN_IDENTITY=ABCDEF123456",
            "MACOS_NOTARY_PROFILE=test-notary",
        ])
        .current_dir(repo_root())
        .output()
        .expect("make dry-run can be executed");

    assert!(
        output.status.success(),
        "make dry-run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let gatekeeper_check = stdout
        .find("spctl --assess --type execute --verbose=2 /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should assess the built app with Gatekeeper");
    let post_staple_codesign_verify = stdout
        .rfind("codesign --verify --deep --strict --verbose=2 /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should verify the built app after stapling");
    let notarize_submit = stdout
        .find("xcrun notarytool submit /tmp/khmerime-macos-notarize.zip --keychain-profile test-notary --wait")
        .expect("installer should submit the app for notarization before Gatekeeper assessment");
    let staple_validate = stdout
        .find("xcrun stapler validate /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should validate the stapled notarization ticket");
    let codesign_verify = stdout
        .find("codesign --verify --deep --strict --verbose=2 /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should verify the explicit Developer ID signature before notarization");
    let notarize_archive = stdout
        .find("ditto -c -k --keepParent /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app /tmp/khmerime-macos-notarize.zip")
        .expect("installer should archive the app for notarization");
    let install_copy = stdout
        .find("ditto /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should copy the built app into Input Methods with ditto");
    let installed_codesign_verify = stdout
        .find("codesign --verify --deep --strict --verbose=2 /Users/")
        .expect("installer should verify the installed Input Methods copy");

    assert!(
        codesign_verify < notarize_archive && notarize_archive < notarize_submit,
        "the app must be explicitly signed and verified before notarization\n{stdout}"
    );
    assert!(
        notarize_submit < staple_validate
            && staple_validate < post_staple_codesign_verify
            && post_staple_codesign_verify < gatekeeper_check,
        "Gatekeeper assessment must happen after notarization, stapling, and code-sign verification\n{stdout}"
    );
    assert!(
        gatekeeper_check < install_copy,
        "Gatekeeper assessment must happen before installing the app\n{stdout}"
    );
    assert!(
        install_copy < installed_codesign_verify,
        "the installed app copy must be code-sign verified after ditto\n{stdout}"
    );
}

#[test]
fn installer_dry_run_uses_manual_signing_identity_when_provided() {
    let output = Command::new("make")
        .args([
            "-n",
            "platform-install-macos",
            "MACOS_TEAM_ID=TESTTEAM",
            "MACOS_CODE_SIGN_IDENTITY=ABCDEF123456",
        ])
        .current_dir(repo_root())
        .output()
        .expect("make dry-run can be executed");

    assert!(
        output.status.success(),
        "make dry-run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CODE_SIGN_STYLE=Manual CODE_SIGN_IDENTITY=- CODE_SIGN_INJECT_BASE_ENTITLEMENTS=NO"),
        "xcodebuild should use local signing before the explicit Developer ID signing step\n{stdout}"
    );
    assert!(
        stdout.contains("codesign --force --deep --options runtime --timestamp"),
        "installer should explicitly sign the final app after xcodebuild\n{stdout}"
    );
    assert!(
        stdout.contains("--sign ABCDEF123456"),
        "installer should use the provided signing identity for explicit final signing\n{stdout}"
    );
    assert!(
        !stdout.contains("CODE_SIGN_STYLE=Automatic"),
        "installer must not require Xcode automatic signing when a local identity is provided\n{stdout}"
    );
}
