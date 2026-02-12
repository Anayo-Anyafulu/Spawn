use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use colored::*;

pub fn format_game_name(name: &str) -> String {
    name.replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str().to_lowercase().as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn set_executable_permission(executable: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(executable)?;
        let mut perms = metadata.permissions();
        let mode = perms.mode();
        perms.set_mode(mode | 0o111);
        fs::set_permissions(executable, perms).context("Failed to set executable permissions")?;
    }
    Ok(())
}

use dialoguer::{FuzzySelect, theme::ColorfulTheme};

pub fn resolve_fuzzy_path(input: &Path, search_dir: &Path) -> Result<PathBuf> {
    if input.exists() {
        return Ok(input.to_path_buf());
    }

    let input_str = input.to_string_lossy().to_lowercase();
    
    let mut matches = Vec::new();

    // Check search_dir
    if let Ok(entries) = fs::read_dir(search_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            
            if file_name.ends_with(".aria2") || file_name.ends_with(".part") || file_name.ends_with(".tmp") {
                continue;
            }

            if file_name.contains(&input_str) {
                matches.push(path);
            }
        }
    }

    // Also check Games packed if it exists and we haven't found a single match yet
    if matches.len() != 1 {
        if let Some(home) = dirs_next::home_dir() {
            let games_packed = home.join("Games packed");
            if games_packed.exists() && games_packed != search_dir {
                if let Ok(entries) = fs::read_dir(&games_packed) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                        if file_name.contains(&input_str) && !matches.contains(&path) {
                            matches.push(path);
                        }
                    }
                }
            }
        }
    }

    match matches.len() {
        0 => Err(anyhow!("{} No file or directory found matching \"{}\"\nHint: Use 'spawn' without arguments to see all available games", "✖".red(), input.display())),
        1 => {
            let matched = matches.remove(0);
            println!("{} Found: {:?}", "✔".green(), matched.file_name().unwrap_or_default());
            Ok(matched)
        }
        _ => {
            let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Multiple matches for \"{}\". Pick one:", input.display()))
                .items(&matches.iter().map(|m| m.file_name().unwrap().to_string_lossy()).collect::<Vec<_>>())
                .default(0)
                .interact_opt()?;

            if let Some(index) = selection {
                let matched = matches.remove(index);
                println!("{} Selected: {:?}", "✔".green(), matched.file_name().unwrap_or_default());
                Ok(matched)
            } else {
                Err(anyhow!("{} Selection cancelled", "✖".red()))
            }
        }
    }
}

pub fn generate_desktop_entry(game_dir: &Path, executable: &Path, game_name: &str, icon: Option<&Path>) -> Result<Vec<PathBuf>> {
    let exec_path = executable.to_string_lossy();
    let working_dir = game_dir.to_string_lossy();

    let mut content = format!(
        "[Desktop Entry]\n\
        Type=Application\n\
        Name={}\n\
        Exec=\"{}\"\n\
        Path={}\n\
        Terminal=false\n\
        Categories=Game;\n",
        game_name, exec_path, working_dir
    );

    if let Some(icon_path) = icon {
        content.push_str(&format!("Icon={}\n", icon_path.to_string_lossy()));
    }

    let mut created_files = Vec::new();
    let desktop_file_name = format!("{}.desktop", game_name.to_lowercase().replace(' ', "-"));

    if let Some(app_dir) = dirs_next::home_dir().map(|h| h.join(".local/share/applications")) {
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).context("Failed to create applications directory")?;
        }
        let app_path = app_dir.join(&desktop_file_name);
        fs::write(&app_path, &content).context("Failed to write .desktop file to applications")?;
        created_files.push(app_path);
    }

    if let Some(desktop_dir) = dirs_next::home_dir().map(|h| h.join("Desktop")) {
        if desktop_dir.exists() {
            let desktop_path = desktop_dir.join(&desktop_file_name);
            fs::write(&desktop_path, &content).context("Failed to write .desktop file to Desktop")?;
            created_files.push(desktop_path);
        }
    }

    Ok(created_files)
}
