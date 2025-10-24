import { createServer, type Server } from 'node:http';
import { URL } from 'node:url';

// Simple test server that mimics the Terraphim API
export async function startTestServer(_config: Record<string, unknown>): Promise<{
  server: Server,
  address: () => string,
  port: number
}> {
	const conversations = new Map();
	const _contexts = new Map();
	let nextConversationId = 1;
	let nextContextId = 1;

<<<<<<< HEAD
  // Create a default conversation on startup
  const defaultConversationId = `conv-${nextConversationId++}`;
  conversations.set(defaultConversationId, {
    id: defaultConversationId,
    title: 'Default Conversation',
    role: 'default',
    created_at: new Date().toISOString(),
    contexts: [], // Changed from global_context
    messages: [],
  });

  const server = createServer((req, res) => {
    const url = new URL(req.url!, `http://${req.headers.host}`);
    const method = req.method;
    const pathname = url.pathname;
=======
	const server = createServer((req, res) => {
		const url = new URL(req.url || '/', `http://${req.headers.host || 'localhost'}`);
		const method = req.method;
		const pathname = url.pathname;
>>>>>>> origin/main

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
		req.on('data', (chunk) => {
			body += chunk.toString();
		});

		req.on('end', () => {
			let requestData = {};
			if (body) {
				try {
					requestData = JSON.parse(body);
				} catch (_e) {
					// Ignore parsing errors
				}
			}

			res.setHeader('Content-Type', 'application/json');

<<<<<<< HEAD
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
              contexts: [], // Changed from global_context
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
=======
			try {
				// Route handling
				if (pathname === '/conversations') {
					if (method === 'GET') {
						// List conversations
						const conversationList = Array.from(conversations.values());
						res.writeHead(200);
						res.end(
							JSON.stringify({
								status: 'Success',
								conversations: conversationList,
							})
						);
						return;
					} else if (method === 'POST') {
						// Create conversation
						const conversationId = `conv-${nextConversationId++}`;
						const conversation = {
							id: conversationId,
							title: (requestData as { title?: string }).title || 'Untitled Conversation',
							role: (requestData as { role?: string }).role || 'default',
							created_at: new Date().toISOString(),
							global_context: [],
							messages: [],
						};
						conversations.set(conversationId, conversation);
						res.writeHead(200);
						res.end(
							JSON.stringify({
								status: 'Success',
								conversation_id: conversationId,
							})
						);
						return;
					}
				}
>>>>>>> origin/main

				// Get specific conversation
				const conversationMatch = pathname.match(/^\/conversations\/([^/]+)$/);
				if (conversationMatch && method === 'GET') {
					const conversationId = conversationMatch[1];
					const conversation = conversations.get(conversationId);
					if (conversation) {
						res.writeHead(200);
						res.end(
							JSON.stringify({
								status: 'Success',
								conversation,
							})
						);
					} else {
						res.writeHead(404);
						res.end(
							JSON.stringify({
								status: 'Error',
								error: 'Conversation not found',
							})
						);
					}
					return;
				}

				// Add context to conversation
				const contextMatch = pathname.match(/^\/conversations\/([^/]+)\/context$/);
				if (contextMatch && method === 'POST') {
					const conversationId = contextMatch[1];
					const conversation = conversations.get(conversationId);

					if (conversation) {
						const contextId = `ctx-${nextContextId++}`;
						const contextItem = {
							id: contextId,
							context_type: (requestData as { context_type?: string }).context_type || 'document',
							title: (requestData as { title?: string }).title || 'Untitled Context',
							summary: (requestData as { summary?: string }).summary || '',
							content: (requestData as { content?: string }).content || '',
							created_at: new Date().toISOString(),
							metadata: (requestData as { metadata?: Record<string, unknown> }).metadata || {},
							relevance_score: (requestData as { relevance_score?: number | null }).relevance_score || null,
						};

<<<<<<< HEAD
            // Add to conversation's context
            conversation.contexts.push(contextItem); // Changed from global_context
            conversations.set(conversationId, conversation);
=======
						// Add to conversation's context
						conversation.global_context.push(contextItem);
						conversations.set(conversationId, conversation);
>>>>>>> origin/main

						res.writeHead(200);
						res.end(
							JSON.stringify({
								status: 'Success',
								context_id: contextId,
							})
						);
					} else {
						res.writeHead(404);
						res.end(
							JSON.stringify({
								status: 'Error',
								error: 'Conversation not found',
							})
						);
					}
					return;
				}

				// Delete or Update context from conversation
				const contextItemMatch = pathname.match(/^\/conversations\/([^/]+)\/context\/([^/]+)$/);
				if (contextItemMatch) {
					const conversationId = contextItemMatch[1];
					const contextId = contextItemMatch[2];
					const conversation = conversations.get(conversationId);

					if (!conversation) {
						res.writeHead(404);
						res.end(
							JSON.stringify({
								status: 'Error',
								error: 'Conversation not found',
							})
						);
						return;
					}

<<<<<<< HEAD
          const contextIndex = conversation.contexts.findIndex((ctx: any) => ctx.id === contextId); // Changed from global_context

          if (method === 'DELETE') {
            if (contextIndex !== -1) {
              // Remove context
              conversation.contexts.splice(contextIndex, 1); // Changed from global_context
              conversations.set(conversationId, conversation);
=======
					const contextIndex = conversation.global_context.findIndex(
						(ctx: { id: string }) => ctx.id === contextId
					);

					if (method === 'DELETE') {
						if (contextIndex !== -1) {
							// Remove context
							conversation.global_context.splice(contextIndex, 1);
							conversations.set(conversationId, conversation);
>>>>>>> origin/main

							res.writeHead(200);
							res.end(
								JSON.stringify({
									status: 'Success',
									error: null,
								})
							);
						} else {
							res.writeHead(200);
							res.end(
								JSON.stringify({
									status: 'Error',
									error: 'Context not found',
								})
							);
						}
						return;
					}

<<<<<<< HEAD
          if (method === 'PUT') {
            if (contextIndex !== -1) {
              // Update context
              const existingContext = conversation.contexts[contextIndex]; // Changed from global_context
              const updatedContext = {
                ...existingContext,
                context_type: (requestData as any).context_type || existingContext.context_type,
                title: (requestData as any).title !== undefined ? (requestData as any).title : existingContext.title,
                summary: (requestData as any).summary !== undefined ? (requestData as any).summary : existingContext.summary,
                content: (requestData as any).content !== undefined ? (requestData as any).content : existingContext.content,
                metadata: (requestData as any).metadata !== undefined ? (requestData as any).metadata : existingContext.metadata,
              };

              conversation.contexts[contextIndex] = updatedContext; // Changed from global_context
              conversations.set(conversationId, conversation);
=======
					if (method === 'PUT') {
						if (contextIndex !== -1) {
							// Update context
							const existingContext = conversation.global_context[contextIndex];
							const updatedContext = {
								...existingContext,
								context_type: (requestData as { context_type?: string }).context_type || existingContext.context_type,
								title:
									(requestData as { title?: string }).title !== undefined
										? (requestData as { title?: string }).title
										: existingContext.title,
								summary:
									(requestData as { summary?: string }).summary !== undefined
										? (requestData as { summary?: string }).summary
										: existingContext.summary,
								content:
									(requestData as { content?: string }).content !== undefined
										? (requestData as { content?: string }).content
										: existingContext.content,
								metadata:
									(requestData as { metadata?: Record<string, unknown> }).metadata !== undefined
										? (requestData as { metadata?: Record<string, unknown> }).metadata
										: existingContext.metadata,
							};

							conversation.global_context[contextIndex] = updatedContext;
							conversations.set(conversationId, conversation);
>>>>>>> origin/main

							res.writeHead(200);
							res.end(
								JSON.stringify({
									status: 'Success',
									context: updatedContext,
									error: null,
								})
							);
						} else {
							res.writeHead(200);
							res.end(
								JSON.stringify({
									status: 'Error',
									context: null,
									error: 'Context not found',
								})
							);
						}
						return;
					}
				}

				// Health check endpoint
				if (pathname === '/health') {
					res.writeHead(200);
					res.end(JSON.stringify({ status: 'OK' }));
					return;
				}

<<<<<<< HEAD
        // Default 404
        res.writeHead(404);
        res.end(JSON.stringify({
          status: 'Error',
          error: 'Not Found',
        }));
      } catch (error) {
        const err = error instanceof Error ? error : new Error(String(error));
        res.writeHead(500);
        res.end(JSON.stringify({
          status: 'Error',
          error: err.message,
        }));
      }
    });
  });
=======
				// Default 404
				res.writeHead(404);
				res.end(
					JSON.stringify({
						status: 'Error',
						error: 'Not Found',
					})
				);
			} catch (error) {
				res.writeHead(500);
				res.end(
					JSON.stringify({
						status: 'Error',
						error: error.message,
					})
				);
			}
		});
	});
>>>>>>> origin/main

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

export async function stopTestServer(testServer: {
  server: Server,
  address: () => string,
  port: number
}): Promise<void> {
	return new Promise((resolve, reject) => {
		if (!testServer || !testServer.server) {
			resolve();
			return;
		}

		testServer.server.close((err: Error | null) => {
			if (err) {
				reject(err);
			} else {
				resolve();
			}
		});
	});
}
