mod config;
mod discovery;
mod installation;
mod utils;
mod steam;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::fs;

use crate::config::{load_config, save_config};
use crate::discovery::{discover_executable, discover_icon};
use crate::installation::{extract_archive, install_appimage};
use crate::steam::add_to_steam;
use crate::utils::{format_game_name, generate_desktop_entry, resolve_fuzzy_path, set_executable_permission};

#[derive(Parser, Debug)]
#[command(author, version, about = "Turns a Linux game archive into a runnable desktop application")]
struct Args {
    /// Path to the game folder or .tar.gz archive
    path: Option<PathBuf>,

    /// Override the game name
    #[arg(short, long)]
    name: Option<String>,

    /// Path to a custom icon
    #[arg(short, long)]
    icon: Option<PathBuf>,

    /// Set the default search directory
    #[arg(long)]
    set_search_dir: Option<PathBuf>,

    /// Set the default install directory
    #[arg(long)]
    set_install_dir: Option<PathBuf>,

    /// Show what would happen without making any changes
    #[arg(long)]
    dry_run: bool,

    /// Update Spawn to the latest version from GitHub
    #[arg(long)]
    update: bool,

    /// Uninstall a game and remove its shortcuts
    #[arg(long)]
    uninstall: Option<String>,

    /// Add the game to Steam as a Non-Steam Game (Experimental)
    #[arg(long)]
    steam: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut config = load_config();

    if let Some(new_dir) = args.set_search_dir {
        let abs_dir = new_dir.canonicalize().context("Failed to resolve new search directory")?;
        config.search_dir = abs_dir;
        save_config(&config)?;
        println!("✔ Search directory updated to: {:?}", config.search_dir);
        return Ok(());
    }

    if let Some(new_dir) = args.set_install_dir {
        let abs_dir = new_dir.canonicalize().context("Failed to resolve new install directory")?;
        config.install_dir = abs_dir;
        save_config(&config)?;
        println!("{} Install directory updated to: {:?}", "✔".green(), config.install_dir);
        return Ok(());
    }

    if args.update {
        return update_spawn();
    }

    if let Some(game_to_uninstall) = args.uninstall {
        return uninstall_game(&game_to_uninstall, &config.install_dir, args.dry_run);
    }

    let input = args.path.ok_or_else(|| anyhow!("{} No path provided\nHint: Use 'spawn <PATH>' or 'spawn <PARTIAL_NAME>'", "✖".red()))?;

    println!("{} {} v{}", "▶".cyan(), "Spawn".bold(), env!("CARGO_PKG_VERSION"));

    if args.dry_run {
        println!("{} Running in DRY RUN mode. No changes will be made.", "⚠".yellow().bold());
    }

    let input_path = resolve_fuzzy_path(&input, &config.search_dir)?;
    let input_path = input_path.canonicalize().context("Failed to resolve input path")?;

    if !input_path.exists() {
        return Err(anyhow!("{} Path does not exist: {:?}\nHint: Ensure the path is correct and accessible", "✖".red(), input_path));
    }

    println!("{} Installing game from: {:?}", "▶".cyan(), input_path);

    let game_dir = if input_path.is_file() {
        println!("{} Where should I install this? [Default: {:?}]", "▶".cyan(), config.install_dir);
        println!("  (Press Enter to use default, or type a new path)");
        
        let mut input_dir = String::new();
        std::io::stdin().read_line(&mut input_dir).context("Failed to read input")?;
        let input_dir = input_dir.trim();
        
        let target_parent = if input_dir.is_empty() {
            config.install_dir.clone()
        } else {
            PathBuf::from(input_dir)
        };

        if !args.dry_run && !target_parent.exists() {
            fs::create_dir_all(&target_parent).context("Failed to create install directory")?;
        }

        if input_path.to_string_lossy().ends_with(".AppImage") {
            install_appimage(&input_path, &target_parent, args.dry_run)?
        } else {
            extract_archive(&input_path, &target_parent, args.dry_run)?
        }
    } else {
        input_path
    };

    let (executable, icon) = if args.dry_run && !game_dir.exists() {
        println!("{} Would discover executable and icon inside the archive", "▶".cyan());
        (PathBuf::from("would_be_executable"), None)
    } else {
        let executable = discover_executable(&game_dir)?;
        println!("{} Discovered executable: {:?}", "✔".green(), executable.file_name().unwrap_or_default());

        let icon = if let Some(icon_path) = args.icon {
            Some(icon_path)
        } else {
            discover_icon(&game_dir)
        };
        if let Some(ref i) = icon {
            let name = i.file_name().unwrap_or_else(|| std::ffi::OsStr::new(""));
            println!("{} Found icon: {:?}", "✔".green(), name);
        }
        (executable, icon)
    };

    if !args.dry_run {
        set_executable_permission(&executable)?;
        println!("{} Fixed executable permissions", "✔".green());
    } else if game_dir.exists() {
        println!("{} Would fix executable permissions", "▶".cyan());
    }

    let game_name = args.name.as_deref().unwrap_or_else(|| {
        game_dir.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown Game")
    });
    let game_name = format_game_name(&game_name);

    if !args.dry_run {
        let desktop_files = generate_desktop_entry(&game_dir, &executable, &game_name, icon.as_deref())?;
        for df in desktop_files {
            println!("{} Shortcut created: {:?}", "✔".green(), df.file_name().unwrap_or_default());
        }
    } else {
        println!("{} Would create desktop shortcuts for {}", "▶".cyan(), game_name.bold());
    }

    if args.steam {
        if let Err(e) = add_to_steam(&game_name, &executable, icon.as_deref()) {
            println!("{} Failed to add to Steam: {:?}", "⚠".yellow(), e);
        }
    }

    println!("\n🎮 {} is ready to play!", game_name.bold().green());

    if let Some(new_version) = check_for_updates() {
        println!("\n✨ A new version of Spawn (v{}) is available!", new_version.bold().yellow());
        println!("   Run 'spawn --update' to update.");
    }

    Ok(())
}

fn check_for_updates() -> Option<String> {
    let url = "https://raw.githubusercontent.com/Anayo-Anyafulu/Spawn/master/Cargo.toml";
    let agent = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(1))
        .timeout_connect(Duration::from_secs(1))
        .build();

    let response = match agent.get(url).call() {
        Ok(r) => r,
        Err(_) => return None,
    };
    let body = response.into_string().ok()?;

    for line in body.lines() {
        if line.trim().starts_with("version =") {
            let version = line.split('"').nth(1)?;
            if version != env!("CARGO_PKG_VERSION") {
                return Some(version.to_string());
            }
            break;
        }
    }
    None
}

fn update_spawn() -> Result<()> {
    println!("{} Updating Spawn...", "▶".cyan());
    let status = Command::new("git")
        .arg("pull")
        .status()
        .context("Failed to execute git pull")?;

    if !status.success() {
        return Err(anyhow!("{} git pull failed", "✖".red()));
    }

    let status = Command::new("cargo")
        .arg("install")
        .arg("--path")
        .arg(".")
        .status()
        .context("Failed to execute cargo install")?;

    if !status.success() {
        return Err(anyhow!("{} cargo install failed", "✖".red()));
    }

    println!("{} Spawn has been updated successfully!", "✔".green().bold());
    Ok(())
}

fn uninstall_game(game_name: &str, install_dir: &Path, dry_run: bool) -> Result<()> {
    println!("{} Uninstalling {}...", "▶".cyan(), game_name.bold());
    
    let formatted_name = format_game_name(game_name);
    let dir_name = game_name.replace(' ', "_");
    let game_path = install_dir.join(&dir_name);
    
    let mut found = false;
    if game_path.exists() {
        found = true;
        if dry_run {
            println!("{} Would remove directory: {:?}", "▶".cyan(), game_path);
        } else {
            println!("{} Removing directory: {:?}", "▶".cyan(), game_path);
            fs::remove_dir_all(&game_path).context("Failed to remove game directory")?;
        }
    }

    let desktop_file_name = format!("{}.desktop", formatted_name.to_lowercase().replace(' ', "-"));
    
    let app_dir = dirs_next::home_dir().map(|h| h.join(".local/share/applications"));
    if let Some(path) = app_dir.map(|d| d.join(&desktop_file_name)) {
        if path.exists() {
            found = true;
            if dry_run {
                println!("{} Would remove shortcut: {:?}", "▶".cyan(), path);
            } else {
                fs::remove_file(&path).context("Failed to remove application shortcut")?;
                println!("{} Removed shortcut: {:?}", "✔".green(), path.file_name().unwrap());
            }
        }
    }

    let desktop_dir = dirs_next::home_dir().map(|h| h.join("Desktop"));
    if let Some(path) = desktop_dir.map(|d| d.join(&desktop_file_name)) {
        if path.exists() {
            found = true;
            if dry_run {
                println!("{} Would remove desktop shortcut: {:?}", "▶".cyan(), path);
            } else {
                fs::remove_file(&path).context("Failed to remove desktop shortcut")?;
                println!("{} Removed desktop shortcut: {:?}", "✔".green(), path.file_name().unwrap());
            }
        }
    }

    if !found {
        println!("{} No installation found for {}", "⚠".yellow(), game_name);
    } else {
        println!("{} {} has been uninstalled.", "✔".green().bold(), formatted_name);
    }

    Ok(())
}
