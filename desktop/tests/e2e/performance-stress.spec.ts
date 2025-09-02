/**
 * Performance and Stress Tests for Context Management System
 *
 * These tests validate system performance under various load conditions,
 * memory usage patterns, and stress scenarios to ensure the context management
 * system performs well in real-world usage conditions.
 */

import { test, expect } from '@playwright/test';
import { ContextTestHelpers } from '../helpers/context-helpers';
import type { Page } from '@playwright/test';

// Performance test configuration
const STRESS_TEST_TIMEOUT = 180000; // 3 minutes for stress tests
const PERFORMANCE_TEST_TIMEOUT = 120000; // 2 minutes for performance tests
const LARGE_DATASET_SIZE = 50;
const CONCURRENT_OPERATIONS = 10;

interface PerformanceMetrics {
  startTime: number;
  endTime: number;
  duration: number;
  memoryUsage?: number;
  networkRequests?: number;
}

// Performance test tags
test.describe('Context Management Performance Tests @performance', () => {
  let helpers: ContextTestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new ContextTestHelpers(page);

    // Navigate to application
    await page.goto('/');
    await helpers.waitForApplicationReady();
    await helpers.clearApplicationState();

    // Enable performance monitoring
    await page.evaluate(() => {
      (window as any).performanceMetrics = {
        startTime: Date.now(),
        networkRequests: 0,
        apiCalls: []
      };
    });
  });

  test('Conversation creation performance benchmark', async ({ page }) => {
    test.setTimeout(PERFORMANCE_TEST_TIMEOUT);

    const metrics: PerformanceMetrics[] = [];

    await test.step('Measure single conversation creation time', async () => {
      for (let i = 0; i < 10; i++) {
        const startTime = Date.now();

        const conversationId = await helpers.createConversation({
          title: `Performance Test ${i + 1}`,
          role: 'Engineer'
        });

        const endTime = Date.now();

        expect(conversationId).toBeDefined();

        metrics.push({
          startTime,
          endTime,
          duration: endTime - startTime
        });

        // Clean up for next iteration
        await helpers.deleteConversation(conversationId);
      }

      // Analyze performance metrics
      const averageTime = metrics.reduce((sum, m) => sum + m.duration, 0) / metrics.length;
      const maxTime = Math.max(...metrics.map(m => m.duration));
      const minTime = Math.min(...metrics.map(m => m.duration));

      console.log(`Conversation creation performance:
        Average: ${averageTime.toFixed(2)}ms
        Min: ${minTime}ms
        Max: ${maxTime}ms`);

      // Performance assertions
      expect(averageTime).toBeLessThan(2000); // Average under 2 seconds
      expect(maxTime).toBeLessThan(5000); // Max under 5 seconds
    });

    await test.step('Measure concurrent conversation creation', async () => {
      const startTime = Date.now();

      const creationPromises = Array.from({ length: CONCURRENT_OPERATIONS }, (_, i) =>
        helpers.createConversation({
          title: `Concurrent Test ${i + 1}`,
          role: 'Engineer'
        })
      );

      const conversationIds = await Promise.all(creationPromises);
      const endTime = Date.now();

      expect(conversationIds).toHaveLength(CONCURRENT_OPERATIONS);
      expect(conversationIds.every(id => id.length > 0)).toBe(true);

      const totalTime = endTime - startTime;
      const averagePerConversation = totalTime / CONCURRENT_OPERATIONS;

      console.log(`Concurrent creation performance:
        Total time: ${totalTime}ms
        Average per conversation: ${averagePerConversation.toFixed(2)}ms`);

      // Concurrent operations should be faster than sequential
      expect(averagePerConversation).toBeLessThan(1000);

      // Cleanup
      const deletePromises = conversationIds.map(id => helpers.deleteConversation(id));
      await Promise.all(deletePromises);
    });
  });

  test('Context management performance with large datasets', async ({ page }) => {
    test.setTimeout(PERFORMANCE_TEST_TIMEOUT);

    await test.step('Setup conversation for large dataset test', async () => {
      const conversationId = await helpers.createConversation({
        title: 'Large Dataset Performance Test',
        role: 'Engineer'
      });

      expect(conversationId).toBeDefined();
    });

    await test.step('Measure context addition performance', async () => {
      const additionMetrics: PerformanceMetrics[] = [];

      for (let i = 0; i < LARGE_DATASET_SIZE; i++) {
        const startTime = Date.now();

        await helpers.addManualContext({
          title: `Performance Context Item ${i + 1}`,
          content: `Large content block for performance testing. This is item number ${i + 1} of ${LARGE_DATASET_SIZE}. `.repeat(50),
          contextType: 'document'
        });

        const endTime = Date.now();

        additionMetrics.push({
          startTime,
          endTime,
          duration: endTime - startTime
        });

        // Check for performance degradation over time
        if (i > 0 && i % 10 === 0) {
          const recent10 = additionMetrics.slice(-10);
          const averageRecent = recent10.reduce((sum, m) => sum + m.duration, 0) / recent10.length;

          const first10 = additionMetrics.slice(0, Math.min(10, additionMetrics.length));
          const averageFirst = first10.reduce((sum, m) => sum + m.duration, 0) / first10.length;

          // Performance should not degrade significantly
          expect(averageRecent).toBeLessThan(averageFirst * 3);
        }
      }

      // Verify all contexts were added
      const contextCount = await page.locator('[data-testid^="context-item-"]').count();
      const expectedCount = Math.min(LARGE_DATASET_SIZE, 20); // Assuming max limit is 20
      expect(contextCount).toBe(expectedCount);

      // Analyze performance
      const averageAddTime = additionMetrics.reduce((sum, m) => sum + m.duration, 0) / additionMetrics.length;
      console.log(`Context addition performance with ${LARGE_DATASET_SIZE} items:
        Average addition time: ${averageAddTime.toFixed(2)}ms`);

      expect(averageAddTime).toBeLessThan(3000); // Average under 3 seconds
    });

    await test.step('Measure context panel rendering performance', async () => {
      const startTime = Date.now();

      // Scroll through context panel to trigger all rendering
      const contextPanel = page.locator('[data-testid="context-panel"]');
      await contextPanel.scrollIntoViewIfNeeded();

      // Scroll to bottom and back to top
      await page.keyboard.press('End');
      await page.waitForTimeout(100);
      await page.keyboard.press('Home');

      const endTime = Date.now();
      const renderTime = endTime - startTime;

      console.log(`Context panel rendering time: ${renderTime}ms`);
      expect(renderTime).toBeLessThan(2000); // Should render quickly
    });

    await test.step('Measure search performance within large context', async () => {
      // Open context search
      await page.click('[data-testid="context-search-toggle"]');

      const startTime = Date.now();
      await page.fill('[data-testid="context-search-input"]', 'Performance Context Item 25');

      // Wait for search results
      await expect(page.locator('[data-testid="context-search-results"]')).toBeVisible();

      const endTime = Date.now();
      const searchTime = endTime - startTime;

      console.log(`Context search time: ${searchTime}ms`);
      expect(searchTime).toBeLessThan(1000); // Search should be fast

      // Verify search found the right item
      const searchResults = await page.locator('[data-testid^="context-search-result-"]').count();
      expect(searchResults).toBeGreaterThan(0);
    });
  });

  test('Message handling performance with conversation history', async ({ page }) => {
    test.setTimeout(PERFORMANCE_TEST_TIMEOUT);

    await test.step('Setup conversation with context', async () => {
      const conversationId = await helpers.createConversation({
        title: 'Message Performance Test',
        role: 'Engineer'
      });

      // Add some context for realistic testing
      for (let i = 0; i < 5; i++) {
        await helpers.addManualContext({
          title: `Context ${i + 1}`,
          content: `Context content for message performance testing ${i + 1}`,
          contextType: 'document'
        });
      }
    });

    await test.step('Measure message addition performance', async () => {
      const messageMetrics: PerformanceMetrics[] = [];

      for (let i = 0; i < 30; i++) {
        const role = i % 2 === 0 ? 'user' : 'assistant';
        const startTime = Date.now();

        await helpers.addMessage({
          content: `Performance test message ${i + 1}. This is a ${role} message for testing message handling performance.`,
          role: role as 'user' | 'assistant'
        });

        const endTime = Date.now();

        messageMetrics.push({
          startTime,
          endTime,
          duration: endTime - startTime
        });
      }

      // Verify all messages added
      const messageCount = await page.locator('[data-testid^="message-"]').count();
      expect(messageCount).toBe(30);

      // Analyze message performance
      const averageTime = messageMetrics.reduce((sum, m) => sum + m.duration, 0) / messageMetrics.length;
      console.log(`Message addition performance:
        Average time: ${averageTime.toFixed(2)}ms`);

      expect(averageTime).toBeLessThan(1500); // Average under 1.5 seconds
    });

    await test.step('Measure message thread scrolling performance', async () => {
      const messageThread = page.locator('[data-testid="message-thread"]');

      const startTime = Date.now();

      // Scroll through entire message history
      await page.keyboard.press('Control+End'); // Scroll to bottom
      await page.waitForTimeout(100);
      await page.keyboard.press('Control+Home'); // Scroll to top
      await page.waitForTimeout(100);

      // Scroll to middle
      for (let i = 0; i < 10; i++) {
        await page.keyboard.press('PageDown');
        await page.waitForTimeout(50);
      }

      const endTime = Date.now();
      const scrollTime = endTime - startTime;

      console.log(`Message thread scrolling time: ${scrollTime}ms`);
      expect(scrollTime).toBeLessThan(3000); // Scrolling should be smooth
    });
  });

  test('Memory usage and cleanup performance', async ({ page, browserName }) => {
    test.setTimeout(PERFORMANCE_TEST_TIMEOUT);

    // Skip in Firefox due to memory measurement limitations
    test.skip(browserName === 'firefox', 'Memory measurements not reliable in Firefox');

    await test.step('Measure baseline memory usage', async () => {
      const baselineMemory = await page.evaluate(() => {
        if ('memory' in performance) {
          const memory = (performance as any).memory;
          return {
            usedJSHeapSize: memory.usedJSHeapSize,
            totalJSHeapSize: memory.totalJSHeapSize
          };
        }
        return null;
      });

      console.log('Baseline memory:', baselineMemory);
    });

    await test.step('Create and cleanup multiple conversations', async () => {
      const conversationIds: string[] = [];

      // Create many conversations with content
      for (let i = 0; i < 20; i++) {
        const conversationId = await helpers.createConversation({
          title: `Memory Test Conversation ${i + 1}`,
          role: 'Engineer'
        });

        conversationIds.push(conversationId);

        // Add context and messages to each conversation
        await helpers.addManualContext({
          title: `Context for conversation ${i + 1}`,
          content: 'Large content block for memory testing. '.repeat(100),
          contextType: 'document'
        });

        await helpers.addMessage({
          content: `Message for conversation ${i + 1}`,
          role: 'user'
        });
      }

      // Measure memory after creation
      const afterCreationMemory = await page.evaluate(() => {
        if ('memory' in performance) {
          const memory = (performance as any).memory;
          return {
            usedJSHeapSize: memory.usedJSHeapSize,
            totalJSHeapSize: memory.totalJSHeapSize
          };
        }
        return null;
      });

      console.log('Memory after creation:', afterCreationMemory);

      // Delete all conversations
      for (const conversationId of conversationIds) {
        await helpers.deleteConversation(conversationId);
      }

      // Force garbage collection if available
      await page.evaluate(() => {
        if ('gc' in window) {
          (window as any).gc();
        }
      });

      // Wait a bit for cleanup
      await page.waitForTimeout(2000);

      // Measure memory after cleanup
      const afterCleanupMemory = await page.evaluate(() => {
        if ('memory' in performance) {
          const memory = (performance as any).memory;
          return {
            usedJSHeapSize: memory.usedJSHeapSize,
            totalJSHeapSize: memory.totalJSHeapSize
          };
        }
        return null;
      });

      console.log('Memory after cleanup:', afterCleanupMemory);

      // Memory should be released (allowing for some overhead)
      if (afterCreationMemory && afterCleanupMemory) {
        const memoryIncrease = afterCreationMemory.usedJSHeapSize;
        const memoryAfterCleanup = afterCleanupMemory.usedJSHeapSize;
        const releaseRatio = (memoryIncrease - memoryAfterCleanup) / memoryIncrease;

        expect(releaseRatio).toBeGreaterThan(0.5); // At least 50% memory should be released
      }
    });
  });

  test('Network performance and error resilience', async ({ page }) => {
    test.setTimeout(PERFORMANCE_TEST_TIMEOUT);

    await test.step('Measure API call performance', async () => {
      // Monitor network requests
      const apiCalls: Array<{ url: string, duration: number, status: number }> = [];

      page.on('response', async (response) => {
        if (response.url().includes('/api/')) {
          const request = response.request();
          const timing = response.request().timing();
          if (timing) {
            apiCalls.push({
              url: response.url(),
              duration: timing.responseEnd - timing.requestStart,
              status: response.status()
            });
          }
        }
      });

      // Perform various operations
      const conversationId = await helpers.createConversation({
        title: 'Network Performance Test',
        role: 'Engineer'
      });

      await helpers.addManualContext({
        title: 'Network Test Context',
        content: 'Content for network performance testing',
        contextType: 'user_input'
      });

      await helpers.addMessage({
        content: 'Network performance test message',
        role: 'user'
      });

      // Analyze network performance
      const averageApiTime = apiCalls.reduce((sum, call) => sum + call.duration, 0) / apiCalls.length;
      const maxApiTime = Math.max(...apiCalls.map(call => call.duration));

      console.log(`API Performance:
        Average response time: ${averageApiTime.toFixed(2)}ms
        Max response time: ${maxApiTime}ms
        Total API calls: ${apiCalls.length}`);

      expect(averageApiTime).toBeLessThan(2000); // Average under 2 seconds
      expect(maxApiTime).toBeLessThan(5000); // Max under 5 seconds

      // All API calls should be successful
      const failedCalls = apiCalls.filter(call => call.status >= 400);
      expect(failedCalls).toHaveLength(0);
    });

    await test.step('Test performance under simulated network latency', async () => {
      // Simulate slow network
      await page.route('**/api/**', async (route) => {
        await new Promise(resolve => setTimeout(resolve, 500)); // 500ms delay
        route.continue();
      });

      const startTime = Date.now();

      const conversationId = await helpers.createConversation({
        title: 'Latency Test Conversation',
        role: 'Engineer'
      });

      const endTime = Date.now();
      const totalTime = endTime - startTime;

      console.log(`Operation time with 500ms latency: ${totalTime}ms`);

      // Should handle latency gracefully
      expect(totalTime).toBeGreaterThan(500); // At least the delay
      expect(totalTime).toBeLessThan(3000); // But still reasonable

      // Remove latency simulation
      await page.unroute('**/api/**');
    });
  });
});

// Stress test scenarios
test.describe('Context Management Stress Tests @performance', () => {
  let helpers: ContextTestHelpers;

  test.beforeEach(async ({ page }) => {
    helpers = new ContextTestHelpers(page);
    await page.goto('/');
    await helpers.waitForApplicationReady();
    await helpers.clearApplicationState();
  });

  test('Stress test: Maximum concurrent operations', async ({ page }) => {
    test.setTimeout(STRESS_TEST_TIMEOUT);

    await test.step('Create base conversation', async () => {
      const conversationId = await helpers.createConversation({
        title: 'Stress Test Conversation',
        role: 'Engineer'
      });

      expect(conversationId).toBeDefined();
    });

    await test.step('Perform maximum concurrent context operations', async () => {
      // Create arrays of concurrent operations
      const contextAdditions = Array.from({ length: 25 }, (_, i) =>
        helpers.addManualContext({
          title: `Stress Context ${i + 1}`,
          content: `Stress test content ${i + 1} `.repeat(50),
          contextType: 'document'
        }).catch(error => {
          console.log(`Context addition ${i + 1} failed:`, error);
          return null;
        })
      );

      const messageAdditions = Array.from({ length: 15 }, (_, i) =>
        helpers.addMessage({
          content: `Stress test message ${i + 1}`,
          role: i % 2 === 0 ? 'user' : 'assistant'
        }).catch(error => {
          console.log(`Message addition ${i + 1} failed:`, error);
          return null;
        })
      );

      // Execute all operations concurrently
      const startTime = Date.now();

      const [contextResults, messageResults] = await Promise.all([
        Promise.all(contextAdditions),
        Promise.all(messageAdditions)
      ]);

      const endTime = Date.now();
      const totalTime = endTime - startTime;

      console.log(`Concurrent stress test completed in ${totalTime}ms`);

      // Check how many operations succeeded
      const successfulContexts = contextResults.filter(result => result !== null).length;
      const successfulMessages = messageResults.filter(result => result !== null).length;

      console.log(`Successful operations:
        Contexts: ${successfulContexts}/25
        Messages: ${successfulMessages}/15`);

      // At least 80% of operations should succeed
      expect(successfulContexts / 25).toBeGreaterThan(0.8);
      expect(successfulMessages / 15).toBeGreaterThan(0.8);

      // Verify final state
      const finalContextCount = await page.locator('[data-testid^="context-item-"]').count();
      const finalMessageCount = await page.locator('[data-testid^="message-"]').count();

      expect(finalContextCount).toBeGreaterThan(0);
      expect(finalMessageCount).toBeGreaterThan(0);
    });
  });

  test('Stress test: Rapid conversation switching', async ({ page }) => {
    test.setTimeout(STRESS_TEST_TIMEOUT);

    let conversationIds: string[] = [];

    await test.step('Create multiple conversations', async () => {
      for (let i = 0; i < 15; i++) {
        const conversationId = await helpers.createConversation({
          title: `Switch Test Conversation ${i + 1}`,
          role: 'Engineer'
        });
        conversationIds.push(conversationId);

        // Add unique content to each conversation
        await helpers.addManualContext({
          title: `Unique Context ${i + 1}`,
          content: `Content unique to conversation ${i + 1}`,
          contextType: 'user_input'
        });

        await helpers.addMessage({
          content: `Message for conversation ${i + 1}`,
          role: 'user'
        });
      }
    });

    await test.step('Perform rapid conversation switching', async () => {
      const switchTimes: number[] = [];

      for (let round = 0; round < 50; round++) {
        const randomIndex = Math.floor(Math.random() * conversationIds.length);
        const targetConversationId = conversationIds[randomIndex];

        const startTime = Date.now();

        await helpers.selectConversation(targetConversationId);

        // Verify switch completed
        await expect(page.locator('[data-testid="current-conversation-title"]'))
          .toHaveText(`Switch Test Conversation ${randomIndex + 1}`);

        const endTime = Date.now();
        const switchTime = endTime - startTime;
        switchTimes.push(switchTime);

        // Brief pause to avoid overwhelming the system
        await page.waitForTimeout(50);
      }

      // Analyze switching performance
      const averageSwitchTime = switchTimes.reduce((sum, time) => sum + time, 0) / switchTimes.length;
      const maxSwitchTime = Math.max(...switchTimes);

      console.log(`Conversation switching performance:
        Average: ${averageSwitchTime.toFixed(2)}ms
        Max: ${maxSwitchTime}ms
        Switches: ${switchTimes.length}`);

      // Performance should remain reasonable
      expect(averageSwitchTime).toBeLessThan(1000); // Average under 1 second
      expect(maxSwitchTime).toBeLessThan(3000); // Max under 3 seconds

      // No switches should fail completely
      expect(switchTimes.every(time => time > 0)).toBe(true);
    });
  });

  test('Stress test: Large content handling', async ({ page }) => {
    test.setTimeout(STRESS_TEST_TIMEOUT);

    await test.step('Create conversation for large content test', async () => {
      const conversationId = await helpers.createConversation({
        title: 'Large Content Stress Test',
        role: 'Engineer'
      });

      expect(conversationId).toBeDefined();
    });

    await test.step('Add extremely large context items', async () => {
      const largeContentSizes = [10000, 25000, 50000, 75000, 100000]; // Character counts

      for (const [index, size] of largeContentSizes.entries()) {
        const largeContent = `Large content block ${index + 1}. `.repeat(size / 30);

        const startTime = Date.now();

        try {
          await helpers.addManualContext({
            title: `Large Content ${index + 1} (${size} chars)`,
            content: largeContent,
            contextType: 'document'
          });

          const endTime = Date.now();
          const addTime = endTime - startTime;

          console.log(`Added large content ${index + 1} (${size} chars) in ${addTime}ms`);

          // Large content should still be added in reasonable time
          expect(addTime).toBeLessThan(10000); // Under 10 seconds

        } catch (error) {
          console.log(`Large content ${index + 1} failed (expected for very large content):`, error);

          // Verify appropriate error handling
          await expect(page.locator('[data-testid="content-too-large-error"]')).toBeVisible();
        }
      }

      // Verify UI remains responsive
      await expect(page.locator('[data-testid="context-panel"]')).toBeVisible();

      const contextCount = await page.locator('[data-testid^="context-item-"]').count();
      expect(contextCount).toBeGreaterThan(0); // At least some content should be added
    });
  });
});
