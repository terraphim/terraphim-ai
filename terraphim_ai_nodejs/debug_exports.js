#!/usr/bin/env node

// Debug script to check what's being exported
try {
  const module = require('./index.js');
  console.log('Module loaded successfully');
  console.log('Available exports:', Object.keys(module));

  if (typeof module === 'object') {
    console.log('Module type: object');
    console.log('Module properties:');
    for (const [key, value] of Object.entries(module)) {
      console.log(`  ${key}: ${typeof value}`);
    }
  } else {
    console.log('Module type:', typeof module);
    console.log('Module value:', module);
  }
} catch (error) {
  console.error('Error loading module:', error.message);
  console.error('Stack:', error.stack);
}