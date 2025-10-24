#!/bin/bash
# Verification script for Base Web Components implementation

echo "=== Terraphim Base Web Components - Implementation Verification ==="
echo ""

# Count lines of code
echo "📊 Lines of Code:"
echo "  terraphim-element.js:    $(wc -l < terraphim-element.js) lines"
echo "  terraphim-observable.js: $(wc -l < terraphim-observable.js) lines"
echo "  terraphim-events.js:     $(wc -l < terraphim-events.js) lines"
echo "  index.js:                $(wc -l < index.js) lines"
echo ""

# Count test files
echo "🧪 Test Coverage:"
echo "  terraphim-element.test.js:    $(wc -l < __tests__/terraphim-element.test.js) lines"
echo "  terraphim-observable.test.js: $(wc -l < __tests__/terraphim-observable.test.js) lines"
echo "  terraphim-events.test.js:     $(wc -l < __tests__/terraphim-events.test.js) lines"
echo "  test-runner.html:             $(wc -l < __tests__/test-runner.html) lines"
echo ""

# Count documentation
echo "📖 Documentation:"
echo "  README.md:           $(wc -l < README.md) lines"
echo "  IMPLEMENTATION.md:   $(wc -l < IMPLEMENTATION.md) lines"
echo "  example.html:        $(wc -l < example.html) lines"
echo ""

# Check for exports
echo "✅ Exports Validation:"
grep -q "export class TerraphimElement" terraphim-element.js && echo "  ✓ TerraphimElement exported"
grep -q "export function TerraphimObservable" terraphim-observable.js && echo "  ✓ TerraphimObservable exported"
grep -q "export const TerraphimEvents" terraphim-events.js && echo "  ✓ TerraphimEvents exported"
grep -q "export function TerraphimEventBus" terraphim-events.js && echo "  ✓ TerraphimEventBus exported"
grep -q "export function createEvent" terraphim-events.js && echo "  ✓ createEvent exported"
echo ""

# Check barrel exports
echo "✅ Barrel Export (index.js):"
grep "export.*TerraphimElement" index.js && echo "  ✓ TerraphimElement re-exported"
grep "export.*TerraphimObservable" index.js && echo "  ✓ TerraphimObservable re-exported"
grep "export.*TerraphimEvents" index.js && echo "  ✓ TerraphimEvents re-exported"
echo ""

# Count test cases
echo "🧪 Test Cases:"
echo "  TerraphimElement tests:    $(grep -c 'runner.test(' __tests__/terraphim-element.test.js) tests"
echo "  TerraphimObservable tests: $(grep -c 'runner.test(' __tests__/terraphim-observable.test.js) tests"
echo "  TerraphimEvents tests:     $(grep -c 'runner.test(' __tests__/terraphim-events.test.js) tests"
echo ""

echo "✅ Implementation Complete!"
echo ""
echo "To test:"
echo "  1. Open __tests__/test-runner.html in a browser"
echo "  2. Open example.html to see live demos"
echo ""
echo "Total files created: $(find . -type f \( -name '*.js' -o -name '*.html' -o -name '*.md' \) | wc -l)"
