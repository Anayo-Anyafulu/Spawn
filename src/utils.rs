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

pub fn resolve_fuzzy_path(input: &Path, search_dir: &Path) -> Result<PathBuf> {
    if input.exists() {
        return Ok(input.to_path_buf());
    }

    let input_str = input.to_string_lossy().to_lowercase();
    
    let mut matches = Vec::new();
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

    match matches.len() {
        0 => Err(anyhow!("{} No file or directory found matching \"{}\" in {:?}", "✖".red(), input.display(), search_dir)),
        1 => {
            let matched = matches.remove(0);
            println!("{} Found matching path in {:?}: {:?}", "✔".green(), search_dir.file_name().unwrap_or_default(), matched.file_name().unwrap_or_default());
            Ok(matched)
        }
        _ => {
            println!("{} Multiple matches found for \"{}\" in {:?}:", "▶".cyan(), input.display(), search_dir);
            for (i, m) in matches.iter().enumerate() {
                println!("  {}. {:?}", i + 1, m.file_name().unwrap_or_default());
            }
            println!("{} Please enter the number of the correct file (or press Enter to cancel):", "▶".cyan());

            let mut choice = String::new();
            std::io::stdin().read_line(&mut choice).context("Failed to read input")?;
            let choice = choice.trim();

            if choice.is_empty() {
                return Err(anyhow!("{} Operation cancelled by user", "✖".red()));
            }

            let index: usize = choice.parse::<usize>().map_err(|_| anyhow!("{} Invalid selection", "✖".red()))?;
            if index == 0 || index > matches.len() {
                return Err(anyhow!("{} Selection out of range", "✖".red()));
            }

            let matched = matches.remove(index - 1);
            println!("{} Selected: {:?}", "✔".green(), matched.file_name().unwrap_or_default());
            Ok(matched)
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
