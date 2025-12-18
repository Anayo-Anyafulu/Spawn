# Spawn 🎮

**Spawn** is a premium CLI tool that turns Linux game archives and AppImages into fully integrated desktop applications with a single command. 

No more manual extraction, hunting for binaries, or fixing permissions. Spawn handles the "boring stuff" so you can get straight to playing.

---

## 🚀 Quick Start

Just point Spawn at a game name or a file:

```bash
# Fuzzy search and install (checks your Downloads folder by default)
spawn "buckshot"

# Install from a specific file
spawn ./my-game-archive.tar.gz
```

---

## ✨ Core Features

- **📦 Universal Support**: Automatically handles `.tar.gz`, `.tar.xz`, `.tar.bz2`, `.zip`, and `.AppImage` files.
- **🔍 Smart Fuzzy Search**: Don't remember the full filename? Just type `spawn toy` to find `Toy_Soldiers_v1.2.zip`.
- **🎩 Title Case Magic**: Automatically converts ugly filenames like `annana_nene` into beautiful shortcut names like **Annana Nene**.
- **🧠 Intelligent Detection**:
    - **Executables**: Uses ELF header verification to find the real game binary, even if it's buried in subfolders.
    - **Icons**: Automatically finds and links the best game icon (`.png`, `.svg`, `.ico`).
- **🤝 Interactive & Safe**:
    - **Selection**: If multiple matches are found, you get to pick.
    - **Overwrite**: Prompts you before touching any existing installations.
    - **Dry Run**: Use `--dry-run` to see what Spawn *would* do without making changes.
- **🎨 Visual Polish**: Color-coded output and smooth progress spinners for a premium terminal experience.
- **🔄 Always Fresh**: 
    - **Update Checker**: Notifies you when a new version is available on GitHub.
    - **Self-Update**: Run `spawn --update` to pull and install the latest version automatically.

---

## ⚙️ Configuration

Spawn is ready to go out of the box, but you can customize it:

```bash
# Change where Spawn looks for games (default: ~/Downloads)
spawn --set-search-dir ~/Games/Downloads

# Change where games are installed (default: ~/Games)
spawn --set-install-dir ~/Storage/Games
```

---

## 🛠️ Installation

Ensure you have [Rust](https://rustup.rs/) installed, then run:

```bash
git clone https://github.com/Anayo-Anyafulu/Spawn.git
cd Spawn
cargo install --path .
```

---

## 💡 The Story Behind Spawn

I built **Spawn** because I was tired of the manual grind. 

Whenever I download games from sites like *itch.io*, they usually come as messy archives. The routine was always the same: extract, hunt for the executable, `chmod +x`, and manually create a shortcut. 

I wanted a way to just "open the file and play." Spawn automates that entire workflow, making it **80% faster** to get from download to gameplay.

---

## 🗺️ Roadmap (v2)

- [ ] **Dependency Doctor**: Automatically suggest missing Linux libraries.
- [ ] **Uninstaller**: One command to clean up game folders and shortcuts.
- [ ] **Steam Integration**: Add games as non-Steam shortcuts automatically.
- [ ] **Cover Art**: Auto-download high-quality icons and covers.
