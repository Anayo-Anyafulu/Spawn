use anyhow::{Context, Result, anyhow};
use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about = "Turns a Linux game archive into a runnable desktop application")]
struct Args {
    /// Path to the game folder or .tar.gz archive
    path: PathBuf,

    /// Override the game name
    #[arg(short, long)]
    name: Option<String>,

    /// Path to a custom icon
    #[arg(short, long)]
    icon: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("▶ Spawn v{}", env!("CARGO_PKG_VERSION"));

    let input_path = resolve_fuzzy_path(&args.path)?;
    let input_path = input_path.canonicalize().context("Failed to resolve input path")?;

    if !input_path.exists() {
        return Err(anyhow!("✖ Path does not exist: {:?}\nHint: Ensure the path is correct and accessible", input_path));
    }

    println!("▶ Installing game from: {:?}", input_path);

    let game_dir = if input_path.is_file() {
        extract_archive(&input_path)?
    } else {
        input_path
    };

    let executable = discover_executable(&game_dir)?;
    println!("✔ Discovered executable: {:?}", executable.file_name().unwrap_or_default());

    set_executable_permission(&executable)?;
    println!("✔ Fixed executable permissions");

    let icon = if let Some(icon_path) = args.icon {
        Some(icon_path)
    } else {
        discover_icon(&game_dir)
    };
    if let Some(ref i) = icon {
        println!("✔ Found icon: {:?}", i.file_name().unwrap_or_default());
    }

    let game_name = args.name.as_deref().unwrap_or_else(|| {
        game_dir.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown Game")
    });

    let desktop_file = generate_desktop_entry(&game_dir, &executable, game_name, icon.as_deref())?;
    println!("✔ Desktop shortcut created: {:?}", desktop_file.file_name().unwrap_or_default());

    println!("\n🎮 {} is ready to play!", game_name);

    Ok(())
}

fn extract_archive(archive_path: &Path) -> Result<PathBuf> {
    let _file_name = archive_path.file_name().ok_or_else(|| anyhow!("Invalid archive path"))?;
    let parent_dir = archive_path.parent().ok_or_else(|| anyhow!("No parent directory"))?;
    
    // Create a directory for extraction if it's just a file
    let stem = archive_path.file_stem().ok_or_else(|| anyhow!("Invalid file name"))?;
    // Handle .tar.gz double extension
    let dir_name = if stem.to_string_lossy().ends_with(".tar") {
        Path::new(stem).file_stem().ok_or_else(|| anyhow!("Invalid tar.gz name"))?
    } else {
        stem
    };
    
    let target_dir = parent_dir.join(dir_name);
    if target_dir.exists() {
        println!("✔ Using existing directory: {:?}", target_dir);
        return Ok(flatten_if_needed(target_dir));
    }

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).context("Failed to create extraction directory")?;
    }

    println!("▶ Extracting {:?} to {:?}", archive_path, target_dir);

    let status = Command::new("tar")
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(&target_dir)
        .status()
        .context("Failed to execute tar command")?;

    if !status.success() {
        return Err(anyhow!("tar command failed with status: {:?}\nHint: Ensure tar is installed and the archive is valid", status));
    }

    println!("✔ Extracted game files");

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

fn resolve_fuzzy_path(input: &Path) -> Result<PathBuf> {
    if input.exists() {
        return Ok(input.to_path_buf());
    }

    let input_str = input.to_string_lossy().to_lowercase();
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    
    let mut matches = Vec::new();
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        
        if file_name.contains(&input_str) {
            matches.push(path);
        }
    }

    match matches.len() {
        0 => Err(anyhow!("✖ No file or directory found matching \"{}\"", input.display())),
        1 => {
            let matched = matches.remove(0);
            println!("✔ Found matching path: {:?}", matched.file_name().unwrap_or_default());
            Ok(matched)
        }
        _ => {
            let mut msg = format!("✖ Multiple matches found for \"{}\":\n", input.display());
            for m in matches {
                msg.push_str(&format!("  - {:?}\n", m.file_name().unwrap_or_default()));
            }
            msg.push_str("Hint: Please be more specific");
            Err(anyhow!(msg))
        }
    }
}

fn generate_desktop_entry(game_dir: &Path, executable: &Path, game_name: &str, icon: Option<&Path>) -> Result<PathBuf> {
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

    let desktop_dir = dirs_next::home_dir()
        .map(|h| h.join(".local/share/applications"))
        .ok_or_else(|| anyhow!("Could not find home directory"))?;

    if !desktop_dir.exists() {
        fs::create_dir_all(&desktop_dir).context("Failed to create applications directory")?;
    }

    let desktop_file_name = format!("{}.desktop", game_name.to_lowercase().replace(' ', "-"));
    let desktop_file_path = desktop_dir.join(desktop_file_name);

    fs::write(&desktop_file_path, content).context("Failed to write .desktop file")?;

    Ok(desktop_file_path)
}
