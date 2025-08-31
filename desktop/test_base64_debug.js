import fs from 'fs';

// Test the exact privateKey from the secret
const privateKey = "RygvlKGUDG9/loCY5KCHrQDrnJEG7P7P9HKb+BE8NS0=";

console.log('🔍 Testing privateKey Base64 decoding...');
console.log('Private key:', privateKey);
console.log('Length:', privateKey.length);
console.log('Length % 4:', privateKey.length % 4);

// Test Node.js base64 decoding
try {
    const decoded = Buffer.from(privateKey, 'base64');
    console.log('✅ Node.js Base64 decode successful,', decoded.length, 'bytes');
    console.log('Decoded bytes:', Array.from(decoded).map(b => b.toString(16).padStart(2, '0')).join(' '));
} catch (error) {
    console.error('❌ Node.js Base64 decode failed:', error.message);
}

// Test with different padding scenarios
const testCases = [
    { name: 'Original', key: privateKey },
    { name: 'No padding', key: privateKey.replace(/=+$/, '') },
    { name: 'Extra padding', key: privateKey + '=' },
    { name: 'Double padding', key: privateKey + '==' },
];

for (const testCase of testCases) {
    console.log(`\n🔍 Testing ${testCase.name}:`);
    console.log('Key:', testCase.key);
    console.log('Length:', testCase.key.length);
    console.log('Length % 4:', testCase.key.length % 4);

    try {
        const decoded = Buffer.from(testCase.key, 'base64');
        console.log(`✅ ${testCase.name} decode successful,`, decoded.length, 'bytes');
    } catch (error) {
        console.log(`❌ ${testCase.name} decode failed:`, error.message);
    }
}

// Test the full secret decoding
console.log('\n🔍 Testing full secret decoding...');
const ATOMIC_SERVER_SECRET = process.env.ATOMIC_SERVER_SECRET;

if (ATOMIC_SERVER_SECRET) {
    try {
        const decoded = Buffer.from(ATOMIC_SERVER_SECRET, 'base64');
        const jsonStr = decoded.toString('utf8');
        const json = JSON.parse(jsonStr);

        console.log('✅ Full secret decode successful');
        console.log('JSON keys:', Object.keys(json));

        // Test the privateKey from the decoded JSON
        if (json.privateKey) {
            console.log('\n🔍 Testing privateKey from decoded JSON:');
            console.log('Private key from JSON:', json.privateKey);
            console.log('Length:', json.privateKey.length);
            console.log('Length % 4:', json.privateKey.length % 4);

            try {
                const privateKeyDecoded = Buffer.from(json.privateKey, 'base64');
                console.log('✅ PrivateKey from JSON decode successful,', privateKeyDecoded.length, 'bytes');
            } catch (error) {
                console.log('❌ PrivateKey from JSON decode failed:', error.message);
            }
        }
    } catch (error) {
        console.error('❌ Full secret decode failed:', error.message);
    }
} else {
    console.log('❌ ATOMIC_SERVER_SECRET not set');
}
