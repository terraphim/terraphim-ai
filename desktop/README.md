# Desktop App for Terraphim AI Assistant

This is a Tauri-based desktop application with Svelte frontend for the Terraphim AI assistant.

## Architecture

- **Backend**: Rust with Tauri for system integration, search, and configuration
- **Frontend**: Svelte with Bulma CSS for the user interface
- **Features**: System tray, global shortcuts, multi-theme support, typeahead search

## Development

To run in development mode:

```sh
yarn install        # install dependencies
yarn run dev       # run the Svelte dev server
yarn run tauri dev # run the Tauri dev server
```

## Testing

We have implemented a comprehensive testing strategy covering multiple aspects:

### Backend Tests (Rust)
```sh
cd src-tauri
cargo test --verbose
```

Tests include:
- Unit tests for Tauri commands (search, config, thesaurus)
- Integration tests for state management
- Error handling and edge cases
- Async functionality testing

### Frontend Tests (Svelte)
```sh
yarn test           # Run unit tests
yarn test:watch     # Run tests in watch mode
yarn test:coverage  # Run tests with coverage
yarn test:ui        # Run tests with UI
```

Tests include:
- Component tests for Search, ThemeSwitcher, etc.
- Store and state management tests
- User interaction tests
- Mock Tauri API integration

### End-to-End Tests
```sh
yarn e2e           # Run E2E tests
yarn e2e:ui        # Run E2E tests with UI
```

Tests include:
- Complete user workflows
- Search functionality
- Navigation and routing
- Theme switching
- Error handling

### Visual Regression Tests
```sh
npx playwright test tests/visual
```

Tests include:
- Theme consistency across all 22 themes
- Responsive design testing
- Component visual consistency
- Accessibility visual checks

### Performance Tests
```sh
# Requires Lighthouse CI
npm install -g @lhci/cli
yarn build
lhci autorun
```

## Test Structure

```
desktop/
├── src-tauri/
│   ├── tests/
│   │   └── cmd_tests.rs        # Backend unit tests
│   └── src/
│       ├── cmd.rs              # Commands with test coverage
│       └── lib.rs              # Exposed for testing
├── src/
│   ├── lib/
│   │   ├── Search/
│   │   │   └── Search.test.ts  # Search component tests
│   │   └── ThemeSwitcher.test.ts # Theme tests
│   └── test-utils/
│       └── setup.ts            # Test configuration
├── tests/
│   ├── e2e/
│   │   ├── search.spec.ts      # E2E search tests
│   │   └── navigation.spec.ts   # E2E navigation tests
│   ├── visual/
│   │   └── themes.spec.ts      # Visual regression tests
│   ├── global-setup.ts         # Test data setup
│   └── global-teardown.ts      # Test cleanup
├── vitest.config.ts            # Frontend test config
└── playwright.config.ts       # E2E test config
```

## Testing Best Practices

1. **Isolation**: Each test is independent and can run in any order
2. **Mocking**: External dependencies are properly mocked
3. **Coverage**: Aim for >80% code coverage
4. **Performance**: Tests run efficiently in CI/CD
5. **Reliability**: Tests are stable and don't have flaky behavior

## Continuous Integration

Tests run automatically on:
- Push to main/develop branches
- Pull requests
- Multiple platforms (Ubuntu, macOS, Windows)

Test results include:
- Unit test results and coverage
- E2E test results with screenshots/videos
- Visual regression differences
- Performance metrics
- Security audit results

## Production

To build for production:

```sh
yarn install      # install dependencies
yarn run build    # build the Svelte app
yarn run tauri build # build the Tauri app
```

## Testing Coverage Goals

- **Backend**: >90% coverage for business logic
- **Frontend**: >85% coverage for components and stores
- **E2E**: Cover all major user workflows
- **Visual**: Test all themes and responsive breakpoints
- **Performance**: Maintain Lighthouse scores >80

## Running All Tests

To run the complete test suite:

```sh
# Install dependencies
yarn install

# Run all tests
yarn test           # Frontend unit tests
cd src-tauri && cargo test && cd .. # Backend tests
yarn e2e            # E2E tests
npx playwright test tests/visual    # Visual tests
```
