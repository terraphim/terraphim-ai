# Tauri WebDriver Tests for KG Graph Functionality

This directory contains WebDriver-based tests for the KG (Knowledge Graph) functionality in the Tauri application, using the official [Tauri WebDriver support](https://v2.tauri.app/develop/tests/webdriver/).

## Overview

The WebDriver tests provide more accurate testing of the native Tauri application behavior by using the official Tauri WebDriver implementation. This allows testing of:

- **Native Tauri Commands**: Direct testing of Tauri backend commands
- **Real Application Behavior**: Testing the actual compiled Tauri app
- **Better Integration**: More accurate representation of production behavior
- **Native Features**: Testing of native OS integrations

## Prerequisites

1. **Tauri Driver**: Install the Tauri WebDriver
   ```bash
   cargo install tauri-driver --locked
   ```

2. **Dependencies**: Install WebDriver dependencies
   ```bash
   yarn add -D selenium-webdriver
   ```

3. **Configuration**: Ensure WebDriver is enabled in `src-tauri/tauri.conf.json`
   ```json
   "plugins": {
     "webdriver": {
       "active": true
     }
   }
   ```

## Test Files

### 1. `kg-graph-webdriver.spec.ts`
Pure WebDriver test using Selenium WebDriver directly with Tauri driver.

**Features:**
- Direct WebDriver integration
- Native Tauri app testing
- Comprehensive KG graph functionality validation
- Error handling and recovery testing

### 2. `kg-graph-playwright-webdriver.spec.ts`
Playwright test with WebDriver capabilities for Tauri testing.

**Features:**
- Playwright's modern testing API
- WebDriver integration for Tauri
- Same comprehensive KG graph testing
- Better debugging and reporting

## Running the Tests

### Basic WebDriver Test
```bash
# Run the pure WebDriver test
node tests/webdriver/kg-graph-webdriver.spec.ts
```

### Playwright WebDriver Test
```bash
# Run with Playwright WebDriver config
yarn test:webdriver

# Run with UI for debugging
yarn test:webdriver:ui

# Run in headed mode
yarn test:webdriver:headed

# Run in CI mode
yarn test:webdriver:ci
```

## Test Coverage

The WebDriver tests validate the following KG graph functionality:

### ✅ **Core Functionality**
- Tauri app loading and initialization
- Search interface functionality
- Graph navigation and routing
- Graph container rendering

### ✅ **Graph Visualization**
- SVG graph element rendering
- Node and edge display
- Loading states and completion
- Error handling and recovery

### ✅ **User Interactions**
- Node click interactions (left-click and right-click)
- Modal system for document viewing
- KG context information display
- Zoom functionality with mouse wheel

### ✅ **Search Integration**
- Search with KG-related terms
- KG tags in search results
- Tag click interactions
- Document modal integration

### ✅ **Navigation and Controls**
- Navigation between search and graph pages
- Graph controls and information display
- Close buttons and modal management
- Error recovery mechanisms

## Configuration Files

### `playwright.webdriver.config.ts`
Playwright configuration specifically for WebDriver tests with:
- Single worker for WebDriver compatibility
- Extended timeouts for Tauri app startup
- WebDriver-specific browser arguments
- CI-friendly settings

### `setup.ts` and `teardown.ts`
Global setup and teardown for WebDriver tests:
- Tauri driver process management
- Proper cleanup and resource management

## Test Results

The tests provide comprehensive validation of KG graph functionality:

```
🔍 PROVING KG Graph Functionality with WebDriver...
✅ Tauri app loaded successfully
✅ Search interface is visible
✅ Search functionality working
📊 Testing graph navigation...
✅ Successfully navigated to graph page
✅ Graph container is visible
✅ Graph loaded immediately
✅ SVG graph element is visible
📊 Graph rendered: X nodes, Y edges
🎯 Testing node interactions...
✅ Node click opened modal successfully
✅ KG context information displayed
✅ Modal closed successfully
🔍 Testing zoom functionality...
✅ Zoom functionality working
🎛️ Testing graph controls...
✅ Graph controls information is displayed
🔙 Testing navigation back to search...
✅ Successfully navigated back to search page
🔍 Testing search with KG terms...
🏷️ Found X KG tags in search results
✅ KG tag click opened document modal
✅ KG context information displayed in modal
🎉 KG Graph Functionality WebDriver Test Complete!
```

## Advantages of WebDriver Tests

### **1. Native Testing**
- Tests the actual compiled Tauri application
- Validates native OS integrations
- More accurate production behavior simulation

### **2. Better Integration**
- Direct access to Tauri backend commands
- Native window management
- Real file system interactions

### **3. Comprehensive Coverage**
- End-to-end functionality validation
- Error handling and recovery testing
- Performance and stability validation

### **4. CI/CD Ready**
- Headless mode support
- Automated testing capabilities
- Detailed reporting and debugging

## Troubleshooting

### Common Issues

1. **Tauri Driver Not Found**
   ```bash
   cargo install tauri-driver --locked
   ```

2. **WebDriver Connection Issues**
   - Ensure Tauri app is running on correct port
   - Check WebDriver plugin is enabled in config
   - Verify Chrome/Chromium is installed

3. **Test Timeouts**
   - Increase timeout values in config
   - Check system resources
   - Verify Tauri app startup time

### Debug Mode
```bash
# Run with UI for visual debugging
yarn test:webdriver:ui

# Run in headed mode to see browser
yarn test:webdriver:headed
```

## Integration with CI/CD

The WebDriver tests are designed for CI/CD integration:

```yaml
# Example GitHub Actions workflow
- name: Run WebDriver Tests
  run: |
    yarn test:webdriver:ci
  env:
    CI: true
```

## Conclusion

The WebDriver tests provide the most accurate validation of KG graph functionality in the Tauri application context, ensuring that all features work correctly in the native application environment.
