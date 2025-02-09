import { invoke } from '@tauri-apps/api/tauri';

// Initialize Tauri
export async function initTauri() {
  try {
    // Test if Tauri is available
    await invoke('ping');
    return true;
  } catch (e) {
    console.warn('Tauri not available:', e);
    return false;
  }
}

// Helper function for Tauri invocations
export async function invokeTauri(command: string, args?: any) {
  try {
    return await invoke(command, args);
  } catch (e) {
    console.error(`Error invoking Tauri command ${command}:`, e);
    throw e;
  }
} 