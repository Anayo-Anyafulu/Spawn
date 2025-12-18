use anyhow::{Context, Result, anyhow};
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use walkdir::WalkDir;

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
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    search_dir: PathBuf,
    install_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            search_dir: dirs_next::download_dir().unwrap_or_else(|| PathBuf::from(".")),
            install_dir: dirs_next::home_dir().map(|h| h.join("Games")).unwrap_or_else(|| PathBuf::from(".")),
        }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs_next::config_dir()
        .ok_or_else(|| anyhow!("Could not find config directory"))?
        .join("spawn");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir.join("config.toml"))
}

fn load_config() -> Config {
    let path = match get_config_path() {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };
    
    fs::read_to_string(path)
        .and_then(|s| toml::from_str(&s).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
        .unwrap_or_else(|_| Config::default())
}

fn save_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    let s = toml::to_string(config).map_err(|e| anyhow!("Failed to serialize config: {}", e))?;
    fs::write(path, s).context("Failed to write config file")
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
        // Ask for install location
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

        extract_archive(&input_path, &target_parent, args.dry_run)?
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
            println!("{} Found icon: {:?}", "✔".green(), i.file_name().unwrap_or_default());
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

    println!("\n🎮 {} is ready to play!", game_name.bold().green());

    // Check for updates in the background (silently)
    if let Some(new_version) = check_for_updates() {
        println!("\n✨ A new version of Spawn (v{}) is available!", new_version.bold().yellow());
        println!("   Run 'git pull' in your Spawn folder to update.");
    }

    Ok(())
}

fn extract_archive(archive_path: &Path, install_dir: &Path, dry_run: bool) -> Result<PathBuf> {
    let _file_name = archive_path.file_name().ok_or_else(|| anyhow!("Invalid archive path"))?;
    let _parent_dir = archive_path.parent().ok_or_else(|| anyhow!("No parent directory"))?;
    
    // Create a directory for extraction if it's just a file
    let stem = archive_path.file_stem().ok_or_else(|| anyhow!("Invalid file name"))?;
    let stem_str = stem.to_string_lossy();
    
    // Handle various tar extensions (.tar.gz, .tar.xz, .tar.bz2, etc.) and .zip
    let dir_name = if stem_str.ends_with(".tar") {
        Path::new(stem_str.as_ref()).file_stem().ok_or_else(|| anyhow!("Invalid tar archive name"))?
    } else {
        stem
    };
    
    let target_dir = install_dir.join(dir_name);
    if target_dir.exists() {
        println!("{} {:?} is already installed.", "⚠".yellow().bold(), dir_name);
        println!("  Do you want to overwrite it? [y/N]");
        
        let mut confirm = String::new();
        std::io::stdin().read_line(&mut confirm).context("Failed to read input")?;
        if confirm.trim().to_lowercase() != "y" {
            println!("{} Using existing directory.", "✔".green());
            return Ok(flatten_if_needed(target_dir));
        }

        if !dry_run {
            fs::remove_dir_all(&target_dir).context("Failed to remove existing directory")?;
        } else {
            println!("{} Would remove existing directory", "▶".cyan());
        }
    }

    if !dry_run {
        fs::create_dir_all(&target_dir).context("Failed to create extraction directory")?;
    }

    if dry_run {
        println!("{} Would extract {:?} to {:?}", "▶".cyan(), archive_path, target_dir);
        return Ok(target_dir);
    }

    println!("{} Extracting {:?}...", "▶".cyan(), archive_path.file_name().unwrap_or_default());
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
        .template("{spinner:.cyan} {msg}")?);
    pb.set_message("Extracting files...");
    pb.enable_steady_tick(Duration::from_millis(100));

    let is_zip = archive_path.to_string_lossy().ends_with(".zip");
    
    let status = if is_zip {
        Command::new("unzip")
            .arg("-q") // quiet
            .arg(archive_path)
            .arg("-d")
            .arg(&target_dir)
            .status()
            .context("Failed to execute unzip command. Hint: Ensure 'unzip' is installed.")?
    } else {
        // Use -xf instead of -xzf to let tar auto-detect compression (gz, xz, bz2, etc.)
        Command::new("tar")
            .arg("-xf")
            .arg(archive_path)
            .arg("-C")
            .arg(&target_dir)
            .status()
            .context("Failed to execute tar command")?
    };

    pb.finish_and_clear();

    if !status.success() {
        let hint = if archive_path.to_string_lossy().ends_with(".xz") {
            "\nHint: This is a .xz archive. Ensure you have 'xz-utils' or 'xz' installed."
        } else if is_zip {
            "\nHint: Ensure 'unzip' is installed and the archive is valid."
        } else {
            "\nHint: Ensure tar is installed and the archive is valid."
        };
        return Err(anyhow!("{} Extraction failed (exit code: {:?}){}", "✖".red(), status.code(), hint));
    }

    println!("{} Extracted game files", "✔".green());

    Ok(flatten_if_needed(target_dir))
}

fn flatten_if_needed(dir: PathBuf) -> PathBuf {
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e.filter_map(|e| e.ok()).collect::<Vec<_>>(),
        Err(_) => return dir,
    };

    if entries.len() == 1 && entries[0].path().is_dir() {
        let inner_dir = entries[0].path();
        println!("✔ Detected nested directory, using: {:?}", inner_dir);
        inner_dir
    } else {
        dir
    }
}

fn format_game_name(name: &str) -> String {
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

fn discover_executable(game_dir: &Path) -> Result<PathBuf> {
    let mut candidates = Vec::new();

    for entry in WalkDir::new(game_dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            
            // Heuristics:
            // 1. Common launcher scripts in root
            if path.parent() == Some(game_dir) && (file_name == "start.sh" || file_name == "run.sh" || file_name == "launcher.sh") {
                return Ok(path.to_path_buf());
            }

            // 2. Ends with .x86_64 or .x86
            if file_name.ends_with(".x86_64") || file_name.ends_with(".x86") {
                if is_elf_binary(path) {
                    candidates.push(path.to_path_buf());
                }
            } else if !file_name.contains('.') {
                // 3. No extension and is not a common text/data file
                // Check if it's likely a binary (this is a simple heuristic)
                // Avoid common directories or files
                if !path.to_string_lossy().contains("/lib/") && !path.to_string_lossy().contains("/docs/") {
                     if is_elf_binary(path) {
                         candidates.push(path.to_path_buf());
                     }
                }
            }
        }
    }

    // Sort by depth (prefer shallower files) and then by name length
    candidates.sort_by_key(|p| (p.components().count(), p.file_name().map(|n| n.len()).unwrap_or(0)));

    candidates.into_iter().next().ok_or_else(|| anyhow!("No executable found in {:?}\nHint: This archive may not be a Linux build", game_dir))
}

fn discover_icon(game_dir: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    for entry in WalkDir::new(game_dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            if file_name.ends_with(".png") || file_name.ends_with(".svg") || file_name.ends_with(".ico") {
                let score = if file_name.contains("icon") || file_name.contains("logo") {
                    10
                } else {
                    1
                };
                candidates.push((score, path.to_path_buf()));
            }
        }
    }

    candidates.sort_by_key(|(s, p)| (-(*s as i32), p.components().count()));
    candidates.into_iter().next().map(|(_, p)| p)
}

fn set_executable_permission(executable: &Path) -> Result<()> {
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

fn is_elf_binary(path: &Path) -> bool {
    use std::io::Read;
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buffer = [0u8; 4];
    if file.read_exact(&mut buffer).is_err() {
        return false;
    }
    buffer == [0x7F, 0x45, 0x4C, 0x46]
}

fn resolve_fuzzy_path(input: &Path, search_dir: &Path) -> Result<PathBuf> {
    if input.exists() {
        return Ok(input.to_path_buf());
    }

    let input_str = input.to_string_lossy().to_lowercase();
    
    let mut matches = Vec::new();
    if let Ok(entries) = fs::read_dir(search_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
            
            // Filter out common temporary/meta files
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

fn generate_desktop_entry(game_dir: &Path, executable: &Path, game_name: &str, icon: Option<&Path>) -> Result<Vec<PathBuf>> {
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

    // 1. Applications Menu
    if let Some(app_dir) = dirs_next::home_dir().map(|h| h.join(".local/share/applications")) {
        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).context("Failed to create applications directory")?;
        }
        let app_path = app_dir.join(&desktop_file_name);
        fs::write(&app_path, &content).context("Failed to write .desktop file to applications")?;
        created_files.push(app_path);
    }

    // 2. Desktop
    if let Some(desktop_dir) = dirs_next::home_dir().map(|h| h.join("Desktop")) {
        if desktop_dir.exists() {
            let desktop_path = desktop_dir.join(&desktop_file_name);
            fs::write(&desktop_path, &content).context("Failed to write .desktop file to Desktop")?;
            created_files.push(desktop_path);
        }
    }

    Ok(created_files)
}

fn check_for_updates() -> Option<String> {
    let url = "https://raw.githubusercontent.com/Anayo-Anyafulu/Spawn/master/Cargo.toml";
    
    // 1 second timeout to ensure it doesn't hang offline
    let agent = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(1))
        .timeout_connect(Duration::from_secs(1))
        .build();

    let response = match agent.get(url).call() {
        Ok(r) => r,
        Err(_) => {
            return None;
        }
    };
    let body = response.into_string().ok()?;

    // Simple parsing of version = "x.y.z"
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
