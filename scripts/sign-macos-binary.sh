#!/bin/bash
set -euo pipefail

# Sign and notarize a macOS binary
# Usage: ./sign-macos-binary.sh <binary_path> <apple_id> <team_id> <app_password> <cert_base64> <cert_password>

# Parameters passed from workflow (not hardcoded secrets)
BINARY_PATH="$1"
APPLE_ID="$2"
TEAM_ID="$3"
APP_PASS="$4"
CERT_BASE64="$5"
CERT_PASS="$6"

echo "==> Signing and notarizing: $(basename "$BINARY_PATH")"

# Create temporary keychain
KEYCHAIN_PATH="$RUNNER_TEMP/signing.keychain-db"
KEYCHAIN_PASS=$(openssl rand -base64 32)

echo "==> Creating temporary keychain"
security create-keychain -p "$KEYCHAIN_PASS" "$KEYCHAIN_PATH"
security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
security unlock-keychain -p "$KEYCHAIN_PASS" "$KEYCHAIN_PATH"

# Import certificate
echo "==> Importing certificate"
CERT_PATH="$RUNNER_TEMP/certificate.p12"
echo "$CERT_BASE64" | base64 --decode > "$CERT_PATH"

security import "$CERT_PATH" \
    -k "$KEYCHAIN_PATH" \
    -P "$CERT_PASS" \
    -T /usr/bin/codesign \
    -T /usr/bin/security

# Set key partition list to allow codesign to access the key
security set-key-partition-list \
    -S apple-tool:,apple: \
    -s -k "$KEYCHAIN_PASS" \
    "$KEYCHAIN_PATH"

# Add keychain to search list
security list-keychains -d user -s "$KEYCHAIN_PATH" $(security list-keychains -d user | sed s/\"//g)

# Find signing identity
SIGNING_IDENTITY=$(security find-identity -v -p codesigning "$KEYCHAIN_PATH" | grep "Developer ID Application" | head -1 | awk -F'"' '{print $2}')
echo "==> Found signing identity: $SIGNING_IDENTITY"

# Sign the binary
echo "==> Signing binary"
codesign \
    --sign "$SIGNING_IDENTITY" \
    --options runtime \
    --timestamp \
    --verbose \
    "$BINARY_PATH"

# Verify signature
echo "==> Verifying signature"
codesign --verify --deep --strict --verbose=2 "$BINARY_PATH"

# Create ZIP for notarization
ZIP_PATH="${BINARY_PATH}.zip"
echo "==> Creating ZIP for notarization"
ditto -c -k --keepParent "$BINARY_PATH" "$ZIP_PATH"

# Submit for notarization
echo "==> Submitting for notarization"
xcrun notarytool submit "$ZIP_PATH" \
    --apple-id "$APPLE_ID" \
    --team-id "$TEAM_ID" \
    --password "$APP_PASS" \
    --wait

# Check notarization status
echo "==> Checking notarization status"
SUBMISSION_ID=$(xcrun notarytool history \
    --apple-id "$APPLE_ID" \
    --team-id "$TEAM_ID" \
    --password "$APP_PASS" \
    | grep -m1 "id:" | awk '{print $2}')

xcrun notarytool log "$SUBMISSION_ID" \
    --apple-id "$APPLE_ID" \
    --team-id "$TEAM_ID" \
    --password "$APP_PASS"

# Verify with spctl
echo "==> Verifying Gatekeeper acceptance"
spctl --assess --type execute --verbose "$BINARY_PATH" || true

# Cleanup
echo "==> Cleaning up"
rm -f "$CERT_PATH" "$ZIP_PATH"
security delete-keychain "$KEYCHAIN_PATH" || true

echo "✅ Successfully signed and notarized: $(basename "$BINARY_PATH")"
