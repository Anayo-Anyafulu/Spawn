mod config;
mod discovery;
mod installation;
mod utils;
mod steam;
mod game_type;
mod proton;
mod windows;
mod archive;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use colored::*;
use dialoguer::{FuzzySelect, Input, Confirm, theme::ColorfulTheme};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use std::fs;

use crate::config::{load_config, save_config, Config};
use crate::discovery::{discover_executable, discover_icon};
use crate::installation::install_appimage;
use crate::steam::add_to_steam;
use crate::utils::{format_game_name, generate_desktop_entry, resolve_fuzzy_path, set_executable_permission};
use crate::game_type::{detect_game_type, GameType};
use crate::archive::extract_archive_unified;
use crate::windows::{install_windows_game, setup_portable_windows_game};

#[derive(Parser, Debug)]
#[command(author, version, about = "Linux CLI for installing and integrating non-Steam games")]
struct Args {
    /// Path to the game folder, archive, or installer
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

    /// Add the game to Steam as a Non-Steam Game
    #[arg(short, long)]
    steam: bool,

    /// Auto mode - no prompts, use safe defaults
    #[arg(long)]
    auto: bool,
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

    match args.path {
        Some(path) => {
            println!("\n{} {} v{}\n", "🎮".bold(), "Spawn".bold().cyan(), env!("CARGO_PKG_VERSION"));
            if args.dry_run {
                println!("{} Running in DRY RUN mode. No changes will be made.\n", "⚠".yellow().bold());
            }
            let input_path = resolve_fuzzy_path(&path, &config.search_dir)?;
            handle_installation(input_path, args.name, args.icon, &config, args.dry_run, args.steam)
        }
        None => {
            run_interactive_mode(&config)
        }
    }
}

fn run_interactive_mode(config: &Config) -> Result<()> {
    println!("\n{} Welcome to {} v{}\n", "🎮".bold(), "Spawn".bold().cyan(), env!("CARGO_PKG_VERSION"));
    
    let mut search_dirs = vec![config.search_dir.clone()];
    if let Some(home) = dirs_next::home_dir() {
        let games_packed = home.join("Games packed");
        if games_packed.exists() && !search_dirs.contains(&games_packed) {
            search_dirs.push(games_packed);
        }
        let games_dir = home.join("Games");
        if games_dir.exists() && !search_dirs.contains(&games_dir) {
            search_dirs.push(games_dir);
        }
    }

    let mut entries = Vec::new();
    for dir in &search_dirs {
        if let Ok(read_dir) = fs::read_dir(dir) {
            for entry in read_dir.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                if name.starts_with('.') || name.ends_with(".aria2") || name.ends_with(".part") || name.ends_with(".tmp") {
                    continue;
                }
                entries.push(path);
            }
        }
    }

    if entries.is_empty() {
        return Err(anyhow!("{} No games found in {:?}\nHint: Put your game folders or archives in your Downloads folder or use --set-search-dir", "✖".red(), search_dirs));
    }

    // Sort entries by name
    entries.sort_by_key(|a| a.file_name().unwrap_or_default().to_os_string());

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a game to install (type to search)")
        .items(&entries.iter().map(|e| e.file_name().unwrap().to_string_lossy()).collect::<Vec<_>>())
        .default(0)
        .interact_opt()?;

    if let Some(index) = selection {
        let input_path = &entries[index];
        
        println!("{} Selected: {}", "✔".green(), input_path.file_name().unwrap().to_string_lossy().bold());

        let game_name: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter Game Name (or press Enter for default)")
            .default(input_path.file_stem().unwrap().to_string_lossy().to_string())
            .interact_text()?;

        let add_to_steam = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Add to Steam?")
            .default(true)
            .interact()?;

        handle_installation(input_path.clone(), Some(game_name), None, config, false, add_to_steam)
    } else {
        println!("{} Installation cancelled.", "▶".cyan());
        Ok(())
    }
}

fn handle_installation(
    input_path: PathBuf,
    name_override: Option<String>,
    icon_override: Option<PathBuf>,
    config: &Config,
    dry_run: bool,
    steam: bool,
) -> Result<()> {
    let input_path = input_path.canonicalize().context("Failed to resolve input path")?;

    if !input_path.exists() {
        return Err(anyhow!("{} Path does not exist: {:?}", "✖".red(), input_path));
    }

    // Detect game type
    let game_info = detect_game_type(&input_path)?;
    
    let game_name = name_override
        .unwrap_or(game_info.name);
    let game_name = format_game_name(&game_name);

    println!("\n{} Preparing: {}\n", "▶".cyan(), game_name.bold());

    // Create install directory if needed
    if !dry_run && !config.install_dir.exists() {
        fs::create_dir_all(&config.install_dir)
            .context("Failed to create install directory")?;
    }

    // Handle based on game type
    let (executable, game_dir, needs_proton) = match game_info.game_type {
        GameType::WindowsInstaller => {
            if dry_run {
                println!("{} Would install Windows game with Proton", "▶".cyan());
                (PathBuf::from("would_be_exe"), config.install_dir.clone(), true)
            } else {
                let exe = install_windows_game(&input_path, &config.install_dir, &game_name)?;
                let dir = exe.parent().unwrap().to_path_buf();
                (exe, dir, true)
            }
        },
        
        GameType::LinuxArchive => {
            let target_dir = config.install_dir.join(&game_name.replace(' ', "_"));
            
            if !dry_run {
                fs::create_dir_all(&target_dir)?;
                
                if input_path.to_string_lossy().ends_with(".AppImage") {
                    install_appimage(&input_path, &config.install_dir, dry_run)?;
                } else {
                    extract_archive_unified(&input_path, &target_dir)?;
                }
            } else {
                println!("{} Would extract archive to {:?}", "▶".cyan(), target_dir);
            }
            
            let executable = if !dry_run {
                discover_executable(&target_dir)?
            } else {
                PathBuf::from("would_be_executable")
            };
            
            (executable, target_dir, false)
        },
        
        GameType::PortableLinux => {
            let executable = discover_executable(&input_path)?;
            (executable, input_path.clone(), false)
        },
        
        GameType::PortableWindows => {
            if dry_run {
                println!("{} Would setup portable Windows game with Proton", "▶".cyan());
                (PathBuf::from("would_be_exe"), input_path.clone(), true)
            } else {
                let exe = setup_portable_windows_game(&input_path)?;
                (exe, input_path.clone(), true)
            }
        },
    };

    if !dry_run {
        println!("{} Found executable: {:?}", "✔".green(), executable.file_name().unwrap_or_default());
    }

    // Set executable permissions (for Linux binaries)
    if !needs_proton && !dry_run {
        set_executable_permission(&executable)?;
        println!("{} Fixed executable permissions", "✔".green());
    }

    // Find icon
    let icon = if let Some(icon_path) = icon_override {
        Some(icon_path)
    } else if !dry_run {
        let found = discover_icon(&game_dir);
        if found.is_none() && needs_proton {
            // For Windows games, if no icon found in game dir, try searching the prefix root
            if let Some(prefix) = game_dir.parent() {
                discover_icon(prefix)
            } else {
                found
            }
        } else {
            found
        }
    } else {
        None
    };

    if let Some(ref i) = icon {
        println!("{} Found icon: {:?}", "✔".green(), i.file_name().unwrap());
    }

    // Generate desktop shortcuts
    if !dry_run {
        let desktop_files = generate_desktop_entry(&game_dir, &executable, &game_name, icon.as_deref())?;
        for df in desktop_files {
            println!("{} Created shortcut: {:?}", "✔".green(), df.file_name().unwrap_or_default());
        }
    } else {
        println!("{} Would create desktop shortcuts", "▶".cyan());
    }

    // Add to Steam
    if steam {
        if dry_run {
            println!("{} Would add to Steam", "▶".cyan());
        } else {
            match add_to_steam(&game_name, &executable, icon.as_deref(), needs_proton) {
                Ok(_) => {
                    println!("{} Added to Steam library", "✔".green());
                },
                Err(e) => {
                    println!("{} Failed to add to Steam: {}", "⚠".yellow(), e);
                    println!("  (The game is still installed and can be launched normally)");
                }
            }
        }
    }

    println!("\n{} {} is ready to play! {}\n", "🎮".bold(), game_name.bold().green(), "✨".bold());

    if let Some(new_version) = check_for_updates() {
        println!("✨ A new version of Spawn (v{}) is available!", new_version.bold().yellow());
        println!("   Run 'spawn --update' to update.\n");
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
