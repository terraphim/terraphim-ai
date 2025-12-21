# Apple Developer Program Enrollment and Code Signing Setup Guide

This guide walks through enrolling in the Apple Developer Program and configuring credentials for automated code signing and notarization in CI/CD.

## Overview

| Step | Time Required | Cost |
|------|---------------|------|
| 1. Enroll in Apple Developer Program | 1-2 days (verification) | $99/year |
| 2. Create Developer ID Certificate | 15 minutes | Included |
| 3. Create App-Specific Password | 5 minutes | Free |
| 4. Export Certificate for CI | 10 minutes | N/A |
| 5. Store Credentials in 1Password | 10 minutes | N/A |
| 6. Configure GitHub Secrets | 5 minutes | N/A |

---

## Step 1: Enroll in Apple Developer Program

### Prerequisites
- An Apple ID (create at https://appleid.apple.com if needed)
- Valid government-issued ID for identity verification
- Credit card for $99/year fee

### Enrollment Process

1. **Go to Apple Developer Program enrollment**
   ```
   https://developer.apple.com/programs/enroll/
   ```

2. **Sign in with your Apple ID**
   - Use a business/work Apple ID if available
   - Personal Apple ID works for individual enrollment

3. **Choose enrollment type**
   - **Individual**: For personal projects or sole proprietors
   - **Organization**: Requires D-U-N-S number (for companies)

   **Recommendation**: Individual enrollment is faster and sufficient for open-source projects

4. **Complete identity verification**
   - Apple will verify your identity
   - May require a phone call or document upload
   - Takes 24-48 hours typically

5. **Pay the annual fee ($99 USD)**

6. **Wait for confirmation email**
   - You'll receive access to developer.apple.com
   - Can take up to 48 hours after payment

### Verification Status Check
```
https://developer.apple.com/account/
```
Look for "Apple Developer Program" in your membership section.

---

## Step 2: Create Developer ID Application Certificate

This certificate is used to sign command-line tools and apps distributed outside the Mac App Store.

### On Your Mac (with Keychain Access)

1. **Open Keychain Access**
   ```bash
   open -a "Keychain Access"
   ```

2. **Generate a Certificate Signing Request (CSR)**
   - Menu: Keychain Access → Certificate Assistant → Request a Certificate From a Certificate Authority
   - Enter your email address
   - Common Name: Your name or company name
   - Select: "Saved to disk"
   - Save the `.certSigningRequest` file

3. **Go to Apple Developer Certificates page**
   ```
   https://developer.apple.com/account/resources/certificates/list
   ```

4. **Create a new certificate**
   - Click the "+" button
   - Select: **Developer ID Application**
   - Click Continue

5. **Upload your CSR**
   - Upload the `.certSigningRequest` file you saved
   - Click Continue

6. **Download the certificate**
   - Download the `.cer` file
   - Double-click to install in Keychain

7. **Verify installation**
   ```bash
   security find-identity -v -p codesigning
   ```

   You should see output like:
   ```
   1) ABCD1234... "Developer ID Application: Your Name (TEAM_ID)"
   ```

### Record Your Team ID
Your Team ID is the 10-character alphanumeric code in parentheses. Note this down:
```
Team ID: __________
```

---

## Step 3: Create App-Specific Password for Notarization

Apple requires an app-specific password (not your main Apple ID password) for notarytool authentication.

1. **Go to Apple ID account page**
   ```
   https://appleid.apple.com/account/manage
   ```

2. **Sign in with your Apple ID**

3. **Navigate to App-Specific Passwords**
   - Under "Sign-In and Security"
   - Click "App-Specific Passwords"

4. **Generate a new password**
   - Click "+" or "Generate an app-specific password"
   - Label: `terraphim-notarization` (or similar)
   - Click "Create"

5. **Copy the password immediately**
   - Format: `xxxx-xxxx-xxxx-xxxx`
   - You won't be able to see it again!

   ```
   App-Specific Password: ____-____-____-____
   ```

---

## Step 4: Export Certificate for CI/CD

The certificate must be exported as a `.p12` file with a password for use in GitHub Actions.

### Export from Keychain

1. **Open Keychain Access**
   ```bash
   open -a "Keychain Access"
   ```

2. **Find your Developer ID certificate**
   - Category: "My Certificates"
   - Look for: "Developer ID Application: Your Name"

3. **Export the certificate**
   - Right-click the certificate
   - Select: "Export..."
   - Format: Personal Information Exchange (.p12)
   - Save as: `developer_id_application.p12`

4. **Set a strong export password**
   - This password will be stored in 1Password
   - Generate a strong random password

   ```
   Certificate Password: __________________
   ```

5. **Verify the export**
   ```bash
   # Check certificate info
   openssl pkcs12 -in developer_id_application.p12 -info -nokeys
   ```

### Base64 Encode for GitHub Secrets

GitHub Secrets work best with base64-encoded certificates:

```bash
# Encode the certificate
base64 -i developer_id_application.p12 -o developer_id_application.p12.b64

# Verify (should be a long string of characters)
head -c 100 developer_id_application.p12.b64
```

---

## Step 5: Store Credentials in 1Password

Create items in 1Password for secure credential storage.

### 5.1 Create Certificate Document

1. **Open 1Password**
2. **Select vault**: TerraphimPlatform (or appropriate vault)
3. **Create new item**: Document
4. **Configure**:
   - Title: `apple.developer.certificate`
   - Attach file: `developer_id_application.p12`
   - Add field "password": [certificate export password]
   - Add field "base64": [paste base64 encoded content]

### 5.2 Create Credentials Login

1. **Create new item**: Login
2. **Configure**:
   - Title: `apple.developer.credentials`
   - Username: [Your Apple ID email]
   - Add custom field "APPLE_TEAM_ID": [Your 10-char Team ID]
   - Add custom field "APPLE_APP_SPECIFIC_PASSWORD": [App-specific password]

### 1Password CLI References

After setup, your workflow will access credentials like:

```bash
# Certificate (base64)
op read "op://TerraphimPlatform/apple.developer.certificate/base64"

# Certificate password
op read "op://TerraphimPlatform/apple.developer.certificate/password"

# Apple ID
op read "op://TerraphimPlatform/apple.developer.credentials/username"

# Team ID
op read "op://TerraphimPlatform/apple.developer.credentials/APPLE_TEAM_ID"

# App-specific password
op read "op://TerraphimPlatform/apple.developer.credentials/APPLE_APP_SPECIFIC_PASSWORD"
```

---

## Step 6: Configure GitHub Secrets (Backup Method)

As a fallback if 1Password is unavailable, also store in GitHub Secrets:

1. **Go to repository settings**
   ```
   https://github.com/terraphim/terraphim-ai/settings/secrets/actions
   ```

2. **Add the following secrets**:

   | Secret Name | Value |
   |-------------|-------|
   | `APPLE_CERTIFICATE_BASE64` | Base64-encoded .p12 file content |
   | `APPLE_CERTIFICATE_PASSWORD` | Certificate export password |
   | `APPLE_ID` | Your Apple ID email |
   | `APPLE_TEAM_ID` | 10-character Team ID |
   | `APPLE_APP_SPECIFIC_PASSWORD` | App-specific password |

---

## Step 7: Test Signing Locally

Before CI integration, verify signing works on your Mac:

### Test Code Signing

```bash
# Build a test binary
cargo build --release --package terraphim_server

# Sign the binary
codesign --sign "Developer ID Application: Your Name (TEAM_ID)" \
    --options runtime \
    --timestamp \
    target/release/terraphim_server

# Verify signature
codesign --verify --deep --strict --verbose=2 target/release/terraphim_server
```

### Test Notarization

```bash
# Store credentials in notarytool (one-time setup)
xcrun notarytool store-credentials "terraphim-notarization" \
    --apple-id "your@email.com" \
    --team-id "TEAM_ID" \
    --password "xxxx-xxxx-xxxx-xxxx"

# Create a zip for notarization
zip -j terraphim_server.zip target/release/terraphim_server

# Submit for notarization
xcrun notarytool submit terraphim_server.zip \
    --keychain-profile "terraphim-notarization" \
    --wait

# Check result (should say "Accepted")
xcrun notarytool log <submission-id> \
    --keychain-profile "terraphim-notarization"
```

### Test Stapling

```bash
# Staple the notarization ticket to the binary
# Note: Stapling only works on .app, .pkg, .dmg - not bare binaries
# For CLI tools, the ticket is retrieved from Apple's servers at runtime

# Verify Gatekeeper acceptance
spctl --assess --type execute --verbose target/release/terraphim_server
```

---

## Troubleshooting

### "Developer ID Application" certificate not available
- Ensure Apple Developer Program membership is active
- Check https://developer.apple.com/account/resources/certificates/list

### Notarization rejected
- Check the log: `xcrun notarytool log <id> --keychain-profile "..."`
- Common issues:
  - Missing `--options runtime` during signing
  - Unsigned dependencies
  - Hardened runtime violations

### "errSecInternalComponent" during signing on CI
- Keychain not unlocked
- Add before signing:
  ```bash
  security unlock-keychain -p "$KEYCHAIN_PASSWORD" signing.keychain
  ```

### spctl says "rejected"
- Binary not notarized or notarization not yet propagated
- Wait a few minutes and retry
- Check Apple's notarization status page

---

## Checklist

Before proceeding to implementation, confirm:

- [ ] Apple Developer Program enrollment complete
- [ ] Developer ID Application certificate created and installed
- [ ] App-specific password generated
- [ ] Certificate exported as .p12 with password
- [ ] Certificate base64-encoded
- [ ] Credentials stored in 1Password:
  - [ ] `apple.developer.certificate` (with base64 and password fields)
  - [ ] `apple.developer.credentials` (with APPLE_TEAM_ID and APPLE_APP_SPECIFIC_PASSWORD)
- [ ] Local signing test passed
- [ ] Local notarization test passed
- [ ] GitHub Secrets configured (backup)

---

## Credentials Summary

Fill in and keep secure:

| Credential | Value | Stored In |
|------------|-------|-----------|
| Apple ID | ________________ | 1Password |
| Team ID | ________________ | 1Password |
| App-Specific Password | ____-____-____-____ | 1Password |
| Certificate Password | ________________ | 1Password |
| Certificate Path (1Password) | `op://TerraphimPlatform/apple.developer.certificate` | - |

---

## Next Steps

Once enrollment is complete and credentials are stored:

1. Run the enrollment checklist above
2. Notify when ready to proceed with implementation
3. We'll update the CI workflow with the signing pipeline
