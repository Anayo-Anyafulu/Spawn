# Spawn 🎮

**Spawn** is a lightweight CLI tool designed to turn Linux game archives into fully integrated desktop applications with a single command.

No more manual extraction, searching for binaries, or manually creating `.desktop` files. Spawn handles the "boring stuff" so you can get straight to playing.

## Features

- **🚀 One-Command Setup**: Point Spawn at a `.tar.gz` archive or a game folder, and it does the rest.
- **🔍 Smart Discovery**: 
    - **Fuzzy Search**: Type partial names (e.g., `spawn toy`) and Spawn will find the matching file automatically.
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

## The Story Behind Spawn 💡

I built **Spawn** because I was tired of the manual grind. 

Whenever I download games from sites like *Linux Gaming*, they usually come as `.tar.gz` archives. The routine was always the same: extract the files, hunt through folders to find the actual executable, fix permissions, and finally run it. It felt slow, repetitive, and honestly, a bit annoying.

I wanted a way to just "open the file and play." So, I wrote this script to automate the entire workflow. Now, instead of manually doing everything, I just run one command and the game is ready. It makes the whole process at least **80% faster**—I get to spend less time in the terminal and more time actually playing.

## Installation

```bash
cargo install --path .
```
