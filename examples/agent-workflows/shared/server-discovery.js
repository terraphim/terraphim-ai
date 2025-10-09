/**
 * AI Agent Workflows - Server Discovery Service
 * Auto-discovers and validates terraphim servers
 */

class ServerDiscoveryService {
  constructor(options = {}) {
    this.commonPorts = options.ports || [3000, 8000, 8080, 8888, 9000];
    this.hostnames = options.hostnames || ['localhost', '127.0.0.1'];
    this.protocols = options.protocols || ['http', 'https'];
    this.discoveryTimeout = options.timeout || 5000;
    this.discoveredServers = [];
    this.isScanning = false;
  }

  // Main discovery method
  async discoverServers(onProgress = null) {
    this.isScanning = true;
    this.discoveredServers = [];
    
    const totalCombinations = this.protocols.length * this.hostnames.length * this.commonPorts.length;
    let completed = 0;

    try {
      const scanPromises = [];
      
      for (const protocol of this.protocols) {
        for (const hostname of this.hostnames) {
          for (const port of this.commonPorts) {
            const serverUrl = `${protocol}://${hostname}:${port}`;
            
            scanPromises.push(
              this.testServer(serverUrl)
                .then(result => {
                  completed++;
                  if (onProgress) {
                    onProgress({
                      completed,
                      total: totalCombinations,
                      percentage: Math.round((completed / totalCombinations) * 100),
                      currentUrl: serverUrl,
                      found: result ? 1 : 0
                    });
                  }
                  return result;
                })
                .catch(error => {
                  completed++;
                  if (onProgress) {
                    onProgress({
                      completed,
                      total: totalCombinations,
                      percentage: Math.round((completed / totalCombinations) * 100),
                      currentUrl: serverUrl,
                      error: error.message
                    });
                  }
                  return null;
                })
            );
          }
        }
      }

      const results = await Promise.all(scanPromises);
      this.discoveredServers = results.filter(server => server !== null);
      
      // Sort by response time (fastest first)
      this.discoveredServers.sort((a, b) => a.responseTime - b.responseTime);
      
      return this.discoveredServers;
    } finally {
      this.isScanning = false;
    }
  }

  // Test individual server
  async testServer(baseUrl) {
    const startTime = Date.now();
    
    try {
      // Test health endpoint
      const healthResponse = await this.fetchWithTimeout(`${baseUrl}/health`, {
        method: 'GET',
        headers: { 'Accept': 'application/json' }
      }, this.discoveryTimeout);

      if (!healthResponse.ok) {
        return null;
      }

      const responseTime = Date.now() - startTime;
      
      // Try to get server info
      let serverInfo = { version: 'unknown', capabilities: [] };
      try {
        const healthData = await healthResponse.json();
        serverInfo = {
          version: healthData.version || 'unknown',
          capabilities: healthData.capabilities || [],
          status: healthData.status || 'ok'
        };
      } catch (e) {
        // Health endpoint might not return JSON, that's ok
      }

      // Test WebSocket availability
      const wsUrl = this.getWebSocketUrl(baseUrl);
      const wsAvailable = await this.testWebSocket(wsUrl);

      // Test workflow endpoints
      const workflowEndpoints = await this.testWorkflowEndpoints(baseUrl);

      return {
        url: baseUrl,
        wsUrl: wsUrl,
        responseTime,
        health: 'ok',
        version: serverInfo.version,
        capabilities: serverInfo.capabilities,
        wsAvailable,
        workflowEndpoints,
        discoveredAt: new Date().toISOString()
      };

    } catch (error) {
      return null;
    }
  }

  // Test WebSocket connection
  async testWebSocket(wsUrl) {
    return new Promise((resolve) => {
      try {
        const ws = new WebSocket(wsUrl);
        const timeout = setTimeout(() => {
          ws.close();
          resolve(false);
        }, 3000);

        ws.onopen = () => {
          clearTimeout(timeout);
          ws.close();
          resolve(true);
        };

        ws.onerror = () => {
          clearTimeout(timeout);
          resolve(false);
        };

        ws.onclose = () => {
          clearTimeout(timeout);
          resolve(false);
        };
      } catch (error) {
        resolve(false);
      }
    });
  }

  // Test workflow endpoints
  async testWorkflowEndpoints(baseUrl) {
    const endpoints = [
      '/workflows/prompt-chain',
      '/workflows/route', 
      '/workflows/parallel',
      '/workflows/orchestrate',
      '/workflows/optimize'
    ];

    const availableEndpoints = [];
    
    for (const endpoint of endpoints) {
      try {
        // Use OPTIONS request to test endpoint without executing
        const response = await this.fetchWithTimeout(`${baseUrl}${endpoint}`, {
          method: 'OPTIONS'
        }, 2000);
        
        if (response.ok || response.status === 405) { // 405 = Method Not Allowed is OK
          availableEndpoints.push(endpoint);
        }
      } catch (error) {
        // Endpoint not available
      }
    }

    return availableEndpoints;
  }

  // Get WebSocket URL from HTTP URL
  getWebSocketUrl(httpUrl) {
    const url = new URL(httpUrl);
    const protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${url.host}/ws`;
  }

  // Fetch with timeout
  async fetchWithTimeout(url, options = {}, timeout = 5000) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout);
    
    try {
      const response = await fetch(url, {
        ...options,
        signal: controller.signal
      });
      clearTimeout(timeoutId);
      return response;
    } catch (error) {
      clearTimeout(timeoutId);
      throw error;
    }
  }

  // Quick test for a specific server
  async quickTest(serverUrl) {
    return await this.testServer(serverUrl);
  }

  // Get cached discovered servers
  getDiscoveredServers() {
    return [...this.discoveredServers];
  }

  // Get the best available server
  getBestServer() {
    if (this.discoveredServers.length === 0) {
      return null;
    }
    
    // Return server with best health and lowest response time
    return this.discoveredServers.find(server => 
      server.health === 'ok' && server.workflowEndpoints.length > 0
    ) || this.discoveredServers[0];
  }

  // Check if scanning is in progress
  isScanning() {
    return this.isScanning;
  }

  // Get recommended server based on criteria
  getRecommendedServer(criteria = {}) {
    const servers = this.getDiscoveredServers();
    
    if (servers.length === 0) {
      return null;
    }

    let filtered = servers.filter(server => server.health === 'ok');
    
    if (criteria.requireWebSocket) {
      filtered = filtered.filter(server => server.wsAvailable);
    }
    
    if (criteria.requiredEndpoints) {
      filtered = filtered.filter(server => 
        criteria.requiredEndpoints.every(endpoint => 
          server.workflowEndpoints.includes(endpoint)
        )
      );
    }

    if (criteria.maxResponseTime) {
      filtered = filtered.filter(server => 
        server.responseTime <= criteria.maxResponseTime
      );
    }

    // Sort by response time and return best
    filtered.sort((a, b) => a.responseTime - b.responseTime);
    return filtered[0] || null;
  }

  // Export discovery results
  exportDiscoveryResults() {
    return {
      discoveredAt: new Date().toISOString(),
      servers: this.discoveredServers,
      scanConfig: {
        ports: this.commonPorts,
        hostnames: this.hostnames,
        protocols: this.protocols,
        timeout: this.discoveryTimeout
      }
    };
  }

  // Import and validate discovery results
  importDiscoveryResults(data) {
    if (!data || !data.servers || !Array.isArray(data.servers)) {
      throw new Error('Invalid discovery results format');
    }

    // Validate each server entry
    const validServers = data.servers.filter(server => 
      server.url && 
      server.health && 
      typeof server.responseTime === 'number'
    );

    this.discoveredServers = validServers;
    return validServers.length;
  }
}

// Export for use in examples
window.ServerDiscoveryService = ServerDiscoveryService;