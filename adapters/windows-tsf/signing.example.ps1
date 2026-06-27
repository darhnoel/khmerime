# Copy this file to adapters/windows-tsf/signing.local.ps1 (git-ignored) and fill in.
# Dot-sourced by scripts/platforms/windows/tsf/build_msi.ps1 to signtool-sign the MSI.
#
# Use EITHER a cert already in the CurrentUser\My store (thumbprint), OR a .pfx file.
# Get a thumbprint: Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert

$CertThumbprint = "<sha1-thumbprint>"   # preferred: cert in CurrentUser\My
$PfxPath        = ""                     # alternative: path to a .pfx
$PfxPassword    = ""
$TimestampUrl   = "http://timestamp.digicert.com"
