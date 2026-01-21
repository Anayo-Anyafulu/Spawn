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
    
    // Check if Steam is Flatpak
    let is_flatpak = is_flatpak_steam();
    
    let status = if is_flatpak {
        println!("{} Detected Flatpak Steam - using flatpak run", "▶".cyan());
        
        // Map the host path to the internal path if possible
        let internal_path = if let Some(home_path) = dirs_next::home_dir() {
            let home_str = home_path.to_string_lossy().to_string();
            installer_path.to_string_lossy().replace(&home_str, "~")
        } else {
            installer_path.display().to_string()
        };

        // For Flatpak Steam, we need to run the installer through flatpak
        // We use a shell script snippet to find the proton binary inside the sandbox
        // and add --filesystem=home to ensure it can see the installer and prefix.
        Command::new("flatpak")
            .arg("run")
            .arg("--filesystem=home")
            .arg("--command=sh")
            .arg("com.valvesoftware.Steam")
            .arg("-c")
            .arg(format!(
                "PROTON_EXE=$(find ~/.local/share/Steam/steamapps/common -name proton -type f | head -n 1); \
                 if [ -z \"$PROTON_EXE\" ]; then \
                   echo 'Error: Proton binary not found inside Steam sandbox'; exit 1; \
                 fi; \
                 WINEPREFIX='{}' STEAM_COMPAT_DATA_PATH='{}' \"$PROTON_EXE\" run \"{}\"",
                wine_prefix.display(),
                wine_prefix.display(),
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
    
    // Try to find the installed executable
    let exe_path = find_windows_executable(&wine_prefix)?;
    
    println!("{} Found installed game executable", "✔".green());
    
    Ok(exe_path)
}

/// Find the main Windows executable in the Wine prefix
fn find_windows_executable(wine_prefix: &Path) -> Result<PathBuf> {
    println!("{} Searching for game executable...", "▶".cyan());
    
    // Check common installation directories in Wine prefix
    let program_files = wine_prefix.join("drive_c/Program Files");
    let program_files_x86 = wine_prefix.join("drive_c/Program Files (x86)");
    
    for search_dir in [program_files, program_files_x86] {
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
        "Could not find game executable in Wine prefix.\n\
        You may need to manually locate the .exe file."
    ))
}

/// Handle portable Windows games (pre-installed folders with .exe)
pub fn setup_portable_windows_game(game_dir: &Path) -> Result<PathBuf> {
    println!("{} Setting up portable Windows game", "▶".cyan());
    
    // Find the main executable
    for entry in walkdir::WalkDir::new(game_dir)
        .max_depth(3)
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
