# Flint Launcher

A lightweight, modern and fast Minecraft launcher built with Tauri and Svelte.

## Features

- 🎮 **Launch Minecraft** - Play multiple versions with ease
- 📚 **Version Management** - Download, install, and manage game versions
- 👤 **Account Management** - Create and switch between accounts
- ☕ **Automatic Java Setup** - Automatic Java runtime installation (8, 16, 17, 21)
- 🎨 **Modern UI** - Clean, responsive interface built with Svelte and Tailwind
- 🚀 **Fast & Lightweight** - Native performance with Tauri

## Prerequisites

- **Node.js** 18+ and npm
- **Rust** (for building Tauri backend)
- **Windows** (currently Windows-only)

## Getting Started

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/flint-launcher.git
cd flint-launcher

# Install dependencies
npm install
```

### Development

```bash
# Start development server with live reload
npm run tauri dev
```

This will launch the app in development mode with hot reload enabled.

### Build

```bash
# Build for production
npm run build

# Build the Tauri application
npm run tauri build
```

The compiled installer will be in `src-tauri/target/release/bundle/`.

## Project Structure

```
flint-launcher/
├── src/                    # SvelteKit frontend
│   ├── routes/            # App pages and layouts
│   ├── app.css            # Global styles
│   └── app.html           # Entry point
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # App setup
│   │   ├── lib.rs         # Tauri commands
│   │   ├── launchprocess.rs # Game launcher logic
│   │   └── library.rs     # Version management
│   ├── tauri.conf.json    # Tauri config
│   └── Cargo.toml         # Rust dependencies
└── package.json           # Node dependencies
```

## How It Works

1. **Account**: Create or switch Minecraft accounts
2. **Library**: Download and manage game versions from Mojang
3. **Java**: Automatic download of required Java runtimes
4. **Launch**: Execute the game with proper JVM arguments

## Technologies

- **Frontend**: Svelte 5, SvelteKit, TailwindCSS, Flowbite
- **Backend**: Rust, Tauri 2
- **Styling**: TailwindCSS with Flowbite components

## License

MIT

## Support

For issues or feature requests, please open an issue on GitHub.
