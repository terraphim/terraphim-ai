/**
 * This test file demonstrates how to calculate signatures for commits
 * using the @tomic/lib library in TypeScript.
 * 
 * To run this test:
 * 1. Install Node.js and npm
 * 2. Run `npm install @tomic/lib`
 * 3. Run `npx ts-node commit_test.ts`
 */

// Import required libraries
// Note: You'll need to install these first with npm
// npm install @tomic/lib

// This is a placeholder for the actual implementation
// In a real scenario, you would import the actual @tomic/lib library
const atomicLib = {
  Agent: class {
    subject: string;
    privateKey: string;
    publicKey: string;
    
    constructor(subject: string, privateKey: string, publicKey: string) {
      this.subject = subject;
      this.privateKey = privateKey;
      this.publicKey = publicKey;
    }
    
    static fromSecret(secret: string): any {
      // In a real implementation, this would parse the secret and create an agent
      const parsed = JSON.parse(Buffer.from(secret, 'base64').toString());
      return new this(
        parsed.subject,
        parsed.privateKey,
        parsed.publicKey
      );
    }
    
    sign(message: string): string {
      // In a real implementation, this would use the private key to sign the message
      console.log(`Signing message: ${message}`);
      // This is a placeholder for the actual signature calculation
      return "placeholder_signature";
    }
  },
  
  Commit: class {
    subject: string;
    createdAt: number;
    signer: string;
    set: Record<string, string>;
    remove: string[];
    destroy: boolean;
    signature?: string;
    
    constructor(params: {
      subject: string;
      createdAt: number;
      signer: string;
      set?: Record<string, string>;
      remove?: string[];
      destroy?: boolean;
    }) {
      this.subject = params.subject;
      this.createdAt = params.createdAt;
      this.signer = params.signer;
      this.set = params.set || {};
      this.remove = params.remove || [];
      this.destroy = params.destroy || false;
    }
    
    sign(agent: any): void {
      // Calculate the string to sign
      let toSign = `${this.subject}:${this.createdAt}:${this.signer}`;
      
      // Add the set properties in alphabetical order
      const sortedKeys = Object.keys(this.set).sort();
      for (const key of sortedKeys) {
        toSign += `${key}:${this.set[key]}`;
      }
      
      // Add the remove properties in alphabetical order
      const sortedRemove = [...this.remove].sort();
      for (const key of sortedRemove) {
        toSign += `${key}:`;
      }
      
      // Add the destroy flag
      if (this.destroy) {
        toSign += "destroy:true";
      }
      
      console.log(`String to sign: ${toSign}`);
      
      // Sign the string
      this.signature = agent.sign(toSign);
    }
    
    toJSON(): object {
      return {
        subject: this.subject,
        created_at: this.createdAt,
        signer: this.signer,
        set: this.set,
        remove: this.remove.length > 0 ? this.remove : undefined,
        destroy: this.destroy,
        signature: this.signature
      };
    }
  }
};

// Main function to demonstrate commit signature calculation
async function main() {
  try {
    // Load environment variables
    const serverUrl = process.env.ATOMIC_SERVER_URL || 'http://localhost:9883';
    const secret = process.env.ATOMIC_SERVER_SECRET;
    
    if (!secret) {
      console.error('ATOMIC_SERVER_SECRET environment variable is not set');
      return;
    }
    
    console.log(`Server URL: ${serverUrl}`);
    
    // Create an agent from the secret
    const agent = atomicLib.Agent.fromSecret(secret);
    console.log(`Agent subject: ${agent.subject}`);
    console.log(`Agent public key: ${agent.publicKey}`);
    
    // Create a unique resource ID for testing
    const timestamp = Math.floor(Date.now() / 1000);
    const testResourceId = `${serverUrl}/test-resource-${timestamp}`;
    console.log(`Creating resource with ID: ${testResourceId}`);
    
    // Create a commit
    const commit = new atomicLib.Commit({
      subject: testResourceId,
      createdAt: timestamp,
      signer: agent.subject,
      set: {
        'https://atomicdata.dev/properties/shortname': `test-resource-${timestamp}`,
        'https://atomicdata.dev/properties/description': 'A test resource created by the TypeScript client'
      },
      destroy: false
    });
    
    // Sign the commit
    commit.sign(agent);
    console.log(`Calculated signature: ${commit.signature}`);
    
    // Convert to JSON
    const commitJson = JSON.stringify(commit.toJSON(), null, 2);
    console.log(`Commit JSON: ${commitJson}`);
    
    // In a real implementation, you would send this commit to the server
    console.log(`To send this commit to the server, run:`);
    console.log(`curl -X POST -H "Content-Type: application/json" -d '${JSON.stringify(commit.toJSON())}' ${serverUrl}/commit`);
  } catch (error) {
    console.error('Error:', error);
  }
}

// Run the main function
main().catch(console.error); 