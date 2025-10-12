import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

describe('Context Management Integration', () => {
  beforeEach(async () => {
    // Start the backend server for integration testing
    try {
      const { stdout } = await execAsync('cargo run --bin terraphim_server -- --config terraphim_server/default/terraphim_engineer_config.json &', {
        timeout: 10000,
        cwd: '/Users/alex/projects/terraphim/terraphim-ai'
      });
      console.log('Server started:', stdout);
    } catch (error) {
      console.log('Server might already be running:', error.message);
    }

    // Wait for server to be ready
    await new Promise(resolve => setTimeout(resolve, 2000));
  });

  afterEach(async () => {
    // Clean up - stop the server
    try {
      await execAsync('pkill -f terraphim_server');
    } catch (error) {
      // Ignore cleanup errors
    }
  });

  it('should create conversations and add context via API', async () => {
    const baseUrl = 'http://127.0.0.1:8080';

    try {
      // 1. Test conversation creation
      console.log('Testing conversation creation...');
      const createResponse = await fetch(`${baseUrl}/conversations`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          title: 'Integration Test Conversation',
          role: 'Terraphim Engineer'
        })
      });

      expect(createResponse.ok).toBe(true);
      const createData = await createResponse.json();
      expect(createData.status).toBe('Success');
      expect(createData.conversation_id).toBeDefined();

      const conversationId = createData.conversation_id;
      console.log('Created conversation:', conversationId);

      // 2. Test context addition
      console.log('Testing context addition...');
      const contextResponse = await fetch(`${baseUrl}/conversations/${conversationId}/context`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          context_type: 'document',
          title: 'Integration Test Document',
          content: 'This is a test document for context management integration testing. It should appear in the conversation context.',
          metadata: {
            source_type: 'test',
            document_id: 'integration-test-doc-1',
            tags: 'integration, test, context'
          }
        })
      });

      expect(contextResponse.ok).toBe(true);
      const contextData = await contextResponse.json();
      expect(contextData.status).toBe('Success');
      expect(contextData.context_id).toBeDefined();

      console.log('Added context:', contextData.context_id);

      // 3. Test conversation retrieval with context
      console.log('Testing conversation retrieval...');
      const getResponse = await fetch(`${baseUrl}/conversations/${conversationId}`);
      expect(getResponse.ok).toBe(true);
      const getData = await getResponse.json();
      expect(getData.status).toBe('Success');
      expect(getData.conversation).toBeDefined();
      expect(getData.conversation.global_context).toBeDefined();
      expect(getData.conversation.global_context.length).toBe(1);

      const contextItem = getData.conversation.global_context[0];
      expect(contextItem.title).toBe('Integration Test Document');
      expect(contextItem.context_type).toBe('document');
      expect(contextItem.content).toContain('integration testing');

      console.log('Context retrieved successfully:', contextItem);

      // 4. Test multiple context items
      console.log('Testing multiple context items...');
      const context2Response = await fetch(`${baseUrl}/conversations/${conversationId}/context`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          context_type: 'search_result',
          title: 'Second Context Item',
          content: 'This is a second context item to test multiple items in conversation.',
          metadata: {
            source_type: 'search',
            rank: 95
          }
        })
      });

      expect(context2Response.ok).toBe(true);

      // Retrieve updated conversation
      const getResponse2 = await fetch(`${baseUrl}/conversations/${conversationId}`);
      const getData2 = await getResponse2.json();
      expect(getData2.conversation.global_context.length).toBe(2);

      console.log('Multiple context items test passed');

      // 5. Test conversation listing
      console.log('Testing conversation listing...');
      const listResponse = await fetch(`${baseUrl}/conversations`);
      expect(listResponse.ok).toBe(true);
      const listData = await listResponse.json();
      expect(listData.status).toBe('Success');
      expect(listData.conversations).toBeDefined();
      expect(listData.conversations.length).toBeGreaterThanOrEqual(1);

      const foundConversation = listData.conversations.find((conv: any) => conv.id === conversationId);
      expect(foundConversation).toBeDefined();
      expect(foundConversation.title).toBe('Integration Test Conversation');

      console.log('All context management integration tests passed!');

    } catch (error) {
      console.error('Integration test failed:', error);
      throw error;
    }
  }, 30000); // 30 second timeout for integration test

  it('should handle error cases gracefully', async () => {
    const baseUrl = 'http://127.0.0.1:8080';

    // Test invalid conversation ID
    const invalidResponse = await fetch(`${baseUrl}/conversations/invalid-id`);
    expect(invalidResponse.status).toBe(404);

    // Test adding context to non-existent conversation
    const contextResponse = await fetch(`${baseUrl}/conversations/invalid-id/context`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        context_type: 'document',
        title: 'Test',
        content: 'Test content'
      })
    });
    expect(contextResponse.status).toBe(404);

    console.log('Error handling tests passed');
  });
});
