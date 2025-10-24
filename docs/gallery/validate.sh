#!/bin/bash

# Validation script for Terraphim Gallery Phase 1

echo "Validating Terraphim Gallery Phase 1 Implementation..."
echo ""

errors=0

# Function to check if file exists
check_file() {
  if [ -f "$1" ]; then
    echo "✓ $1"
  else
    echo "✗ $1 (MISSING)"
    ((errors++))
  fi
}

# CSS Files
echo "Checking CSS files..."
check_file "styles/theme-light.css"
check_file "styles/theme-dark.css"
check_file "styles/gallery.css"
check_file "styles/responsive.css"
echo ""

# JavaScript Utilities
echo "Checking JavaScript utilities..."
check_file "scripts/router.js"
check_file "scripts/search.js"
echo ""

# Data Files
echo "Checking data files..."
check_file "data/components.json"
check_file "data/nav-structure.json"
echo ""

# Web Components
echo "Checking Web Components..."
check_file "../../components/gallery/theme-toggle.js"
check_file "../../components/gallery/nav-item.js"
check_file "../../components/gallery/nav-category.js"
check_file "../../components/gallery/gallery-header.js"
check_file "../../components/gallery/gallery-sidebar.js"
check_file "../../components/gallery/gallery-main.js"
check_file "../../components/gallery/terraphim-gallery.js"
echo ""

# HTML Entry Point
echo "Checking HTML entry point..."
check_file "index.html"
echo ""

# Documentation
echo "Checking documentation..."
check_file "README.md"
check_file "IMPLEMENTATION_SUMMARY.md"
echo ""

# Summary
echo "================================"
if [ $errors -eq 0 ]; then
  echo "✓ All files validated successfully!"
  echo "  Total files: 18"
  echo ""
  echo "To run the gallery:"
  echo "  cd docs/gallery"
  echo "  python3 -m http.server 8000"
  echo "  open http://localhost:8000"
  exit 0
else
  echo "✗ Validation failed with $errors error(s)"
  exit 1
fi
