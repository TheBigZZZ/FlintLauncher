import { TrayIcon } from '@tauri-apps/api/tray';
import { Menu } from '@tauri-apps/api/menu';
import { invoke } from '@tauri-apps/api/core';
import { resourceDir } from '@tauri-apps/api/path';

// Global flag to track if tray has been initialized
let trayInitialized = false;
let trayInstance: TrayIcon | null = null;

export async function setupTray() {
  try {
    // Prevent multiple tray instances from being created using local flag
    if (trayInitialized && trayInstance) {
      console.log('ℹ️ System tray already initialized, skipping duplicate setup');
      return trayInstance;
    }

    console.log('🔧 Setting up system tray...');

    const menu = await Menu.new({
      items: [
        {
          id: 'show',
          text: 'Show Window',
          action: async () => {
            console.log('📂 Tray clicked: Show Window');
            try {
              await invoke('show_main_window');
            } catch (err) {
              console.error('Failed to show window:', err);
            }
          },
        },
        {
          id: 'quit',
          text: 'Quit Launcher',
          action: async () => {
            console.log('🚪 Tray clicked: Quit Launcher');
            try {
              await invoke('quit_app');
            } catch (err) {
              console.error('Failed to quit app:', err);
            }
          },
        },
      ],
    });

    // Get the resource directory and load the tray icon
    const resourcePath = await resourceDir();
    const iconPath = `${resourcePath}icons/32x32.png`;

    trayInstance = await TrayIcon.new({
      id: 'flint-launcher',
      icon: iconPath,
      menu,
      tooltip: 'Flint Launcher',
    });

    trayInitialized = true;
    console.log('✅ System tray initialized successfully');
    return trayInstance;
  } catch (err) {
    console.error('❌ Failed to initialize system tray:', err);
    throw err;
  }
}

// Cleanup function to destroy the tray icon if needed
export async function cleanupTray() {
  if (trayInstance) {
    try {
      await trayInstance.close();
      trayInstance = null;
      trayInitialized = false;
      console.log('🧹 System tray cleaned up');
    } catch (err) {
      console.error('❌ Failed to cleanup system tray:', err);
    }
  }
}

