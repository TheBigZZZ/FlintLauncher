# Flint Launcher

A lightweight, modern and fast Minecraft launcher built with Tauri and Svelte.

## Features

- 🎮 **Launch Minecraft** - Play multiple versions with ease
- 📚 **Version Management** - Download and manage game versions with fast, parallelized downloads
- 👤 **Account Management** - Create and switch between accounts
- 🎮 **Modloader Support** - Install Fabric and Forge modloaders alongside vanilla
- 👨‍💻 **Game Profiles** - Create custom profiles with individual RAM allocation (512MB-16GB)
- ☕ **Automatic Java Setup** - Smart Java runtime detection (system → bundled → auto-download)
- 📊 **Real-time Progress** - Live activity logs with per-file download tracking
- 🎨 **Modern UI** - Clean, responsive interface built with Svelte and Tailwind
- 🚀 **Fast & Lightweight** - Native performance with Tauri and optimized parallel downloads

## Prerequisites

- **Node.js** 18+ and npm
- **Rust** (for building Tauri backend)
- **Windows** (currently Windows-only)

## Getting Started

### Installation

```bash
# Clone the repository
git clone https://github.com/FaizeenHoque/FlintLauncher.git
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
├── src/                       # SvelteKit frontend
│   ├── routes/               # App pages and layouts
│   │   ├── +layout.svelte    # Main layout with navbar
│   │   ├── +page.svelte      # Account/main page
│   │   ├── library/          # Version/profile management
│   │   └── account/          # Account creation/settings
│   ├── lib/stores/           # Svelte stores (download progress)
│   ├── app.css               # Global styles
│   └── app.html              # Entry point
├── src-tauri/                # Rust backend
│   ├── src/
│   │   ├── main.rs           # App initialization
│   │   ├── lib.rs            # Tauri command exports
│   │   ├── launchprocess/    # Game launching logic
│   │   │   ├── gameSpawning.rs      # Java process spawning
│   │   │   ├── javaDiscovery.rs     # Java runtime detection
│   │   │   ├── classpathBuilder.rs  # Classpath construction
│   │   │   └── pathManagement.rs    # Data directory management
│   │   ├── library.rs        # Version/profile management commands
│   │   └── accounts/         # Account management
│   ├── tauri.conf.json       # Tauri configuration
│   ├── capabilities/         # Security capabilities
│   └── Cargo.toml            # Rust dependencies
├── static/                   # Static assets
├── package.json              # Node dependencies and scripts
└── README.md                 # This file
```

## How It Works

### Main Workflow
1. **Account**: Create or switch Minecraft accounts with proper UUID generation
2. **Library**: Download and manage game versions from Mojang with real-time progress
3. **Modloaders** (Optional): Install Fabric or Forge to extend Minecraft functionality
4. **Profiles**: Create custom game profiles with:
   - Individual RAM allocation (adjustable from 512MB to 16GB)
   - Modloader selection (Vanilla, Fabric, or Forge)
   - Last played tracking
5. **Java**: Smart runtime detection checks system PATH, bundled Java, then auto-downloads if needed
6. **Launch**: Execute the game with optimized JVM arguments and capture real-time output

### Download System
- **Parallel Downloads**: Up to 32 concurrent file downloads for fast installation
- **Shared Connection Pool**: Single HTTP client with connection pooling and 300s timeout
- **Progress Tracking**: Real-time per-file status in activity log
- **Cancellable**: Stop downloads at any time and retry failed components

## Profiles & Modloaders

### Game Profiles
Create multiple profiles for different playstyles:
- **Custom RAM**: Adjust per-profile JVM heap size (512MB-16GB)
- **Independent Configs**: Each profile maintains its own game settings
- **Play History**: Track last played time for each profile
- **Modloader Support**: Assign Fabric or Forge to specific profiles

### Modloader Support
**Fabric**: Lightweight modding platform
- Automatic version fetching from Fabric API
- Stable versions prioritized

**Forge**: Legacy and modern modding support
- Maven-based version management
- Full Minecraft version compatibility

---

## Technologies

- **Frontend**: Svelte 5, SvelteKit, TailwindCSS, Flowbite
- **Backend**: Rust, Tauri 2
- **Styling**: TailwindCSS with Flowbite components

## License

MIT

## Support

For issues or feature requests, please open an issue on GitHub.
