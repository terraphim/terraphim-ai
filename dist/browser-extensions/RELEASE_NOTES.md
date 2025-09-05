# Terraphim Browser Extensions Release

Generated on: 2025-09-04 10:01:29

## Extensions Included

### TerraphimAIParseExtension
- **Version**: 1.0.0
- **Purpose**: Uses Knowledge Graph from Terraphim AI to parse text in a tab and replace known text with links to concepts
- **Features**: 
  - WASM-based text processing
  - Cloudflare AI integration
  - Side panel interface
  - Context menu integration
  - Configurable wiki link modes

### TerraphimAIContext  
- **Version**: 0.0.2
- **Purpose**: Searches for the selected text in Terraphim AI, Atomic Server or Logseq
- **Features**:
  - Context menu search
  - Multiple backend support
  - Quick text lookup
  - Notification support

## Installation Instructions

### For Chrome Web Store Submission
1. Use the `*-chrome.zip` files
2. Upload to Chrome Developer Dashboard
3. Fill out the store listing with appropriate descriptions

### For Development/Testing
1. Extract the `*-chrome.zip` file
2. Open Chrome and navigate to `chrome://extensions/`
3. Enable "Developer mode"
4. Click "Load unpacked" and select the extracted folder

## Security Notes

- No hardcoded API credentials are included in the packages
- All credentials must be configured through the extension options page
- API keys are stored securely using Chrome's storage.sync API
- Pre-commit hooks ensure no credentials are accidentally committed

## Files Included

- RELEASE_NOTES.md (0 bytes)
- TerraphimAIContext-v0.0.2-chrome.zip (9955 bytes)
- TerraphimAIContext-v0.0.2-source.zip (11113 bytes)
- TerraphimAIParseExtension-v1.0.0-chrome.zip (264063 bytes)
- TerraphimAIParseExtension-v1.0.0-source.zip (279006 bytes)

## Support

For issues or support, please refer to the main Terraphim AI repository:
https://github.com/terraphim/terraphim-ai
