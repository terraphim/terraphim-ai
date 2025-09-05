import { createServer } from 'http';
import { URL } from 'url';

// Simple test server that mimics the Terraphim API
export async function startTestServer(config: any): Promise<any> {
  const conversations = new Map();
  const contexts = new Map();
  let nextConversationId = 1;
  let nextContextId = 1;

  const server = createServer((req, res) => {
    const url = new URL(req.url!, `http://${req.headers.host}`);
    const method = req.method;
    const pathname = url.pathname;

    // Enable CORS
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');

    if (method === 'OPTIONS') {
      res.writeHead(200);
      res.end();
      return;
    }

    // Parse request body for POST requests
    let body = '';
    req.on('data', chunk => {
      body += chunk.toString();
    });

    req.on('end', () => {
      let requestData = {};
      if (body) {
        try {
          requestData = JSON.parse(body);
        } catch (e) {
          // Ignore parsing errors
        }
      }

      res.setHeader('Content-Type', 'application/json');

      try {
        // Route handling
        if (pathname === '/conversations') {
          if (method === 'GET') {
            // List conversations
            const conversationList = Array.from(conversations.values());
            res.writeHead(200);
            res.end(JSON.stringify({
              status: 'Success',
              conversations: conversationList,
            }));
            return;
          } else if (method === 'POST') {
            // Create conversation
            const conversationId = `conv-${nextConversationId++}`;
            const conversation = {
              id: conversationId,
              title: (requestData as any).title || 'Untitled Conversation',
              role: (requestData as any).role || 'default',
              created_at: new Date().toISOString(),
              global_context: [],
              messages: [],
            };
            conversations.set(conversationId, conversation);
            res.writeHead(200);
            res.end(JSON.stringify({
              status: 'Success',
              conversation_id: conversationId,
            }));
            return;
          }
        }

        // Get specific conversation
        const conversationMatch = pathname.match(/^\/conversations\/([^\/]+)$/);
        if (conversationMatch && method === 'GET') {
          const conversationId = conversationMatch[1];
          const conversation = conversations.get(conversationId);
          if (conversation) {
            res.writeHead(200);
            res.end(JSON.stringify({
              status: 'Success',
              conversation,
            }));
          } else {
            res.writeHead(404);
            res.end(JSON.stringify({
              status: 'Error',
              error: 'Conversation not found',
            }));
          }
          return;
        }

        // Add context to conversation
        const contextMatch = pathname.match(/^\/conversations\/([^\/]+)\/context$/);
        if (contextMatch && method === 'POST') {
          const conversationId = contextMatch[1];
          const conversation = conversations.get(conversationId);

          if (conversation) {
            const contextId = `ctx-${nextContextId++}`;
            const contextItem = {
              id: contextId,
              context_type: (requestData as any).context_type || 'document',
              title: (requestData as any).title || 'Untitled Context',
              content: (requestData as any).content || '',
              created_at: new Date().toISOString(),
              metadata: (requestData as any).metadata || {},
            };

            // Add to conversation's context
            conversation.global_context.push(contextItem);
            conversations.set(conversationId, conversation);

            res.writeHead(200);
            res.end(JSON.stringify({
              status: 'Success',
              context_id: contextId,
            }));
          } else {
            res.writeHead(404);
            res.end(JSON.stringify({
              status: 'Error',
              error: 'Conversation not found',
            }));
          }
          return;
        }

        // Health check endpoint
        if (pathname === '/health') {
          res.writeHead(200);
          res.end(JSON.stringify({ status: 'OK' }));
          return;
        }

        // Default 404
        res.writeHead(404);
        res.end(JSON.stringify({
          status: 'Error',
          error: 'Not Found',
        }));
      } catch (error) {
        res.writeHead(500);
        res.end(JSON.stringify({
          status: 'Error',
          error: error.message,
        }));
      }
    });
  });

  return new Promise((resolve, reject) => {
    server.listen(0, 'localhost', () => {
      const address = server.address();
      if (!address || typeof address === 'string') {
        reject(new Error('Failed to start test server'));
        return;
      }

      const testServer = {
        server,
        address: () => `http://localhost:${address.port}`,
        port: address.port,
      };

      resolve(testServer);
    });

    server.on('error', reject);
  });
}

export async function stopTestServer(testServer: any): Promise<void> {
  return new Promise((resolve, reject) => {
    if (!testServer || !testServer.server) {
      resolve();
      return;
    }

    testServer.server.close((err: any) => {
      if (err) {
        reject(err);
      } else {
        resolve();
      }
    });
  });
}
