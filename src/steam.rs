use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use steam_shortcuts_util::{parse_shortcuts, shortcuts_to_bytes, Shortcut};
use colored::Colorize;

pub fn add_to_steam(game_name: &str, executable: &Path, icon: Option<&Path>) -> Result<()> {
    let shortcuts_path = find_shortcuts_vdf()?;
    println!("{} Found Steam shortcuts at: {:?}", "▶".cyan(), shortcuts_path);

    let content = fs::read(&shortcuts_path).context("Failed to read shortcuts.vdf")?;
    let mut shortcuts = parse_shortcuts(&content)
        .map_err(|e| anyhow!("Failed to parse shortcuts.vdf: {:?}", e))?;

    // Check if already exists
    if shortcuts.iter().any(|s| s.app_name == game_name) {
        println!("{} Game already exists in Steam shortcuts.", "⚠".yellow());
        return Ok(());
    }

    let new_shortcut = Shortcut {
        app_name: game_name,
        exe: executable.to_str().unwrap_or_default(),
        start_dir: executable.parent().and_then(|p| p.to_str()).unwrap_or_default(),
        icon: icon.and_then(|p| p.to_str()).unwrap_or_default(),
        shortcut_path: "",
        launch_options: "",
        is_hidden: false,
        allow_desktop_config: true,
        allow_overlay: true,
        open_vr: 0,
        dev_kit: 0,
        dev_kit_game_id: "",
        last_play_time: 0,
        tags: Vec::new(),
        app_id: 0,
        order: "",
        dev_kit_overrite_app_id: 0,
    };

    shortcuts.push(new_shortcut);

    let new_content = shortcuts_to_bytes(&shortcuts);
    fs::write(&shortcuts_path, new_content).context("Failed to write shortcuts.vdf")?;

    println!("{} Added {} to Steam! (Restart Steam to see changes)", "✔".green(), game_name);
    Ok(())
}

fn find_shortcuts_vdf() -> Result<PathBuf> {
    let steam_dir = dirs_next::home_dir()
        .map(|h| h.join(".steam/steam/userdata"))
        .ok_or_else(|| anyhow!("Could not find Steam directory"))?;

    if !steam_dir.exists() {
        return Err(anyhow!("Steam userdata directory not found at {:?}", steam_dir));
    }

    // Find the first numeric directory (User ID)
    let entries = fs::read_dir(&steam_dir)?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name.chars().all(|c| c.is_numeric()) {
                let shortcuts_path = path.join("config/shortcuts.vdf");
                if shortcuts_path.exists() {
                    return Ok(shortcuts_path);
                }
            }
        }
    }

    Err(anyhow!("Could not find shortcuts.vdf in {:?}", steam_dir))
}
