use std::path::Path;
use std::process::Command;

#[test]
fn installer_dry_run_verifies_gatekeeper_before_copying_app() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists");

    let output = Command::new("make")
        .args([
            "-n",
            "platform-install-macos",
            "MACOS_TEAM_ID=TESTTEAM",
            "MACOS_CODE_SIGN_IDENTITY=ABCDEF123456",
            "MACOS_NOTARY_PROFILE=test-notary",
        ])
        .current_dir(&repo_root)
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
    let notarize_submit = stdout
        .find("xcrun notarytool submit /tmp/khmerime-macos-notarize.zip --keychain-profile test-notary --wait")
        .expect("installer should submit the app for notarization before Gatekeeper assessment");
    let staple_validate = stdout
        .find("xcrun stapler validate /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should validate the stapled notarization ticket");
    let install_copy = stdout
        .find("ditto /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should copy the built app into Input Methods with ditto");

    assert!(
        notarize_submit < staple_validate && staple_validate < gatekeeper_check,
        "Gatekeeper assessment must happen after notarization and stapling\n{stdout}"
    );
    assert!(
        gatekeeper_check < install_copy,
        "Gatekeeper assessment must happen before installing the app\n{stdout}"
    );
}

#[test]
fn installer_dry_run_uses_manual_signing_identity_when_provided() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists");

    let output = Command::new("make")
        .args([
            "-n",
            "platform-install-macos",
            "MACOS_TEAM_ID=TESTTEAM",
            "MACOS_CODE_SIGN_IDENTITY=ABCDEF123456",
        ])
        .current_dir(&repo_root)
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
        stdout.contains("CODE_SIGN_STYLE=Manual CODE_SIGN_IDENTITY=ABCDEF123456 DEVELOPMENT_TEAM=TESTTEAM"),
        "installer should use the provided local signing identity with manual signing\n{stdout}"
    );
    assert!(
        !stdout.contains("CODE_SIGN_STYLE=Automatic"),
        "installer must not require Xcode automatic signing when a local identity is provided\n{stdout}"
    );
}
