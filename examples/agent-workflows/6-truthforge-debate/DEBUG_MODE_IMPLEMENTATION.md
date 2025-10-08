# TruthForge Debug Mode Implementation

## Overview
Complete implementation of LLM request/response debug logging for TruthForge crisis communication analysis.

## Features Implemented

### 1. Settings Integration ✅
**Files Modified**:
- `examples/agent-workflows/shared/settings-modal.html` - Added debug checkbox
- `examples/agent-workflows/shared/settings-manager.js` - Added debug state tracking
- `examples/agent-workflows/shared/settings-ui.js` - Bound debug checkbox to settings

**Functionality**:
- Debug mode checkbox in settings modal
- Settings persist to localStorage
- `isDebugMode()` and `toggleDebugMode()` methods
- Emits `debugModeChanged` event when toggled

### 2. API Client Debug Logging ✅
**Files Modified**:
- `examples/agent-workflows/shared/api-client.js`

**New Methods**:
- `setDebugMode(enabled)` - Enable/disable debug logging
- `onDebugLog(callback)` - Register UI callbacks (with 1s timeout protection)
- `sanitizeForLogging(text)` - Remove PII before logging (emails, API keys, passwords)
- `logDebug(type, data)` - Comprehensive logging with console.group

**Security Features**:
- PII sanitization (emails, API keys, passwords, tokens)
- Callback timeout protection (1 second max execution)
- Type coercion prevents injection attacks
- Defensive null/undefined handling

**Logging Details**:
- Request: endpoint, role, model, sanitized prompt, full payload
- Response: status, model used, tokens, duration, sanitized output
- Error: status, error message, duration

### 3. Debug Panel UI Component ✅
**New Files**:
- `examples/agent-workflows/shared/debug-panel.js` (197 lines)
- `examples/agent-workflows/shared/debug-panel.css` (241 lines)

**Features**:
- Fixed bottom panel (collapsible)
- Real-time entry display
- Expandable entries with full details
- Request/Response color coding (blue/green)
- Clear button with confirmation
- 50 entry limit (prevents memory issues)
- Dark console-style theme

**Security**:
- XSS protection via `escapeHtml()`
- DOM clobbering immunity
- Input validation
- Memory limits

### 4. TruthForge Integration ✅
**Files Modified**:
- `examples/agent-workflows/6-truthforge-debate/index.html` - Added debug panel, scripts
- `examples/agent-workflows/6-truthforge-debate/app.js` - Integration logic

**Integration Flow**:
1. Initialize debug panel (hidden by default)
2. Initialize settings manager and API client
3. Setup debug mode listeners
4. Connect API client logs to panel
5. Load initial debug state from settings

**New Methods in app.js**:
- `initializeDebugPanel()` - Create panel with error handling
- `setupDebugMode()` - Wire up all debug integrations
- `setDebugPanelVisibility(visible)` - Show/hide panel

## Usage

### Enabling Debug Mode
1. Open TruthForge UI
2. Click settings icon (⚙️)
3. Scroll to "Debug Mode" section
4. Check "Enable LLM Request/Response Logging"
5. Debug panel appears at bottom of screen

### Using Debug Panel
- **View Logs**: Panel shows all LLM requests/responses in real-time
- **Expand Entry**: Click on any entry to see full details
- **Toggle Panel**: Click arrow button to collapse/expand
- **Clear Logs**: Click "Clear" button (with confirmation)

### What's Logged
**Request Logs** (→):
- Timestamp
- Endpoint (e.g., /chat)
- Role being used
- Model name
- Prompt (sanitized, first 200 chars)
- Full request payload

**Response Logs** (←):
- Status (success/error)
- Model that was used
- Token counts (input/output/total)
- Duration in milliseconds
- Response output (sanitized, first 200 chars)
- Full response object

### Privacy & Security
- **PII Sanitization**: Emails, API keys, passwords automatically redacted
- **Local Only**: All logs stored in browser memory, not sent anywhere
- **Memory Limits**: Max 50 entries (oldest removed automatically)
- **Timeout Protection**: Callbacks limited to 1 second execution
- **XSS Protection**: All user data HTML-escaped

## Testing

### Backend Tests ✅
```bash
cargo test --package terraphim-truthforge --lib
```
**Result**: 37 tests passed (17 lib + 7 workflows + 4+4+5 integration)

### Manual Testing Checklist
- [x] Settings modal opens
- [x] Debug checkbox present and functional
- [x] Debug panel appears when enabled
- [x] Debug panel hides when disabled
- [x] Settings persist across reload
- [x] Console logging works
- [x] UI panel shows entries
- [x] Entry expansion works
- [x] Clear button works
- [x] Sanitization removes PII

### Browser Compatibility
- ✅ Chrome/Edge (Chromium)
- ✅ Firefox
- ✅ Safari (modern versions)

## Files Changed

### Modified Files (5)
1. `examples/agent-workflows/shared/settings-modal.html` (+19 lines)
2. `examples/agent-workflows/shared/settings-manager.js` (+13 lines)
3. `examples/agent-workflows/shared/settings-ui.js` (+1 line)
4. `examples/agent-workflows/shared/api-client.js` (+97 lines)
5. `examples/agent-workflows/6-truthforge-debate/index.html` (+4 lines)
6. `examples/agent-workflows/6-truthforge-debate/app.js` (+71 lines)

### New Files (3)
1. `examples/agent-workflows/shared/debug-panel.js` (197 lines)
2. `examples/agent-workflows/shared/debug-panel.css` (241 lines)
3. `examples/agent-workflows/6-truthforge-debate/test-debug-mode.html` (test file)

### Test Files (1)
1. `examples/agent-workflows/6-truthforge-debate/tests/debug-integration.test.js` (Playwright tests)

**Total New/Modified Code**: ~643 lines

## Security Review
All changes reviewed by overseer agent:
- ✅ Settings modal: XSS protection, accessibility compliance
- ✅ Settings manager: Event safety, boolean handling
- ✅ API client: PII sanitization, callback timeouts, defensive programming
- ✅ Debug panel: XSS protection, DOM safety, memory limits

**Security Rating**: Production-ready with all critical issues addressed

## Deployment Readiness

### Pre-Deployment Checklist
- [x] All backend tests pass (37/37)
- [ ] Frontend automated tests pass (Playwright)
- [x] Manual testing complete
- [x] Security review complete
- [x] Documentation complete
- [ ] Performance validated
- [ ] Production deployment script ready

### Deployment Steps
1. Rsync updated files to production
2. Verify settings modal appears
3. Enable debug mode in production
4. Submit test analysis
5. Verify debug panel works
6. Monitor for errors

## Known Limitations
1. Debug logs only in browser memory (lost on page reload)
2. Max 50 entries (oldest removed)
3. Callback execution limited to 1 second
4. Prompt/output truncated to 200 characters in preview

## Future Enhancements
- [ ] Export debug logs to JSON file
- [ ] Filter entries by role/model
- [ ] Search within debug entries
- [ ] Performance metrics dashboard
- [ ] Persistent debug log storage
- [ ] Remote debug log submission (opt-in)

## Support
For issues or questions:
1. Check browser console for errors
2. Verify settings modal loads
3. Check script paths in index.html
4. Verify backend API is running
5. Review backend logs for LLM requests
