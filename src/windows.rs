use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use colored::*;

use crate::proton::{find_proton, get_proton_executable, is_flatpak_steam};

/// Install a Windows game using Proton
pub fn install_windows_game(installer_path: &Path, install_dir: &Path, game_name: &str) -> Result<PathBuf> {
    println!("\n{} Installing Windows game using Proton", "▶".cyan().bold());
    
    // Find Proton
    let proton_dir = find_proton()?;
    let _proton_exe = get_proton_executable(&proton_dir)?;
    
    // Create game directory
    let game_dir = install_dir.join(game_name.replace(' ', "_"));
    if !game_dir.exists() {
        fs::create_dir_all(&game_dir)
            .context("Failed to create game directory")?;
    }
    
    println!("{} Game will be installed to: {:?}", "▶".cyan(), game_dir);
    println!("{} Running installer...", "▶".cyan());
    println!("  (This may take a few minutes. Follow the installer prompts)");
    
    // Set up Wine prefix for this game
    let wine_prefix = game_dir.join(".wine");
    if !wine_prefix.exists() {
        fs::create_dir_all(&wine_prefix)
            .context("Failed to create Wine prefix directory")?;
    }
    
    // Pre-create common installation folders to help some repacks
    let drive_c = wine_prefix.join("pfx/drive_c");
    if !drive_c.exists() {
        // Run a dummy command to let Proton initialize the prefix structure
        // This is necessary because we need the pfx/drive_c structure to exist
    }
    
    // Create folders explicitly just in case
    for folder in &["Games", "Program Files", "Program Files (x86)"] {
        let p = drive_c.join(folder);
        if !p.exists() {
            let _ = fs::create_dir_all(&p);
        }
    }
    
    // Check if Steam is Flatpak
    let is_flatpak = is_flatpak_steam();
    
    let status = if is_flatpak {
        println!("{} Detected Flatpak Steam - using flatpak run", "▶".cyan());
        
        // For Flatpak with --filesystem=home, absolute paths from the host should match exactly inside.
        // Wine/Proton require absolute paths for WINEPREFIX.
        let internal_path = installer_path.display().to_string();
        let internal_prefix = wine_prefix.display().to_string();

        // For Flatpak Steam, we need to run the installer through flatpak
        // We use a shell script snippet to find the proton binary inside the sandbox.
        // Internal Flatpak Steam directory is usually at /var/data/Steam.
        Command::new("flatpak")
            .arg("run")
            .arg("--filesystem=home")
            .arg("--command=sh")
            .arg("com.valvesoftware.Steam")
            .arg("-c")
            .arg(format!(
                "PROTON_EXE=$(find /var/data/Steam/steamapps/common -name proton -type f | head -n 1); \
                 if [ -z \"$PROTON_EXE\" ]; then \
                   PROTON_EXE=$(find ~/.local/share/Steam/steamapps/common -name proton -type f | head -n 1); \
                 fi; \
                 if [ -z \"$PROTON_EXE\" ]; then \
                   echo 'Error: Proton binary not found inside Steam sandbox'; exit 1; \
                 fi; \
                 PROTON_DIR=\"$(dirname \"$PROTON_EXE\")\"; \
                 mkdir -p \"{0}\"; \
                 export WINEPREFIX='{0}'; \
                 export STEAM_COMPAT_DATA_PATH='{0}'; \
                 export STEAM_COMPAT_CLIENT_INSTALL_PATH='/var/data/Steam'; \
                 export STEAM_COMPAT_STAGING_PATH=\"$PROTON_DIR\"; \
                 \"$PROTON_EXE\" run \"{1}\"",
                internal_prefix,
                internal_path
            ))
            .status()
            .context("Failed to run installer with Proton via Flatpak")?
    } else {
        // For native Steam, run Proton directly
        let proton_exe = get_proton_executable(&proton_dir)?;
        Command::new(proton_exe)
            .arg("run")
            .arg(installer_path)
            .env("WINEPREFIX", &wine_prefix)
            .env("STEAM_COMPAT_DATA_PATH", &wine_prefix)
            .status()
            .context("Failed to run installer with Proton")?
    };
    
    if !status.success() {
        return Err(anyhow!(
            "Installer failed with exit code: {:?}",
            status.code()
        ));
    }
    
    println!("{} Installation completed", "✔".green());
    
    // Verify installation size
    let prefix_size = get_dir_size(&wine_prefix.join("pfx/drive_c"))?;
    if prefix_size < 500 * 1024 * 1024 { // Less than 500MB is suspicious for a repack
        println!("\n{} Warning: Installation directory is very small ({:.2} MB).", "⚠".yellow().bold(), prefix_size as f64 / (1024.0 * 1024.0));
        println!("  The installation may have failed secretly (e.g., ISDone.dll error).");
        println!("  Hint: Try checking the 'Limit RAM to 2GB' box in the installer.\n");
    }

    // Try to find the installed executable
    let exe_path = match find_windows_executable(&wine_prefix) {
        Ok(path) => path,
        Err(_) => {
            println!("{} Could not find game executable automatically.", "⚠".yellow());
            interactive_exe_picker(&wine_prefix)?
        }
    };
    
    println!("{} Found installed game executable", "✔".green());
    
    Ok(exe_path)
}

fn get_dir_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    if !path.exists() { return Ok(0); }
    for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                size += metadata.len();
            }
        }
    }
    Ok(size)
}

/// Find the main Windows executable in the Wine prefix
fn find_windows_executable(wine_prefix: &Path) -> Result<PathBuf> {
    println!("{} Searching for game executable...", "▶".cyan());
    
    // Check common installation directories in Wine prefix
    let program_files = wine_prefix.join("pfx/drive_c/Program Files");
    let program_files_x86 = wine_prefix.join("pfx/drive_c/Program Files (x86)");
    let games_dir = wine_prefix.join("pfx/drive_c/Games");
    
    for search_dir in [program_files, program_files_x86, games_dir] {
        if !search_dir.exists() {
            continue;
        }
        
        // Look for .exe files, excluding common system/uninstaller files
        for entry in walkdir::WalkDir::new(&search_dir)
            .max_depth(4)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                let lower_name = file_name.to_lowercase();
                
                // Skip installers, uninstallers, and system files
                if lower_name.ends_with(".exe")
                    && !lower_name.contains("unins")
                    && !lower_name.contains("setup")
                    && !lower_name.contains("install")
                    && !lower_name.contains("crash")
                    && !lower_name.contains("redist")
                {
                    return Ok(path.to_path_buf());
                }
            }
        }
    }
    
    Err(anyhow!(
        "Could not find game executable in Wine prefix."
    ))
}

fn interactive_exe_picker(wine_prefix: &Path) -> Result<PathBuf> {
    println!("{} Scanning for all executables...", "▶".cyan());
    let drive_c = wine_prefix.join("pfx/drive_c");
    let mut exes = Vec::new();

    for entry in walkdir::WalkDir::new(&drive_c)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let lower_name = file_name.to_lowercase();
            
            if lower_name.ends_with(".exe") 
                && !lower_name.contains("unins")
                && !lower_name.contains("setup")
                && !lower_name.contains("install")
                && !lower_name.contains("crash")
                && !lower_name.contains("redist")
                && !path.to_string_lossy().contains("windows/system")
            {
                exes.push(path.to_path_buf());
            }
        }
    }

    if exes.is_empty() {
        return Err(anyhow!("No executables found in the prefix. Installation probably failed."));
    }

    println!("\n{} Please select the game executable:", "▶".cyan().bold());
    for (i, exe) in exes.iter().enumerate() {
        let rel_path = exe.strip_prefix(&drive_c).unwrap_or(exe);
        println!("  {}. {:?}", (i + 1).to_string().green(), rel_path);
    }
    println!("  {}. Cancel", (exes.len() + 1).to_string().red());

    loop {
        print!("\nEnter number (1-{}): ", exes.len() + 1);
        use std::io::{Write, stdin};
        std::io::stdout().flush()?;
        
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        let choice: usize = match input.trim().parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        if choice == exes.len() + 1 {
            return Err(anyhow!("Selection cancelled by user"));
        }

        if choice > 0 && choice <= exes.len() {
            return Ok(exes[choice - 1].clone());
        }
    }
}

/// Handle portable Windows games (pre-installed folders with .exe)
pub fn setup_portable_windows_game(game_dir: &Path) -> Result<PathBuf> {
    println!("{} Setting up portable Windows game", "▶".cyan());
    
    // Find the main executable
    for entry in walkdir::WalkDir::new(game_dir)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            let lower_name = file_name.to_lowercase();
            
            // Look for likely game executables
            if lower_name.ends_with(".exe")
                && !lower_name.contains("unins")
                && !lower_name.contains("setup")
                && !lower_name.contains("install")
            {
                println!("{} Found executable: {:?}", "✔".green(), file_name);
                return Ok(path.to_path_buf());
            }
        }
    }
    
    Err(anyhow!("No Windows executable found in the game directory"))
}
