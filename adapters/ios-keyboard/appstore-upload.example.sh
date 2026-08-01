# Copy this file to adapters/ios-keyboard/appstore-upload.local.sh and fill in the
# App Store Connect API key values. That file is git-ignored — key material never
# belongs in the repo.
#
# Create the key at https://appstoreconnect.apple.com → Users and Access → Integrations
# → App Store Connect API → `+`. Give it the **App Manager** role. Apple lets you
# download the .p8 exactly once; store it somewhere durable and back it up.
#
# Absent this file, `make ios-package` still builds and exports the .ipa and simply
# skips the upload — same "config absent → do the unsigned/local thing" rule the
# macOS and Android flows follow.

# The 10-character Key ID shown next to the key in App Store Connect.
KHMERIME_ASC_KEY_ID="<key-id>"

# The Issuer ID (a UUID) shown above the key list. Same for every key on the account.
KHMERIME_ASC_ISSUER_ID="<issuer-id>"

# Path to the downloaded private key. `xcrun altool` also finds it automatically if
# it is placed at ~/.appstoreconnect/private_keys/AuthKey_<KEY_ID>.p8.
KHMERIME_ASC_KEY_PATH="${HOME}/.appstoreconnect/private_keys/AuthKey_<key-id>.p8"
