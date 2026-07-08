# Copy this file to macos-signing.local.mk and fill in local signing values.
# macos-signing.local.mk is git-ignored because certificate hashes and Team IDs
# are machine/account-specific.

# App bundle signing (Developer ID Application) — used by install + package flows.
MACOS_CODE_SIGN_IDENTITY := <developer-id-application-sha1>
# Installer signing (Developer ID Installer) — required for a signed .pkg.
# Leave unset to keep emitting an unsigned .pkg from `make macos-package`.
MACOS_INSTALLER_SIGN_IDENTITY := <developer-id-installer-sha1>
MACOS_TEAM_ID := <team-id>
MACOS_NOTARY_PROFILE := khmerime-notary
