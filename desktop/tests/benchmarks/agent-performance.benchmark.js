/**
 * Performance Benchmarks for Agent Operations
 * 
 * Comprehensive performance testing suite for TerraphimAgent operations
 * including agent creation, command processing, memory operations, and workflow execution.
 */

import { describe, it, beforeAll, afterAll, beforeEach, expect } from 'vitest';
import { spawn } from 'child_process';
import { WebSocket } from 'ws';
import fetch from 'node-fetch';

// Performance measurement utilities
class PerformanceMeasurement {
  constructor(operationName) {
    this.operationName = operationName;
    this.measurements = [];
    this.startTime = null;
  }

  start() {
    this.startTime = performance.now();
  }

  end() {
    if (this.startTime === null) {
      throw new Error('start() must be called before end()');
    }
    const duration = performance.now() - this.startTime;
    this.measurements.push(duration);
    this.startTime = null;
    return duration;
  }

  getStats() {
    if (this.measurements.length === 0) {
      return { count: 0, avg: 0, min: 0, max: 0, p95: 0, p99: 0 };
    }

    const sorted = [...this.measurements].sort((a, b) => a - b);
    const count = sorted.length;
    const avg = sorted.reduce((sum, val) => sum + val, 0) / count;
    const min = sorted[0];
    const max = sorted[count - 1];
    const p95 = sorted[Math.floor(count * 0.95)];
    const p99 = sorted[Math.floor(count * 0.99)];

    return { count, avg, min, max, p95, p99 };
  }

  report() {
    const stats = this.getStats();
    console.log(`\n📊 Performance Report: ${this.operationName}`);
    console.log(`   Samples: ${stats.count}`);
    console.log(`   Average: ${stats.avg.toFixed(2)}ms`);
    console.log(`   Min:     ${stats.min.toFixed(2)}ms`);
    console.log(`   Max:     ${stats.max.toFixed(2)}ms`);
    console.log(`   P95:     ${stats.p95.toFixed(2)}ms`);
    console.log(`   P99:     ${stats.p99.toFixed(2)}ms`);
    return stats;
  }
}

// Benchmark configuration
const BENCHMARK_CONFIG = {
  serverPort: 8002, // Use different port for benchmarks
  websocketUrl: 'ws://127.0.0.1:8002/ws',
  httpUrl: 'http://127.0.0.1:8002',
  timeout: 60000, // 1 minute for performance tests
  
  // Performance thresholds (in milliseconds)
  thresholds: {
    webSocketConnection: { avg: 500, p95: 1000 },
    messageProcessing: { avg: 100, p95: 200 },
    workflowStart: { avg: 2000, p95: 5000 },
    commandProcessing: { avg: 3000, p95: 10000 },
    memoryOperations: { avg: 50, p95: 100 },
    contextEnrichment: { avg: 500, p95: 1000 },
    batchOperations: { avg: 5000, p95: 15000 },
  },

  // Test scale parameters
  scale: {
    connectionLoad: 10,      // Number of concurrent connections
    messageLoad: 50,         // Number of messages per connection
    commandBatch: 20,        // Number of commands in batch test
    workflowConcurrency: 5,  // Number of concurrent workflows
  }
};

describe('Agent Performance Benchmarks', () => {
  let serverProcess = null;
  let connections = [];
  let measurements = {};

  beforeAll(async () => {
    console.log('🚀 Starting Agent Performance Benchmark Suite');
    
    // Start test server
    serverProcess = spawn('cargo', [
      'run', '--release', '--', 
      '--config', 'terraphim_server/default/ollama_llama_config.json',
      '--port', BENCHMARK_CONFIG.serverPort.toString()
    ], {
      stdio: 'pipe',
      cwd: process.cwd() + '/..'
    });

    // Wait for server to start
    let serverReady = false;
    let attempts = 0;
    const maxAttempts = 60; // 1 minute timeout

    while (!serverReady && attempts < maxAttempts) {
      try {
        const response = await fetch(`${BENCHMARK_CONFIG.httpUrl}/health`);
        if (response.ok) {
          serverReady = true;
        }
      } catch (error) {
        await new Promise(resolve => setTimeout(resolve, 1000));
        attempts++;
      }
    }

    if (!serverReady) {
      throw new Error('Benchmark server failed to start within timeout');
    }

    console.log('✅ Benchmark server started successfully');

    // Initialize measurement objects
    measurements = {
      connectionTime: new PerformanceMeasurement('WebSocket Connection'),
      messageProcessing: new PerformanceMeasurement('Message Processing'),
      workflowStart: new PerformanceMeasurement('Workflow Start'),
      commandProcessing: new PerformanceMeasurement('Command Processing'),
      memoryOperations: new PerformanceMeasurement('Memory Operations'),
      contextEnrichment: new PerformanceMeasurement('Context Enrichment'),
      batchOperations: new PerformanceMeasurement('Batch Operations'),
      throughput: new PerformanceMeasurement('Throughput Operations'),
    };
  }, BENCHMARK_CONFIG.timeout);

  afterAll(async () => {
    // Close all connections
    for (const ws of connections) {
      if (ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
    }

    // Generate performance report
    console.log('\n📈 AGENT PERFORMANCE BENCHMARK RESULTS');
    console.log('=====================================');
    
    Object.values(measurements).forEach(measurement => {
      const stats = measurement.report();
      
      // Check against thresholds
      const operationKey = measurement.operationName.toLowerCase().replace(/[^a-z]/g, '');
      const threshold = BENCHMARK_CONFIG.thresholds[operationKey];
      
      if (threshold) {
        const avgPassed = stats.avg <= threshold.avg;
        const p95Passed = stats.p95 <= threshold.p95;
        
        console.log(`   Threshold Check: ${avgPassed && p95Passed ? '✅ PASS' : '❌ FAIL'}`);
        if (!avgPassed) console.log(`     ⚠️  Average ${stats.avg.toFixed(2)}ms exceeds threshold ${threshold.avg}ms`);
        if (!p95Passed) console.log(`     ⚠️  P95 ${stats.p95.toFixed(2)}ms exceeds threshold ${threshold.p95}ms`);
      }
    });

    if (serverProcess) {
      serverProcess.kill('SIGTERM');
      
      await new Promise(resolve => {
        serverProcess.on('exit', resolve);
        setTimeout(() => {
          serverProcess.kill('SIGKILL');
          resolve();
        }, 5000);
      });
    }
  });

  beforeEach(() => {
    connections = [];
  });

  describe('WebSocket Connection Performance', () => {
    it('should establish connections within performance thresholds', async () => {
      const connectionPromises = [];

      for (let i = 0; i < BENCHMARK_CONFIG.scale.connectionLoad; i++) {
        connectionPromises.push(new Promise((resolve, reject) => {
          measurements.connectionTime.start();
          
          const ws = new WebSocket(BENCHMARK_CONFIG.websocketUrl);
          connections.push(ws);
          
          ws.on('open', () => {
            const duration = measurements.connectionTime.end();
            resolve(duration);
          });

          ws.on('error', reject);
          
          setTimeout(() => reject(new Error('Connection timeout')), 10000);
        }));
      }

      const connectionTimes = await Promise.all(connectionPromises);
      
      const avgConnectionTime = connectionTimes.reduce((sum, time) => sum + time, 0) / connectionTimes.length;
      const maxConnectionTime = Math.max(...connectionTimes);
      
      console.log(`📊 Connection Performance: ${BENCHMARK_CONFIG.scale.connectionLoad} connections`);
      console.log(`   Average: ${avgConnectionTime.toFixed(2)}ms`);
      console.log(`   Maximum: ${maxConnectionTime.toFixed(2)}ms`);
      
      // Verify all connections are established
      expect(connections.length).toBe(BENCHMARK_CONFIG.scale.connectionLoad);
      
      // Performance assertions
      expect(avgConnectionTime).toBeLessThan(BENCHMARK_CONFIG.thresholds.webSocketConnection.avg);
      expect(maxConnectionTime).toBeLessThan(BENCHMARK_CONFIG.thresholds.webSocketConnection.p95);
    });

    it('should handle rapid message sending efficiently', async () => {
      // Use first connection for message performance test
      const ws = connections[0];
      const messageCount = BENCHMARK_CONFIG.scale.messageLoad;
      let responsesReceived = 0;
      
      const responsePromise = new Promise((resolve) => {
        ws.on('message', () => {
          const duration = measurements.messageProcessing.end();
          responsesReceived++;
          
          if (responsesReceived === messageCount) {
            resolve();
          } else if (responsesReceived < messageCount) {
            // Start timing for next message
            measurements.messageProcessing.start();
          }
        });
      });

      // Start timing for first message
      measurements.messageProcessing.start();
      
      // Send rapid messages
      for (let i = 0; i < messageCount; i++) {
        const message = {
          command_type: 'heartbeat',
          session_id: null,
          workflow_id: null,
          data: {
            timestamp: new Date().toISOString(),
            sequence: i
          }
        };
        
        ws.send(JSON.stringify(message));
      }

      await responsePromise;
      
      const stats = measurements.messageProcessing.getStats();
      console.log(`📊 Message Processing: ${messageCount} messages`);
      console.log(`   Average Response Time: ${stats.avg.toFixed(2)}ms`);
      console.log(`   P95 Response Time: ${stats.p95.toFixed(2)}ms`);
      
      expect(stats.avg).toBeLessThan(BENCHMARK_CONFIG.thresholds.messageProcessing.avg);
      expect(stats.p95).toBeLessThan(BENCHMARK_CONFIG.thresholds.messageProcessing.p95);
    });
  });

  describe('Workflow Performance', () => {
    it('should start workflows within performance thresholds', async () => {
      const ws = connections[0];
      const workflowCount = BENCHMARK_CONFIG.scale.workflowConcurrency;
      let workflowsStarted = 0;
      
      const workflowPromise = new Promise((resolve) => {
        ws.on('message', (data) => {
          try {
            const message = JSON.parse(data.toString());
            if (message.response_type && message.response_type.includes('workflow')) {
              measurements.workflowStart.end();
              workflowsStarted++;
              
              if (workflowsStarted === workflowCount) {
                resolve();
              } else if (workflowsStarted < workflowCount) {
                measurements.workflowStart.start();
              }
            }
          } catch (error) {
            // Ignore parse errors
          }
        });
      });

      // Start timing for first workflow
      measurements.workflowStart.start();
      
      // Start multiple workflows
      for (let i = 0; i < workflowCount; i++) {
        const sessionId = `perf-test-${i}-${Date.now()}`;
        
        const message = {
          command_type: 'start_workflow',
          session_id: sessionId,
          workflow_id: sessionId,
          data: {
            workflowType: 'prompt-chaining',
            config: {
              steps: ['analyze', 'optimize'],
              performance_test: true,
              index: i
            },
            timestamp: new Date().toISOString()
          }
        };

        ws.send(JSON.stringify(message));
        
        // Small delay between workflow starts
        await new Promise(resolve => setTimeout(resolve, 50));
      }

      await Promise.race([
        workflowPromise,
        new Promise((_, reject) => 
          setTimeout(() => reject(new Error('Workflow start timeout')), 30000)
        )
      ]);
      
      const stats = measurements.workflowStart.getStats();
      console.log(`📊 Workflow Start Performance: ${workflowCount} workflows`);
      console.log(`   Average Start Time: ${stats.avg.toFixed(2)}ms`);
      console.log(`   P95 Start Time: ${stats.p95.toFixed(2)}ms`);
      
      expect(stats.avg).toBeLessThan(BENCHMARK_CONFIG.thresholds.workflowStart.avg);
      expect(stats.p95).toBeLessThan(BENCHMARK_CONFIG.thresholds.workflowStart.p95);
    });

    it('should handle concurrent workflow execution efficiently', async () => {
      const concurrentWorkflows = Math.min(connections.length, 5);
      const workflowPromises = [];

      for (let i = 0; i < concurrentWorkflows; i++) {
        const ws = connections[i];
        
        workflowPromises.push(new Promise((resolve, reject) => {
          let messageReceived = false;
          
          measurements.commandProcessing.start();
          
          ws.on('message', (data) => {
            if (!messageReceived) {
              messageReceived = true;
              measurements.commandProcessing.end();
              resolve();
            }
          });

          const sessionId = `concurrent-${i}-${Date.now()}`;
          const message = {
            command_type: 'start_workflow',
            session_id: sessionId,
            workflow_id: sessionId,
            data: {
              workflowType: 'parallelization',
              config: {
                concurrent: true,
                performance_test: true,
                worker_count: 3
              },
              timestamp: new Date().toISOString()
            }
          };

          ws.send(JSON.stringify(message));
          
          setTimeout(() => {
            if (!messageReceived) {
              reject(new Error(`Workflow ${i} timeout`));
            }
          }, 20000);
        }));
      }

      await Promise.all(workflowPromises);
      
      const stats = measurements.commandProcessing.getStats();
      console.log(`📊 Concurrent Workflow Performance: ${concurrentWorkflows} workflows`);
      console.log(`   Average Execution Time: ${stats.avg.toFixed(2)}ms`);
      console.log(`   P95 Execution Time: ${stats.p95.toFixed(2)}ms`);
      
      expect(stats.avg).toBeLessThan(BENCHMARK_CONFIG.thresholds.commandProcessing.avg);
      expect(stats.p95).toBeLessThan(BENCHMARK_CONFIG.thresholds.commandProcessing.p95);
    });
  });

  describe('Command Processing Performance', () => {
    it('should process different command types efficiently', async () => {
      const ws = connections[0];
      const commandTypes = ['generate', 'analyze', 'answer', 'create', 'review'];
      let commandsProcessed = 0;
      
      const commandPromise = new Promise((resolve) => {
        ws.on('message', (data) => {
          try {
            const message = JSON.parse(data.toString());
            if (message.response_type !== 'heartbeat') {
              measurements.commandProcessing.end();
              commandsProcessed++;
              
              if (commandsProcessed === commandTypes.length) {
                resolve();
              } else if (commandsProcessed < commandTypes.length) {
                measurements.commandProcessing.start();
              }
            }
          } catch (error) {
            // Ignore parse errors
          }
        });
      });

      // Start timing for first command
      measurements.commandProcessing.start();
      
      for (let i = 0; i < commandTypes.length; i++) {
        const sessionId = `cmd-perf-${i}-${Date.now()}`;
        const commandType = commandTypes[i];
        
        const message = {
          command_type: 'start_workflow',
          session_id: sessionId,
          workflow_id: sessionId,
          data: {
            workflowType: 'prompt-chaining',
            config: {
              command_type: commandType,
              text: `Performance test ${commandType} command`,
              performance_test: true
            },
            timestamp: new Date().toISOString()
          }
        };

        ws.send(JSON.stringify(message));
        
        // Delay between commands to avoid overwhelming
        await new Promise(resolve => setTimeout(resolve, 100));
      }

      await Promise.race([
        commandPromise,
        new Promise((_, reject) => 
          setTimeout(() => reject(new Error('Command processing timeout')), 30000)
        )
      ]);
      
      const stats = measurements.commandProcessing.getStats();
      console.log(`📊 Command Processing Performance: ${commandTypes.length} command types`);
      console.log(`   Average Processing Time: ${stats.avg.toFixed(2)}ms`);
      console.log(`   P95 Processing Time: ${stats.p95.toFixed(2)}ms`);
      
      expect(stats.avg).toBeLessThan(BENCHMARK_CONFIG.thresholds.commandProcessing.avg);
      expect(stats.p95).toBeLessThan(BENCHMARK_CONFIG.thresholds.commandProcessing.p95);
    });
  });

  describe('Throughput Performance', () => {
    it('should maintain high throughput under load', async () => {
      const testDuration = 10000; // 10 seconds
      const maxConnections = Math.min(connections.length, 5);
      let totalOperations = 0;
      let operationsPerSecond = 0;
      
      const startTime = Date.now();
      const endTime = startTime + testDuration;
      
      // Create load generators for each connection
      const loadGenerators = connections.slice(0, maxConnections).map((ws, index) => {
        return new Promise((resolve) => {
          let operationCount = 0;
          
          const sendMessage = () => {
            if (Date.now() >= endTime) {
              resolve(operationCount);
              return;
            }
            
            const message = {
              command_type: 'heartbeat',
              session_id: null,
              workflow_id: null,
              data: {
                timestamp: new Date().toISOString(),
                throughput_test: true,
                connection: index,
                operation: operationCount
              }
            };
            
            measurements.throughput.start();
            ws.send(JSON.stringify(message));
            operationCount++;
            
            // Continue sending messages
            setTimeout(sendMessage, 50); // 20 ops/sec per connection
          };
          
          ws.on('message', () => {
            measurements.throughput.end();
          });
          
          sendMessage();
        });
      });
      
      const operationCounts = await Promise.all(loadGenerators);
      totalOperations = operationCounts.reduce((sum, count) => sum + count, 0);
      operationsPerSecond = totalOperations / (testDuration / 1000);
      
      const stats = measurements.throughput.getStats();
      console.log(`📊 Throughput Performance: ${testDuration / 1000}s load test`);
      console.log(`   Total Operations: ${totalOperations}`);
      console.log(`   Operations/Second: ${operationsPerSecond.toFixed(2)}`);
      console.log(`   Average Latency: ${stats.avg.toFixed(2)}ms`);
      console.log(`   P95 Latency: ${stats.p95.toFixed(2)}ms`);
      
      // Throughput expectations
      expect(operationsPerSecond).toBeGreaterThan(50); // At least 50 ops/sec
      expect(stats.avg).toBeLessThan(500); // Average latency under 500ms
      expect(stats.p95).toBeLessThan(1000); // P95 latency under 1s
    });
  });

  describe('Memory and Resource Performance', () => {
    it('should efficiently manage memory operations', async () => {
      const ws = connections[0];
      const memoryOperations = 20;
      let operationsCompleted = 0;
      
      const memoryPromise = new Promise((resolve) => {
        ws.on('message', (data) => {
          try {
            const message = JSON.parse(data.toString());
            if (message.data && message.data.memory_test) {
              measurements.memoryOperations.end();
              operationsCompleted++;
              
              if (operationsCompleted === memoryOperations) {
                resolve();
              } else if (operationsCompleted < memoryOperations) {
                measurements.memoryOperations.start();
              }
            }
          } catch (error) {
            // Ignore parse errors
          }
        });
      });

      // Start timing for first memory operation
      measurements.memoryOperations.start();
      
      for (let i = 0; i < memoryOperations; i++) {
        const sessionId = `memory-${i}-${Date.now()}`;
        
        const message = {
          command_type: 'start_workflow',
          session_id: sessionId,
          workflow_id: sessionId,
          data: {
            workflowType: 'routing',
            config: {
              memory_operation: true,
              operation_type: 'context_enrichment',
              memory_test: true
            },
            timestamp: new Date().toISOString()
          }
        };

        ws.send(JSON.stringify(message));
        
        // Small delay between memory operations
        await new Promise(resolve => setTimeout(resolve, 25));
      }

      await Promise.race([
        memoryPromise,
        new Promise((_, reject) => 
          setTimeout(() => reject(new Error('Memory operations timeout')), 15000)
        )
      ]);
      
      const stats = measurements.memoryOperations.getStats();
      console.log(`📊 Memory Operations Performance: ${memoryOperations} operations`);
      console.log(`   Average Memory Op Time: ${stats.avg.toFixed(2)}ms`);
      console.log(`   P95 Memory Op Time: ${stats.p95.toFixed(2)}ms`);
      
      expect(stats.avg).toBeLessThan(BENCHMARK_CONFIG.thresholds.memoryOperations.avg);
      expect(stats.p95).toBeLessThan(BENCHMARK_CONFIG.thresholds.memoryOperations.p95);
    });

    it('should handle batch operations efficiently', async () => {
      const ws = connections[0];
      const batchSize = BENCHMARK_CONFIG.scale.commandBatch;
      
      measurements.batchOperations.start();
      
      const batchPromise = new Promise((resolve, reject) => {
        let responsesReceived = 0;
        
        ws.on('message', (data) => {
          try {
            const message = JSON.parse(data.toString());
            if (message.data && message.data.batch_test) {
              responsesReceived++;
              
              if (responsesReceived === batchSize) {
                measurements.batchOperations.end();
                resolve();
              }
            }
          } catch (error) {
            // Ignore parse errors
          }
        });
        
        setTimeout(() => reject(new Error('Batch operation timeout')), 30000);
      });

      // Send batch of operations
      for (let i = 0; i < batchSize; i++) {
        const sessionId = `batch-${i}-${Date.now()}`;
        
        const message = {
          command_type: 'start_workflow',
          session_id: sessionId,
          workflow_id: sessionId,
          data: {
            workflowType: 'parallelization',
            config: {
              batch_operation: true,
              batch_index: i,
              batch_size: batchSize,
              batch_test: true
            },
            timestamp: new Date().toISOString()
          }
        };

        ws.send(JSON.stringify(message));
      }

      await batchPromise;
      
      const stats = measurements.batchOperations.getStats();
      console.log(`📊 Batch Operations Performance: ${batchSize} operations`);
      console.log(`   Total Batch Time: ${stats.max.toFixed(2)}ms`);
      console.log(`   Average Per Operation: ${(stats.max / batchSize).toFixed(2)}ms`);
      
      expect(stats.max).toBeLessThan(BENCHMARK_CONFIG.thresholds.batchOperations.avg);
    });
  });

  describe('Error Handling Performance', () => {
    it('should handle errors efficiently without performance degradation', async () => {
      const ws = connections[0];
      const errorMessages = [
        { command_type: '', session_id: 'test' }, // Invalid command type
        { command_type: 'invalid_command', session_id: 'test' }, // Unknown command
        {}, // Empty message
        { command_type: 'start_workflow' }, // Missing session_id
      ];
      
      let errorsHandled = 0;
      
      const errorPromise = new Promise((resolve) => {
        const handleMessage = () => {
          errorsHandled++;
          if (errorsHandled === errorMessages.length) {
            resolve();
          }
        };
        
        ws.on('message', handleMessage);
        ws.on('error', handleMessage);
        
        setTimeout(resolve, 5000); // Max 5 seconds for error handling
      });

      const startTime = performance.now();
      
      for (const errorMessage of errorMessages) {
        ws.send(JSON.stringify(errorMessage));
        await new Promise(resolve => setTimeout(resolve, 50));
      }

      await errorPromise;
      
      const totalTime = performance.now() - startTime;
      const avgErrorHandlingTime = totalTime / errorMessages.length;
      
      console.log(`📊 Error Handling Performance: ${errorMessages.length} error cases`);
      console.log(`   Total Error Handling Time: ${totalTime.toFixed(2)}ms`);
      console.log(`   Average Per Error: ${avgErrorHandlingTime.toFixed(2)}ms`);
      
      // Error handling should be fast
      expect(avgErrorHandlingTime).toBeLessThan(100);
      expect(totalTime).toBeLessThan(1000);
      
      // Connection should still be alive after errors
      expect(ws.readyState).toBe(WebSocket.OPEN);
    });
  });
});