# Spawn 🎮

**Spawn** is a lightweight CLI tool designed to turn Linux game archives into fully integrated desktop applications with a single command.

No more manual extraction, searching for binaries, or manually creating `.desktop` files. Spawn handles the "boring stuff" so you can get straight to playing.

## Features

- **🚀 One-Command Setup**: Point Spawn at a `.tar.gz` archive or a game folder, and it does the rest.
- **🔍 Smart Discovery**: 
    - **Executables**: Uses ELF header verification and common script detection (`start.sh`, `run.sh`) to find the real game binary.
    - **Icons**: Automatically finds and links game icons (`.png`, `.svg`, `.ico`).
- **📂 Robust Extraction**: Seamlessly handles nested directory structures inside archives.
- **🛡️ Respectful & Safe**: 
    - **Permissions**: Adds execute bits without overwriting your existing filesystem permissions.
    - **Idempotency**: Skips extraction if the game directory already exists.
- **✨ Polished UX**: Concise, status-driven terminal output with actionable hints.

## Usage

```bash
# Basic usage
spawn ./my-game-archive.tar.gz

# Custom name and icon
spawn ./game-folder --name "My Awesome Game" --icon ./custom-logo.png
```

## Why Spawn?

Linux gaming often involves downloading standalone archives (like from "Linux Gaming" websites) that require manual setup. **Spawn** was created to solve this exact frustration.

Instead of manually extracting files, hunting for binaries, and writing `.desktop` files from scratch, Spawn automates the entire process. It makes setting up new games **80% faster**—you just run the script, and the game is ready in your launcher.

## Installation

```bash
cargo install --path .
```
