import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    name: 'Agent Performance Benchmarks',
    include: ['tests/benchmarks/**/*.benchmark.{js,ts}'],
    exclude: ['tests/unit/**', 'tests/integration/**', 'tests/e2e/**'],
    environment: 'node',
    timeout: 120000, // 2 minutes per test for performance tests
    testTimeout: 120000,
    hookTimeout: 30000,
    teardownTimeout: 30000,
    globals: true,

    // Performance test specific settings
    reporters: ['verbose', 'json'],
    outputFile: {
      json: './test-results/benchmark-results.json'
    },

    // Disable parallel execution for benchmarks to get consistent results
    pool: 'forks',
    poolOptions: {
      forks: {
        singleFork: true
      }
    },

    // Ensure we have enough resources for performance tests
    threads: false,
    isolate: true,

    // Coverage is not needed for benchmarks
    coverage: {
      enabled: false
    }
  },

  // Ensure proper module resolution for benchmarks
  resolve: {
    alias: {
      '@': './src'
    }
  }
});
