import { spawn, ChildProcess } from 'child_process';

let tauriDriverProcess: ChildProcess | null = null;

async function globalSetup() {
  console.log('🚀 Starting Tauri WebDriver global setup...');
  
  // Start Tauri driver
  tauriDriverProcess = spawn('tauri-driver', [], {
    stdio: 'inherit',
    shell: true,
    detached: false
  });

  // Wait for driver to start
  await new Promise(resolve => setTimeout(resolve, 5000));
  
  console.log('✅ Tauri WebDriver global setup complete');
}

async function globalTeardown() {
  console.log('🧹 Starting Tauri WebDriver global teardown...');
  
  if (tauriDriverProcess) {
    tauriDriverProcess.kill();
    console.log('✅ Tauri WebDriver process terminated');
  }
  
  console.log('✅ Tauri WebDriver global teardown complete');
}

export { globalSetup, globalTeardown }; 