# Copy this file to macos-signing.local.mk and fill in local signing values.
# macos-signing.local.mk is git-ignored because certificate hashes and Team IDs
# are machine/account-specific.

MACOS_CODE_SIGN_IDENTITY := <developer-id-application-sha1>
MACOS_TEAM_ID := <team-id>
MACOS_NOTARY_PROFILE := khmerime-notary
