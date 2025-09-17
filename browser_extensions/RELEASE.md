# Terraphim Browser Extensions - Release Guide

This document outlines the complete process for releasing Terraphim browser extensions to the Chrome Web Store and other distribution channels.

## Pre-Release Checklist

### Code Quality
- [ ] All security checks pass (`./scripts/check-api-keys.sh`)
- [ ] No hardcoded API credentials in source code
- [ ] WASM module builds successfully
- [ ] All extension manifests are valid JSON
- [ ] Extensions load and function correctly in Chrome

### Version Management
- [ ] Version numbers updated in `manifest.json` files
- [ ] Version follows semantic versioning (MAJOR.MINOR.PATCH)
- [ ] Changelog updated with new features and fixes
- [ ] Git tags created for release versions

### Testing
- [ ] Extensions tested in Chrome (latest stable)
- [ ] All core functionality verified
- [ ] API integrations working (Terraphim server, Cloudflare)
- [ ] Security features validated (credential storage, etc.)
- [ ] Cross-platform testing completed

## Build and Package Process

### 1. Prepare Release Branch

```bash
# Create release branch
git checkout -b release/browser-extensions-v1.0.0

# Update version numbers in manifests
# TerraphimAIParseExtension/manifest.json
# TerraphimAIContext/manifest.json

# Commit version updates
git add browser_extensions/*/manifest.json
git commit -m "chore: bump browser extension versions for release"
```

### 2. Build Extensions

```bash
# Clean previous builds
rm -rf dist/browser-extensions/

# Build extensions
./scripts/build-browser-extensions.sh

# Verify build success
echo $?  # Should be 0
```

### 3. Package for Distribution

```bash
# Create release packages
./scripts/package-browser-extensions.sh

# Verify packages created
ls -la dist/browser-extensions/
```

### 4. Quality Assurance

```bash
# Validate package integrity
cd dist/browser-extensions/

# Check ZIP files
zip -T *.zip

# Verify contents
unzip -l TerraphimAIParseExtension-v*-chrome.zip | head -20
unzip -l TerraphimAIContext-v*-chrome.zip
```

## Chrome Web Store Submission

### Developer Console Setup

1. **Access Chrome Web Store Developer Console**
   - Go to: https://chrome.google.com/webstore/developer/dashboard
   - Sign in with Google account
   - Ensure you have a developer account (one-time $5 registration fee)

2. **Create New Items (First Time Only)**
   - Click "Add new item"
   - Upload ZIP package (`*-chrome.zip` file)
   - Fill out store listing information

### Store Listing Information

#### TerraphimAIParseExtension

**Name**: Terraphim AI - Knowledge Graph Parser

**Summary**: Transform web content with AI-powered knowledge graph links and semantic analysis

**Description**:
```
Terraphim AI Parse Extension enhances your browsing experience by intelligently analyzing web content and creating contextual links to relevant concepts in your personal knowledge graph.

KEY FEATURES:
• 🧠 AI-powered text analysis using WebAssembly for maximum performance
• 🔗 Automatic knowledge graph link generation with multiple formatting options
• 🎯 Contextual concept recognition and semantic matching
• ⚡ Lightning-fast processing with local WASM computation
• 🛡️ Privacy-first design - your data never leaves your control
• 🎨 Configurable wiki-link styles and output formats

KNOWLEDGE INTEGRATION:
• Connect with your Terraphim AI server (local or cloud)
• Support for multiple knowledge repositories
• Seamless integration with personal knowledge management systems
• Real-time concept matching and link generation

SECURITY & PRIVACY:
• No hardcoded credentials - configure securely through options
• Encrypted storage of API credentials
• Local processing with optional cloud AI enhancement
• Full control over data sharing and privacy settings

Perfect for researchers, students, knowledge workers, and anyone who wants to create meaningful connections between web content and their personal knowledge base.

Setup is simple: install the extension, configure your Terraphim server connection, and start browsing with enhanced semantic understanding!
```

**Category**: Productivity

**Language**: English

**Small Tile Icon**: 128x128 PNG (create from assets)
**Screenshots**: 1280x800 PNG showing extension in action (create)
**Promotional Images**: Various sizes as required

#### TerraphimAIContext

**Name**: Terraphim AI - Context Search

**Summary**: Quick context search for selected text across multiple knowledge sources

**Description**:
```
Terraphim AI Context Extension enables lightning-fast contextual search across your knowledge repositories with just a right-click.

KEY FEATURES:
• 🔍 Instant context search on selected text
• 📚 Multi-source search across Terraphim AI, Atomic Server, and Logseq
• ⚡ Fast, non-intrusive search experience
• 🎯 Smart context recognition and relevant results
• 🔔 Discrete notifications for search results

SEARCH CAPABILITIES:
• Right-click any selected text to search instantly
• Support for multiple backend knowledge systems
• Configurable search providers and endpoints
• Quick access to relevant documents and concepts

PRIVACY FOCUSED:
• Secure credential management
• No data retention or tracking
• Local-first with optional cloud integration
• Full control over search sources and privacy

Perfect for researchers, writers, and knowledge workers who need quick access to contextual information without interrupting their workflow.

Simply select text on any webpage, right-click, and search across your knowledge base instantly!
```

**Category**: Productivity

### Submission Process

1. **Upload Packages**
   - Upload `TerraphimAIParseExtension-v*-chrome.zip`
   - Upload `TerraphimAIContext-v*-chrome.zip`

2. **Complete Store Listings**
   - Fill out all required fields
   - Add screenshots and promotional images
   - Set pricing (free)
   - Configure distribution regions

3. **Privacy Compliance**
   - Complete privacy policy requirements
   - Declare data usage and permissions
   - Ensure GDPR compliance

4. **Submit for Review**
   - Click "Submit for Review"
   - Monitor review status in developer console
   - Respond to any review feedback promptly

### Review Process

**Typical Timeline**: 1-7 days for review

**Common Review Issues**:
- Permissions not clearly justified
- Missing or inadequate privacy policy
- Package integrity issues
- Functionality not matching description

**Review Requirements**:
- All permissions must be justified
- Extension must function as described
- No prohibited content or practices
- Compliance with Chrome Web Store policies

## Post-Release Activities

### 1. Version Tagging

```bash
# Tag release in Git
git tag -a v1.0.0-browser-extensions -m "Browser Extensions v1.0.0 release"
git push origin v1.0.0-browser-extensions

# Merge back to main
git checkout main
git merge release/browser-extensions-v1.0.0
git push origin main
```

### 2. Release Documentation

```bash
# Update release notes
# Document new features, bug fixes, and changes
# Include installation and upgrade instructions

# Archive release packages
# Store ZIP files and source archives in secure location
# Maintain version history and release artifacts
```

### 3. User Communication

- Update project README with extension links
- Announce on relevant channels (if applicable)
- Prepare user documentation and setup guides
- Monitor for user feedback and issues

## Maintenance and Updates

### Regular Updates

**Security Updates**: Critical patches should be released immediately
**Feature Updates**: Plan for regular feature releases (monthly/quarterly)
**Bug Fixes**: Address reported issues promptly

### Update Process

1. **Prepare Update**
   - Fix bugs or add features
   - Update version numbers
   - Test thoroughly

2. **Build and Package**
   - Use same build process as initial release
   - Ensure all security checks pass

3. **Submit Update**
   - Upload new packages to Chrome Web Store
   - Update store listing if needed
   - Submit for review

### Monitoring

- **Chrome Web Store Console**: Monitor reviews, ratings, and user feedback
- **Analytics**: Track installation numbers and user engagement
- **Error Reporting**: Monitor for runtime errors and crashes
- **Security Alerts**: Watch for security vulnerability reports

## Emergency Procedures

### Critical Security Issue

1. **Immediate Response**
   - Remove extension from store if severe
   - Assess impact and affected users
   - Prepare emergency patch

2. **Communication**
   - Notify users through available channels
   - Document issue and resolution
   - Update security documentation

3. **Resolution**
   - Develop and test fix
   - Expedite review process
   - Deploy fix as soon as approved

### Store Policy Violations

1. **Review Violation Notice**
   - Understand specific policy violation
   - Assess required changes

2. **Corrective Action**
   - Implement required changes
   - Update extension accordingly
   - Resubmit for review

3. **Prevention**
   - Update development practices
   - Enhance testing procedures
   - Document lessons learned

## Installation Instructions for Major Browsers

### Chrome (Primary Support)

#### Chrome Web Store Installation
1. Go to [Chrome Web Store](https://chrome.google.com/webstore)
2. Search for "Terraphim AI"
3. Click "Add to Chrome" for the desired extension
4. Click "Add extension" in the confirmation dialog
5. Configure settings through the extension options page

#### Developer Mode Installation (for testing)
1. Download the extension ZIP file
2. Extract the contents to a local folder
3. Open Chrome and navigate to `chrome://extensions/`
4. Enable "Developer mode" toggle in the top right
5. Click "Load unpacked" button
6. Select the extracted extension folder
7. The extension will appear in your extensions list

### Mozilla Firefox

#### Firefox Add-ons Store
*Note: Firefox versions require manifest adaptation (V2 to V3 conversion)*
1. Visit [Firefox Add-ons](https://addons.mozilla.org)
2. Search for "Terraphim AI" (when available)
3. Click "Add to Firefox"
4. Confirm installation in the popup dialog

#### Temporary Installation (Developer)
1. Download and extract the extension
2. Open Firefox and navigate to `about:debugging`
3. Click "This Firefox" in the left sidebar
4. Click "Load Temporary Add-on"
5. Select the `manifest.json` file from the extension folder
6. Extension will be loaded until Firefox restarts

### Microsoft Edge

#### Microsoft Edge Add-ons Store
1. Visit [Microsoft Edge Add-ons](https://microsoftedge.microsoft.com/addons)
2. Search for "Terraphim AI" (when available)
3. Click "Get" to install
4. Confirm installation

#### Developer Mode Installation
1. Download and extract extension files
2. Open Edge and navigate to `edge://extensions/`
3. Enable "Developer mode" toggle
4. Click "Load unpacked"
5. Select the extension folder
6. Configure through extension options

### Opera

#### Opera Add-ons Store
1. Visit [Opera Add-ons](https://addons.opera.com)
2. Search for "Terraphim AI"
3. Click "Add to Opera"
4. Confirm installation

#### Chrome Extension Compatibility
*Opera supports Chrome extensions natively*
1. Enable "Install Chrome extensions" in Opera settings
2. Visit Chrome Web Store
3. Install directly from Chrome Web Store
4. Extensions will work natively in Opera

### Safari (macOS)

#### Safari Web Extensions (Future Support)
*Note: Requires additional development for Safari compatibility*
1. Download Safari-compatible version (when available)
2. Open Safari Preferences > Extensions
3. Install downloaded extension package
4. Enable in Extensions preferences

#### Current Workaround
- Use Chrome, Firefox, or Edge on macOS for full functionality
- Safari Web Extensions support planned for future releases

### Brave Browser

#### Chrome Web Store Compatibility
*Brave uses Chromium engine and supports Chrome extensions*
1. Visit Chrome Web Store
2. Search for "Terraphim AI"
3. Click "Add to Chrome" (works in Brave)
4. Confirm installation
5. Configure through extension options

### Vivaldi Browser

#### Chrome Extension Support
1. Open Vivaldi Settings
2. Go to Extensions section
3. Enable "Install Extensions from Chrome Web Store"
4. Visit Chrome Web Store
5. Install Terraphim AI extensions normally

## Distribution Alternatives

### Direct Distribution

For enterprise or specialized deployments:

1. **Enterprise Deployment**
   - Host `.crx` files internally
   - Use Chrome Enterprise policies for deployment
   - Maintain update channels

2. **Developer Mode Installation**
   - Provide unpacked extension downloads
   - Include installation instructions
   - Support for testing and development

### Browser-Specific Considerations

**Chrome/Chromium-based browsers**: Full support with Manifest V3
**Firefox**: Requires manifest adaptation for V2 compatibility
**Safari**: Requires Swift/Safari Web Extensions development
**Mobile browsers**: Limited extension support on mobile platforms

## Success Metrics

### Key Performance Indicators

- **Installation Numbers**: Track growth over time
- **User Ratings**: Maintain high rating (4+ stars)
- **Review Sentiment**: Monitor user feedback
- **Error Rates**: Keep runtime errors low
- **Update Adoption**: Track update installation rates

### Quality Metrics

- **Review Process**: Maintain fast approval times
- **Security Incidents**: Zero tolerance for security issues
- **User Support**: Respond to issues promptly
- **Code Quality**: Maintain high code quality standards

## Support and Resources

### Documentation
- Chrome Web Store Developer Documentation
- Chrome Extension Development Guide
- Terraphim AI API Documentation

### Tools
- Chrome Web Store Developer Console
- Chrome Extension Source Viewer (for testing)
- Browser Developer Tools

### Community
- Chrome Extensions Google Group
- Stack Overflow (chrome-extension tag)
- Terraphim AI Community (internal)

---

**Important**: Always test extensions thoroughly before release and maintain security best practices throughout the release process.
