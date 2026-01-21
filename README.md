# Spawn 🎮

**A focused Linux CLI for installing, organizing, and integrating non-Steam games**

Spawn is a Linux-first CLI tool that turns game installers, archives, and portable game folders into clean, playable desktop games with minimal effort.

You give Spawn a path — a file or a directory — and Spawn handles the rest.

## What Spawn Handles

Spawn automatically detects and processes:

### 🪟 Windows Games (with Proton)
- **Windows installers** (.exe, .msi)
- **Portable Windows games** (folders with .exe files)
- Automatically uses Steam's Proton to run Windows games on Linux
- Creates proper Wine prefixes for each game

### 🐧 Linux-Native Games
- **Archives**: .zip, .tar.gz, .tar.xz, .tar.bz2
- **Portable games**: Pre-extracted game folders
- **AppImages**: Linux application bundles
- Automatically fixes executable permissions
- Identifies the correct binary to launch

## Key Features

✅ **Path-Based Design** - No assumptions about where your games come from  
✅ **Automatic Game Type Detection** - Identifies Windows vs Linux, installers vs archives  
✅ **Proton Integration** - Uses Steam's Proton for Windows games (no manual setup)  
✅ **Steam Integration** - Adds games as Non-Steam entries with Proton support  
✅ **Smart Steam Detection** - Prompts you to close Steam before modifying shortcuts  
✅ **Clean File Organization** - Games installed to `~/Games` by default  
✅ **Desktop Shortcuts** - Automatic .desktop file creation  
✅ **Interactive Mode** - Explains every action with user confirmations  
✅ **Auto Mode** - `--auto` flag for scripted installations

## Installation

### From Source

```bash
git clone https://github.com/Anayo-Anyafulu/Spawn.git
cd Spawn
cargo install --path .
```

### Requirements

- Rust toolchain (for building)
- Steam (for Proton and Non-Steam game integration)
- Standard Linux tools: `unzip`, `tar` (usually pre-installed)

## Usage

### Basic Installation

```bash
# Install a Linux game from archive
spawn ~/Downloads/cool_game.tar.gz

# Install a Windows game with Proton
spawn ~/Downloads/game_installer.exe

# Install from a portable game folder
spawn ~/Games/MyPortableGame

# Add game to Steam automatically
spawn ~/Downloads/game.zip --steam
```

### Command-Line Options

```bash
spawn <PATH>               # Install game from path
spawn <PATH> --steam       # Also add to Steam
spawn <PATH> --name "Name" # Override game name
spawn <PATH> --icon path   # Use custom icon
spawn <PATH> --auto        # Auto mode, no prompts
spawn --uninstall "Name"   # Uninstall a game
spawn --dry-run <PATH>     # See what would happen
```

### Configuration

```bash
# Set default install directory
spawn --set-install-dir ~/MyGames

# Set default search directory
spawn --set-search-dir ~/Downloads
```

## How It Works

### 1. Detection
Spawn inspects your input and determines:
- Is it Windows or Linux?
- Is it an installer, archive, or portable folder?

```
🎮 Spawn v0.1.0

▶ Detected: Windows Installer (requires Proton)

▶ Processing: My Awesome Game
```

### 2. Installation
Based on the detected type:
- **Windows installer**: Runs with Proton, tracks installation
- **Linux archive**: Extracts, finds executable, fixes permissions
- **Portable**: Validates and organizes

### 3. Integration
- Creates desktop shortcuts (`.local/share/applications/` and `~/Desktop`)
- Adds to Steam (if `--steam` flag used)
- Steam addition includes:
  - Automatic Proton configuration for Windows games
  - Correct executable and working directory
  - Icon support

## Examples

### Linux Game Archive
```bash
$ spawn ~/Downloads/Dispatch.tar.gz

🎮 Spawn v0.1.0

▶ Detected: Linux Archive

▶ Processing: Dispatch

▶ Extracting Dispatch.tar.gz...
✔ Extracted successfully
✔ Executable: dispatch
✔ Fixed executable permissions
✔ Icon: icon.png
✔ Shortcut: dispatch.desktop

🎮 Dispatch is ready to play! ✨
```

### Windows Game Installer
```bash
$ spawn ~/Downloads/game_setup.exe --steam

🎮 Spawn v0.1.0

▶ Detected: Windows Installer (requires Proton)

▶ Processing: My Game

▶ Installing Windows game using Proton
✔ Found Proton: Proton 8.0
▶ Game will be installed to: /home/user/Games/My_Game
▶ Running installer...
  (Follow the installer prompts)
✔ Installation completed
✔ Found installed game executable
✔ Icon: game.ico
✔ Shortcut: my-game.desktop

⚠ Steam is currently running.
  Steam must be closed to safely modify Non-Steam game shortcuts.
  This ensures your shortcuts are saved correctly.

  Please close Steam and press Enter to continue...

✔ Steam has been closed. Continuing...

✔ Added My Game to Steam! (Restart Steam to see changes)

🎮 My Game is ready to play! ✨
```

## Philosophy

Spawn exists to answer one question:

**"I already have this game — how do I make it cleanly playable on Linux?"**

### What Spawn Is NOT

- ❌ Not a game launcher
- ❌ Not a Proton/Wine manager
- ❌ Not a game downloader
- ❌ Not a GUI application
- ❌ Not a background service

Spawn does **one thing well**: turn existing games into clean, playable Linux installs.

## Supported Formats

### Archives
- `.zip`
- `.tar.gz` / `.tgz`
- `.tar.xz`
- `.tar.bz2`
- `.AppImage`

### Windows
- `.exe` (installers)
- `.msi` (installers)
- Portable Windows games (folders with `.exe`)

### Linux
- ELF binaries
- Shell scripts (`.sh`)
- Portable Linux games (pre-extracted folders)

## Configuration Files

- Config: `~/.config/spawn/config.toml`
- Games: `~/Games/` (customizable)
- Shortcuts: `~/.local/share/applications/`

## Troubleshooting

### "Proton not found"
Install Proton through Steam:
1. Open Steam
2. Go to Library → Tools
3. Install "Proton Experimental" or latest Proton version

### "No executable found"
Spawn couldn't identify the game's main executable.  
This may happen with unusual archive structures or non-standard game layouts.

### Steam integration issues
- Make sure Steam is installed at `~/.steam/steam` or `~/.local/share/Steam`
- Close Steam before adding games as Non-Steam shortcuts
- Restart Steam after adding games to see them in your library

## Contributing

Contributions welcome! Please feel free to submit pull requests or open issues.

## License

MIT License - See LICENSE file for details

## Acknowledgments

Built with:
- [clap](https://github.com/clap-rs/clap) - Command-line parsing
- [steam_shortcuts_util](https://github.com/PhilipK/steam_shortcuts_util) - Steam shortcuts management
- [sysinfo](https://github.com/GuillaumeGomez/sysinfo) - Process detection

---

Made with ❤️ for Linux gamers
