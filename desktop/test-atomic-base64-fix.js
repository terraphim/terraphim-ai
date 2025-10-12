import fs from 'fs';

const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;

async function testAtomicBase64Fix() {
  console.log('🔍 Testing Atomic Server Base64 Fix...');

  if (!ATOMIC_SERVER_SECRET) {
    console.error('❌ ATOMIC_SERVER_SECRET environment variable is not set');
    return;
  }

  console.log('📊 Secret length:', ATOMIC_SERVER_SECRET.length);
  console.log('📊 Secret (first 50 chars):', ATOMIC_SERVER_SECRET.substring(0, 50));

  // Test 1: Node.js base64 decoding (should work)
  try {
    const decoded = Buffer.from(ATOMIC_SERVER_SECRET, 'base64');
    console.log('✅ Node.js Base64 decode successful,', decoded.length, 'bytes');

    const jsonStr = decoded.toString('utf8');
    const json = JSON.parse(jsonStr);
    console.log('✅ JSON parse successful');
    console.log('📊 JSON keys:', Object.keys(json));

    // Check if privateKey needs padding
    const privateKey = json.privateKey;
    if (privateKey) {
      console.log('📊 Private key length:', privateKey.length);
      console.log('📊 Private key length % 4:', privateKey.length % 4);

      // Test if privateKey needs padding
      let paddedPrivateKey = privateKey;
      while (paddedPrivateKey.length % 4 !== 0) {
        paddedPrivateKey += '=';
      }

      try {
        Buffer.from(paddedPrivateKey, 'base64');
        console.log('✅ Private key with padding decodes successfully');
      } catch (e) {
        console.log('❌ Private key with padding still fails:', e.message);
      }
    }

  } catch (error) {
    console.error('❌ Node.js decoding failed:', error);
  }

  // Test 2: Create a fixed secret with proper padding
  try {
    const decoded = Buffer.from(ATOMIC_SERVER_SECRET, 'base64');
    const jsonStr = decoded.toString('utf8');
    const json = JSON.parse(jsonStr);

    // Fix the privateKey padding if needed
    if (json.privateKey && json.privateKey.length % 4 !== 0) {
      let paddedPrivateKey = json.privateKey;
      while (paddedPrivateKey.length % 4 !== 0) {
        paddedPrivateKey += '=';
      }
      json.privateKey = paddedPrivateKey;
      console.log('🔧 Fixed privateKey padding');
    }

    // Fix the publicKey padding if needed
    if (json.publicKey && json.publicKey.length % 4 !== 0) {
      let paddedPublicKey = json.publicKey;
      while (paddedPublicKey.length % 4 !== 0) {
        paddedPublicKey += '=';
      }
      json.publicKey = paddedPublicKey;
      console.log('🔧 Fixed publicKey padding');
    }

    // Create the fixed secret
    const fixedJsonStr = JSON.stringify(json);
    const fixedSecret = Buffer.from(fixedJsonStr, 'utf8').toString('base64');

    console.log('📊 Original secret length:', ATOMIC_SERVER_SECRET.length);
    console.log('📊 Fixed secret length:', fixedSecret.length);
    console.log('📊 Secrets are different:', ATOMIC_SERVER_SECRET !== fixedSecret);

    // Test the fixed secret
    const testDecoded = Buffer.from(fixedSecret, 'base64');
    const testJson = JSON.parse(testDecoded.toString('utf8'));
    console.log('✅ Fixed secret decodes successfully');

    // Save the fixed secret to a file for testing
    const fixedSecretPath = './fixed-atomic-secret.txt';
    fs.writeFileSync(fixedSecretPath, fixedSecret);
    console.log('💾 Saved fixed secret to:', fixedSecretPath);

    // Test 3: Create a test configuration with the fixed secret
    const testConfig = {
      id: "Server",
      global_shortcut: "Ctrl+Shift+F",
      roles: {
        'Atomic Debug Fixed': {
          shortname: "AtomicDebugFixed",
          name: "Atomic Debug Fixed",
          relevance_function: "title-scorer",
          terraphim_it: false,
          theme: "spacelab",
          kg: null,
          haystacks: [
            {
              location: "http://localhost:9883/",
              service: "Atomic",
              read_only: true,
              atomic_server_secret: fixedSecret
            }
          ],
          extra: {}
        }
      },
      default_role: "Atomic Debug Fixed",
      selected_role: "Atomic Debug Fixed"
    };

    const configPath = './atomic-debug-fixed-config.json';
    fs.writeFileSync(configPath, JSON.stringify(testConfig, null, 2));
    console.log('💾 Saved test config to:', configPath);

    // Test 4: Try to update the Terraphim server with the fixed config
    console.log('🔧 Testing Terraphim server update with fixed config...');

    const updateResponse = await fetch('http://localhost:8000/config', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(testConfig)
    });

    console.log('📊 Config update response status:', updateResponse.status);

    if (!updateResponse.ok) {
      const errorText = await updateResponse.text();
      console.log('❌ Config update failed:', errorText);

      // Try to parse error details
      try {
        const errorJson = JSON.parse(errorText);
        console.log('📊 Error details:', errorJson);

        if (errorJson.message && errorJson.message.includes('Base64 decode error')) {
          console.log('🔍 Still getting Base64 decode error with fixed secret!');
          console.log('🔍 This suggests the issue is in the Rust code, not the secret format');
        }
      } catch (e) {
        console.log('📊 Could not parse error as JSON');
      }
    } else {
      console.log('✅ Successfully updated Terraphim server config with fixed secret');

      // Wait for configuration to be applied
      await new Promise(resolve => setTimeout(() => resolve(undefined), 3000));

      // Test atomic haystack search through Terraphim
      console.log('🔍 Testing atomic haystack search with fixed secret...');

      const searchResponse = await fetch('http://localhost:8000/documents/search', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          search_term: 'test',
          role: 'Atomic Debug Fixed',
          limit: 10
        })
      });

      console.log('📊 Search response status:', searchResponse.status);

      if (!searchResponse.ok) {
        const errorText = await searchResponse.text();
        console.log('❌ Search failed:', errorText);
      } else {
        const searchResults = await searchResponse.json();
        console.log('✅ Search successful!');
        console.log('📊 Search results:', JSON.stringify(searchResults, null, 2));
      }
    }

  } catch (error) {
    console.error('❌ Error creating fixed secret:', error);
  }
}

// Run the test
testAtomicBase64Fix().catch(console.error);
