#!/bin/bash
# Example shell script demonstrating package manager commands

set -e

echo "Setting up project..."

# Install dependencies
npm install

# Run tests
yarn test

# Build the project
pnpm build

# Start development server
npm run dev
