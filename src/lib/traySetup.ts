import { TrayIcon } from '@tauri-apps/api/tray';
import { Menu } from '@tauri-apps/api/menu';
import { invoke } from '@tauri-apps/api/core';

// Create a minimal 32x32 green icon (RGBA format) - F (letter for Flint Launcher)
function createGreenIconRGBA(): Uint8Array {
  // 32x32 pixels = 1024 pixels, each pixel is 4 bytes (RGBA) = 4096 bytes
  const data = new Uint8Array(32 * 32 * 4);
  
  // Fill with green icon - create a simple "F" shape
  // For now, just fill with green background
  for (let i = 0; i < data.length; i += 4) {
    data[i] = 34;      // R - dark gray
    data[i + 1] = 197; // G - green
    data[i + 2] = 94;  // B - dark green
    data[i + 3] = 255; // A - fully opaque
  }
  
  // Create a simple "F" letter in the middle (approximation)
  // Top horizontal line
  for (let y = 4; y < 8; y++) {
    for (let x = 8; x < 24; x++) {
      const idx = (y * 32 + x) * 4;
      data[idx] = 255;     // R
      data[idx + 1] = 255; // G
      data[idx + 2] = 255; // B
      // A already 255
    }
  }
  
  // Vertical line
  for (let y = 8; y < 28; y++) {
    for (let x = 8; x < 12; x++) {
      const idx = (y * 32 + x) * 4;
      data[idx] = 255;
      data[idx + 1] = 255;
      data[idx + 2] = 255;
    }
  }
  
  // Middle horizontal line
  for (let y = 14; y < 18; y++) {
    for (let x = 8; x < 20; x++) {
      const idx = (y * 32 + x) * 4;
      data[idx] = 255;
      data[idx + 1] = 255;
      data[idx + 2] = 255;
    }
  }
  
  return data;
}

export async function setupTray() {
  try {
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

    const tray = await TrayIcon.new({
      icon: {
        rgba: createGreenIconRGBA(),
        width: 32,
        height: 32,
      },
      menu,
      tooltip: 'Flint Launcher',
    });

    console.log('✅ System tray initialized successfully');
    return tray;
  } catch (err) {
    console.error('❌ Failed to initialize system tray:', err);
    throw err;
  }
}

