# Terraphim Browser Extensions - Installation Guide

This guide provides step-by-step installation instructions for Terraphim AI browser extensions across all major browsers.

## Extensions Overview

### TerraphimAIParseExtension
**Purpose**: Transform web content with AI-powered knowledge graph links and semantic analysis

**Features**:
- 🧠 AI-powered text analysis using WebAssembly
- 🔗 Automatic knowledge graph link generation
- 🎯 Contextual concept recognition
- ⚡ Lightning-fast local processing
- 🛡️ Privacy-first design

### TerraphimAIContext
**Purpose**: Quick context search for selected text across multiple knowledge sources

**Features**:
- 🔍 Instant context search on selected text
- 📚 Multi-source search (Terraphim AI, Atomic Server, Logseq)
- ⚡ Fast, non-intrusive search experience
- 🎯 Smart context recognition

## Installation by Browser

### Google Chrome (Recommended)

#### Method 1: Chrome Web Store (Production)
1. Open Google Chrome
2. Visit [Chrome Web Store](https://chrome.google.com/webstore)
3. Search for "Terraphim AI Parse Extension" or "Terraphim AI Context"
4. Click **Add to Chrome**
5. Confirm by clicking **Add extension**
6. Configure settings via extension icon → Options

#### Method 2: Developer Mode (Testing/Development)
1. Download the extension ZIP file
2. Extract to a permanent folder (don't delete after installation)
3. Open Chrome and navigate to `chrome://extensions/`
4. Toggle **Developer mode** ON (top-right corner)
5. Click **Load unpacked**
6. Select the extracted extension folder
7. Configure settings via extension options

### Mozilla Firefox

#### Method 1: Firefox Add-ons (Future)
*Note: Currently requires manifest conversion for Firefox compatibility*
1. Visit [Firefox Add-ons](https://addons.mozilla.org)
2. Search for "Terraphim AI" (when available)
3. Click **Add to Firefox**
4. Confirm installation

#### Method 2: Temporary Installation (Developer)
1. Download and extract extension files
2. Open Firefox and navigate to `about:debugging`
3. Click **This Firefox** (left sidebar)
4. Click **Load Temporary Add-on**
5. Select `manifest.json` from extension folder
6. Extension loads until Firefox restart

### Microsoft Edge

#### Method 1: Microsoft Edge Add-ons
1. Open Microsoft Edge
2. Visit [Edge Add-ons Store](https://microsoftedge.microsoft.com/addons)
3. Search for "Terraphim AI" (when available)
4. Click **Get** to install
5. Confirm installation

#### Method 2: Developer Mode
1. Download and extract extension files
2. Navigate to `edge://extensions/`
3. Enable **Developer mode**
4. Click **Load unpacked**
5. Select extension folder
6. Configure through options

### Safari (macOS)

#### Current Status
**Limited Support**: Safari requires Swift-based Safari Web Extensions development.

**Recommended Alternatives**:
- Install Chrome, Firefox, or Edge on macOS
- Use those browsers for full Terraphim functionality
- Safari support planned for future releases

### Brave Browser

#### Chrome Web Store Compatibility
*Brave uses Chromium and supports Chrome extensions natively*

1. Open Brave browser
2. Visit [Chrome Web Store](https://chrome.google.com/webstore)
3. Search for "Terraphim AI"
4. Click **Add to Chrome** (works in Brave)
5. Confirm installation
6. Configure through extension options

### Opera Browser

#### Method 1: Opera Add-ons
1. Visit [Opera Add-ons](https://addons.opera.com)
2. Search for "Terraphim AI" (when available)
3. Click **Add to Opera**
4. Confirm installation

#### Method 2: Chrome Extension Compatibility
1. Enable "Install Chrome extensions" in Opera settings
2. Visit Chrome Web Store
3. Install directly (Chrome extensions work in Opera)

### Vivaldi Browser

#### Chrome Extension Support
1. Open Vivaldi Settings
2. Navigate to Extensions section
3. Enable "Install Extensions from Chrome Web Store"
4. Visit Chrome Web Store
5. Install Terraphim AI extensions normally

## Post-Installation Setup

### 1. Configure Terraphim Server Connection

**For Local Development**:
1. Start your Terraphim server locally
2. Open extension options
3. Enable "Auto-discovery" mode
4. Or manually enter: `http://localhost:8000`

**For Production**:
1. Open extension options
2. Enter your Terraphim server URL
3. Configure authentication if required

### 2. Setup Cloudflare AI (Optional)

1. Get Cloudflare Account ID and API Token:
   - Visit [Cloudflare Dashboard](https://dash.cloudflare.com)
   - Go to Profile → API Tokens
   - Create token with AI:Read permissions
   - Copy Account ID from right sidebar

2. Configure in extension options:
   - Enter Account ID (32-character hex string)
   - Enter API Token
   - Settings are encrypted and synced across devices

### 3. Test Installation

**TerraphimAIParseExtension**:
1. Visit any webpage with text content
2. Click extension icon to open side panel
3. Select text processing mode
4. Process page content
5. Verify knowledge graph links appear

**TerraphimAIContext**:
1. Select any text on a webpage
2. Right-click to open context menu
3. Choose "Search in Terraphim AI"
4. Verify search results notification

## Troubleshooting

### Extension Not Loading
- **Chrome/Edge**: Check Developer mode is enabled
- **Firefox**: Ensure manifest.json is selected, not folder
- **All browsers**: Verify folder permissions and file integrity

### Connection Issues
- Verify Terraphim server is running and accessible
- Check firewall/network settings
- Test server health endpoint manually
- Review extension console for error messages

### Performance Issues
- Clear browser cache and restart
- Disable other extensions temporarily
- Check available system memory
- Verify WASM support in browser

### Permission Errors
- Review extension permissions in browser settings
- Ensure storage permissions are granted
- Check for corporate browser policies blocking extensions

## Advanced Installation

### Enterprise Deployment
1. **Package Distribution**:
   - Host `.crx` files on internal servers
   - Use Chrome Enterprise policies
   - Deploy via Active Directory or MDM

2. **Configuration Management**:
   - Pre-configure server URLs
   - Set default API endpoints
   - Lock certain settings for compliance

### Development Setup
1. **Build from Source**:
   ```bash
   git clone https://github.com/terraphim/terraphim-ai
   cd terraphim-ai
   ./scripts/build-browser-extensions.sh
   ```

2. **Load Development Version**:
   - Use developer mode in any browser
   - Point to `browser_extensions/` directory
   - Enable debugging for development

## Security Considerations

### Data Privacy
- Extensions store credentials securely using browser APIs
- No hardcoded credentials in source code
- All API communications use HTTPS
- Local processing prioritized over cloud APIs

### Permission Model
Extensions request minimal permissions:
- **Storage**: For secure credential management
- **ActiveTab**: For processing current page content
- **ContextMenus**: For right-click search functionality
- **Host Permissions**: Only for configured Terraphim servers

### Safe Installation
- Only install from official sources
- Verify extension publisher information
- Review requested permissions before installation
- Keep extensions updated for security patches

## Support

### Getting Help
- Check browser console for error messages
- Verify Terraphim server connectivity
- Review extension options configuration
- Test with minimal configuration first

### Reporting Issues
Include in bug reports:
- Browser type and version
- Extension version
- Error messages from browser console
- Steps to reproduce the issue
- Server configuration details (without credentials)

### Community Resources
- [Terraphim AI Repository](https://github.com/terraphim/terraphim-ai)
- Extension documentation in `browser_extensions/`
- Build and release guides for developers

---

**Note**: Always download extensions from official sources and verify publisher authenticity before installation.
