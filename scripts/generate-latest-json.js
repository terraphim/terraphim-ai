#!/usr/bin/env node
/**
 * generate-latest-json.js - Generate updater manifest for Terraphim Desktop
 *
 * This script creates the latest.json file required by Tauri's updater.
 * It uses 1Password CLI to sign the update packages.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { execSync } = require('child_process');

// Configuration
const PROJECT_ROOT = path.dirname(__dirname);
const BUNDLE_DIR = path.join(PROJECT_ROOT, 'desktop', 'target', 'release', 'bundle');

// Colors for console output
const colors = {
    reset: '\x1b[0m',
    bright: '\x1b[1m',
    red: '\x1b[31m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m'
};

function log(message, color = 'reset') {
    console.log(`${colors[color]}${message}${colors.reset}`);
}

function error(message) {
    log(`ERROR: ${message}`, 'red');
    process.exit(1);
}

function success(message) {
    log(`✅ ${message}`, 'green');
}

function info(message) {
    log(`ℹ️  ${message}`, 'blue');
}

function getVersion() {
    try {
        const packageJson = JSON.parse(fs.readFileSync(path.join(PROJECT_ROOT, 'desktop', 'src-tauri', 'tauri.conf.json'), 'utf8'));
        return packageJson.package.version;
    } catch (err) {
        // Fallback to git tag or environment
        const gitTag = process.env.GITHUB_REF_NAME || process.env.GITHUB_REF;
        if (gitTag) {
            return gitTag.replace(/^v/, '').replace(/^app-v/, '');
        }
        return '0.2.0';
    }
}

function getBaseUrl(version) {
    const repo = process.env.GITHUB_REPOSITORY || 'terraphim/terraphim-ai';
    const tag = version.startsWith('v') ? version : `v${version}`;
    return `https://github.com/${repo}/releases/download/${tag}`;
}

function findBundleFiles() {
    const files = {
        'darwin-x86_64': null,
        'darwin-aarch64': null,
        'linux-x86_64': null,
        'windows-x86_64': null
    };

    if (!fs.existsSync(BUNDLE_DIR)) {
        info(`Bundle directory not found: ${BUNDLE_DIR}`);
        return files;
    }

    // Look for macOS bundles
    const macOSDir = path.join(BUNDLE_DIR, 'macos');
    if (fs.existsSync(macOSDir)) {
        const macFiles = fs.readdirSync(macOSDir);
        const dmgFiles = macFiles.filter(f => f.endsWith('.dmg'));

        for (const dmg of dmgFiles) {
            if (dmg.includes('x64') || dmg.includes('x86_64')) {
                files['darwin-x86_64'] = path.join(macOSDir, dmg);
            } else if (dmg.includes('aarch64') || dmg.includes('arm64')) {
                files['darwin-aarch64'] = path.join(macOSDir, dmg);
            } else {
                // Default to aarch64 for Apple Silicon
                files['darwin-aarch64'] = path.join(macOSDir, dmg);
            }
        }
    }

    // Look for Linux bundles
    const linuxDir = path.join(BUNDLE_DIR, 'appimage');
    if (fs.existsSync(linuxDir)) {
        const linuxFiles = fs.readdirSync(linuxDir);
        const appImageFiles = linuxFiles.filter(f => f.endsWith('.AppImage'));
        if (appImageFiles.length > 0) {
            files['linux-x86_64'] = path.join(linuxDir, appImageFiles[0]);
        }
    }

    // Look for Windows bundles
    const windowsDir = path.join(BUNDLE_DIR, 'nsis');
    if (fs.existsSync(windowsDir)) {
        const windowsFiles = fs.readdirSync(windowsDir);
        const exeFiles = windowsFiles.filter(f => f.endsWith('.exe'));
        if (exeFiles.length > 0) {
            files['windows-x86_64'] = path.join(windowsDir, exeFiles[0]);
        }
    }

    return files;
}

function generateSignature(filePath) {
    if (!fs.existsSync(filePath)) {
        return '';
    }

    try {
        // Use 1Password CLI to get the private key and sign
        const privateKey = execSync('op item get "Tauri Update Signing" --vault "Terraphim-Deployment" --field "TAURI_PRIVATE_KEY"', {
            encoding: 'utf8',
            stdio: 'pipe'
        }).trim();

        // Create temporary key file
        const tempKeyFile = path.join('/tmp', `tauri-key-${Date.now()}.key`);
        fs.writeFileSync(tempKeyFile, privateKey);
        fs.chmodSync(tempKeyFile, 0o600);

        try {
            // Use Tauri CLI to sign the file
            const signature = execSync(`cd ${PROJECT_ROOT}/desktop && npm run tauri signer sign -- -k ${tempKeyFile} -f ${filePath}`, {
                encoding: 'utf8',
                stdio: 'pipe'
            }).trim();

            // Clean up temp key
            fs.unlinkSync(tempKeyFile);

            return signature;
        } catch (signError) {
            // Clean up temp key on error
            fs.unlinkSync(tempKeyFile);
            throw signError;
        }
    } catch (err) {
        log(`Warning: Could not generate signature for ${filePath}: ${err.message}`, 'yellow');
        return '';
    }
}

function generatePlatformEntry(filePath, baseUrl, filename) {
    if (!filePath || !fs.existsSync(filePath)) {
        return null;
    }

    const signature = generateSignature(filePath);
    const url = `${baseUrl}/${filename}`;

    return {
        signature,
        url,
        with_elevated_task: false
    };
}

function generateManifest() {
    info('Generating Tauri updater manifest...');

    const version = getVersion();
    const baseUrl = getBaseUrl(version);
    const bundleFiles = findBundleFiles();

    info(`Version: ${version}`);
    info(`Base URL: ${baseUrl}`);

    const platforms = {};

    // macOS Intel
    if (bundleFiles['darwin-x86_64']) {
        const filename = path.basename(bundleFiles['darwin-x86_64']);
        platforms['darwin-x86_64'] = generatePlatformEntry(
            bundleFiles['darwin-x86_64'],
            baseUrl,
            filename
        );
        info(`Found macOS Intel bundle: ${filename}`);
    }

    // macOS Apple Silicon
    if (bundleFiles['darwin-aarch64']) {
        const filename = path.basename(bundleFiles['darwin-aarch64']);
        platforms['darwin-aarch64'] = generatePlatformEntry(
            bundleFiles['darwin-aarch64'],
            baseUrl,
            filename
        );
        info(`Found macOS ARM64 bundle: ${filename}`);
    }

    // Linux
    if (bundleFiles['linux-x86_64']) {
        const filename = path.basename(bundleFiles['linux-x86_64']);
        platforms['linux-x86_64'] = generatePlatformEntry(
            bundleFiles['linux-x86_64'],
            baseUrl,
            filename
        );
        info(`Found Linux bundle: ${filename}`);
    }

    // Windows
    if (bundleFiles['windows-x86_64']) {
        const filename = path.basename(bundleFiles['windows-x86_64']);
        platforms['windows-x86_64'] = generatePlatformEntry(
            bundleFiles['windows-x86_64'],
            baseUrl,
            filename
        );
        info(`Found Windows bundle: ${filename}`);
    }

    // Remove null entries
    Object.keys(platforms).forEach(key => {
        if (platforms[key] === null) {
            delete platforms[key];
        }
    });

    const manifest = {
        version,
        notes: `Terraphim Desktop v${version} - Auto-update enabled`,
        pub_date: new Date().toISOString(),
        platforms
    };

    const manifestPath = path.join(PROJECT_ROOT, 'latest.json');
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));

    success(`Generated updater manifest: ${manifestPath}`);
    info(`Platforms included: ${Object.keys(platforms).join(', ')}`);

    return manifest;
}

function main() {
    try {
        log('🚀 Tauri Updater Manifest Generator', 'bright');

        const manifest = generateManifest();

        // Output summary
        console.log('\n' + JSON.stringify(manifest, null, 2));

        success('Updater manifest generated successfully!');
    } catch (err) {
        error(`Failed to generate manifest: ${err.message}`);
    }
}

if (require.main === module) {
    main();
}
