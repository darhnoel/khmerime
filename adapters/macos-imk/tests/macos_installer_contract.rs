use std::path::Path;
use std::process::Command;

#[test]
fn installer_dry_run_verifies_gatekeeper_before_copying_app() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root exists");

    let output = Command::new("make")
        .args(["-n", "platform-install-macos", "DEVELOPMENT_TEAM=TESTTEAM"])
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
        .find("spctl --assess --verbose /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should assess the built app with Gatekeeper");
    let install_copy = stdout
        .find("cp -r /tmp/khmerime-macos-build/Build/Products/Release/KhmerIMEMacOS.app")
        .expect("installer should copy the built app into Input Methods");

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
            "DEVELOPMENT_TEAM=TESTTEAM",
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
