# 🎯 Tauri Setup Instructions

## Current State
Your `tauri.conf.json` has a hardcoded public key but no proper 1Password integration.

## 🔐 Tauri Signing Setup

### **Option 1: Manual Setup (Quick)**
1. **Get your keys**:
   ```bash
   # If you have access to 1Password
   op signin --account my.1password.com
   op read "op://TerraphimPlatform/TauriSigning/TAURI_PRIVATE_KEY"
   op read "op://TerraphimPlatform/TauriSigning/TAURI_PUBLIC_KEY" 
   op read "op://TerraphimPlatform/TauriSigning/credential"
   ```

2. **Update tauri.conf.json manually**:
   ```json
   {
     "tauri": {
       "bundle": {
         "targets": "all",
         "identifier": "com.terraphim.ai.desktop",
         "signing": {
           "privateKey": "YOUR_TAURI_PRIVATE_KEY_HERE",
           "publicKey": "YOUR_TAURI_PUBLIC_KEY_HERE", 
           "credential": "YOUR_TAURI_CREDENTIAL_HERE"
         }
       }
     }
   }
   ```

### **Option 2: Automated Setup (Recommended)**

Run the provided setup script:
```bash
# Setup Tauri signing with 1Password integration
./scripts/setup-tauri-signing.sh
```

This will:
- ✅ Read keys from 1Password `TerraphimPlatform` vault
- ✅ Create local `.tauriconfig` 
- ✅ Set environment variables for current session
- ✅ Configure Tauri to auto-sign during builds

## 🚀 Build Signed Packages

After setting up signing, build with:
```bash
cd desktop
yarn tauri build --bundles deb rpm appimage --target x86_64-unknown-linux-gnu

# Or use the comprehensive build script
./packaging/scripts/build-all-formats.sh 1.0.0
```

## 🔧 If 1Password Access Issues

If you can't access the `TerraphimPlatform` vault:

1. **Create temporary keys for testing**:
   ```bash
   # Generate temporary keys
   cargo tauri keygen --name "Terraphim Test" --email "test@terraphim.ai"
   
   # Use these keys in tauri.conf.json temporarily
   ```

2. **Contact your team** to get proper access to:
   - `TerraphimPlatform/TauriSigning/TAURI_PRIVATE_KEY`
   - `TerraphimPlatform/TauriSigning/TAURI_PUBLIC_KEY` 
   - `TerraphimPlatform/TauriSigning/credential`

## 📋 Current Configuration Analysis

**Current tauri.conf.json issues:**
- ❌ Hardcoded public key (not secure)
- ❌ No private key configuration
- ❌ No 1Password integration
- ❌ No signing setup for builds

**After setup:**
- ✅ Secure 1Password integration
- ✅ Automatic key management
- ✅ Local key caching via `.tauriconfig`
- ✅ Environment variables for builds
- ✅ Proper key rotation capability

## 🚨 Security Notes

- **Never commit private keys** to git repository
- **Use environment variables** for build-time signing
- **Rotate keys regularly** via 1Password
- **Test signature verification** after builds

## 🎯 Next Steps

1. Run `./scripts/setup-tauri-signing.sh`
2. Test with a small build: `yarn tauri build --bundles deb`
3. Verify signatures: `yarn tauri signer verify`
4. Proceed with full release build