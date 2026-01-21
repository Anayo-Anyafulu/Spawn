use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use colored::*;

/// Find Steam's Proton installation
pub fn find_proton() -> Result<PathBuf> {
    let steam_dir = find_steam_dir()?;
    
    // Check common Proton locations
    let compatibilitytools_dir = steam_dir.join("steamapps/common");
    
    if !compatibilitytools_dir.exists() {
        return Err(anyhow!(
            "Steam compatibility tools directory not found at {:?}\n\
            Proton is required to run Windows games.\n\
            Install Proton through Steam.",
            compatibilitytools_dir
        ));
    }

    // Find the most recent Proton version
    let entries = fs::read_dir(&compatibilitytools_dir)
        .context("Failed to read Steam common directory")?;
    
    let mut proton_versions = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            if name.starts_with("Proton") {
                proton_versions.push(path);
            }
        }
    }

    if proton_versions.is_empty() {
        return Err(anyhow!(
            "No Proton installation found in {:?}\n\
            Please install Proton through Steam:\n\
            1. Open Steam\n\
            2. Go to Library > Tools\n\
            3. Install 'Proton Experimental' or latest Proton version",
            compatibilitytools_dir
        ));
    }

    // Sort and pick the latest version
    proton_versions.sort();
    let proton_path = proton_versions.last().unwrap().clone();
    
    println!("{} Found Proton: {:?}", "✔".green(), proton_path.file_name().unwrap());
    
    Ok(proton_path)
}

/// Find Steam installation directory
pub fn find_steam_dir() -> Result<PathBuf> {
    let home = dirs_next::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;
    
    // Check common Steam locations (in order of preference)
    let candidates = vec![
        home.join(".local/share/Steam"),        // Native install & Flatpak target
        home.join(".steam/steam"),              // Symlink (follow it)
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"), // Flatpak
    ];
    
    for candidate in candidates {
        // Follow symlinks with canonicalize
        if let Ok(canonical) = candidate.canonicalize() {
            if canonical.exists() && canonical.join("steamapps").exists() {
                return Ok(canonical);
            }
        }
        // Also check without canonicalize in case it's a direct path
        if candidate.exists() && candidate.join("steamapps").exists() {
            return Ok(candidate);
        }
    }
    
    Err(anyhow!(
        "Steam installation not found.\n\
        Spawn requires Steam to be installed for:\n\
        - Proton (to run Windows games)\n\
        - Non-Steam game integration\n\
        Please install Steam first."
    ))
}

/// Check if Proton is available
#[allow(dead_code)]
pub fn check_proton_available() -> bool {
    find_proton().is_ok()
}

/// Get Proton executable path
pub fn get_proton_executable(proton_dir: &Path) -> Result<PathBuf> {
    let proton_exe = proton_dir.join("proton");
    
    if !is_flatpak_steam() && !proton_exe.exists() {
        return Err(anyhow!(
            "Proton executable not found at {:?}",
            proton_exe
        ));
    }
    
    Ok(proton_exe)
}

/// Check if Steam is installed via Flatpak
pub fn is_flatpak_steam() -> bool {
    // Check if the Steam directory is in the Flatpak location
    let home = match dirs_next::home_dir() {
        Some(h) => h,
        None => return false,
    };
    
    let flatpak_steam = home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam");
    flatpak_steam.exists()
}
