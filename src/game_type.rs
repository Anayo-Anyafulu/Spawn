use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use colored::*;

/// Represents the detected type of game
#[derive(Debug, Clone, PartialEq)]
pub enum GameType {
    /// Windows installer (.exe, .msi, repack) - needs Proton
    WindowsInstaller,
    /// Linux-native game in archive format (.zip, .tar.gz, etc.)
    LinuxArchive,
    /// Portable Linux game (already extracted folder)
    PortableLinux,
    /// Portable Windows game (already extracted, contains .exe)
    PortableWindows,
}

/// Information about a detected game
#[derive(Debug)]
#[allow(dead_code)]
pub struct GameInfo {
    pub game_type: GameType,
    pub source_path: PathBuf,
    pub name: String,
}

impl GameType {
    pub fn description(&self) -> &str {
        match self {
            GameType::WindowsInstaller => "Windows Installer (requires Proton)",
            GameType::LinuxArchive => "Linux Archive",
            GameType::PortableLinux => "Portable Linux Game",
            GameType::PortableWindows => "Portable Windows Game (requires Proton)",
        }
    }
}

/// Detect the type of game based on the input path
pub fn detect_game_type(path: &Path) -> Result<GameInfo> {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let game_type = if path.is_file() {
        detect_file_type(path)?
    } else if path.is_dir() {
        detect_directory_type(path)?
    } else {
        return Err(anyhow!("Path is neither a file nor a directory: {:?}", path));
    };

    println!("{} Detected: {}", "▶".cyan(), game_type.description().bold());

    Ok(GameInfo {
        game_type,
        source_path: path.to_path_buf(),
        name,
    })
}

fn detect_file_type(path: &Path) -> Result<GameType> {
    let file_name = path.to_string_lossy().to_lowercase();

    // Check for Windows executables and installers
    if file_name.ends_with(".exe") || file_name.ends_with(".msi") {
        return Ok(GameType::WindowsInstaller);
    }

    // Check for AppImage
    if file_name.ends_with(".appimage") {
        return Ok(GameType::LinuxArchive);
    }

    // Check for archives - could be Linux or Windows games
    if is_archive(&file_name) {
        // Peek inside to determine if it's Linux or Windows
        // For now, assume Linux archives unless we detect Windows content
        return Ok(GameType::LinuxArchive);
    }

    Err(anyhow!(
        "Unsupported file type: {:?}\nSupported: .exe, .msi, .zip, .tar.gz, .tar.xz, .tar.bz2, .AppImage",
        path
    ))
}

fn detect_directory_type(path: &Path) -> Result<GameType> {
    // Check if directory contains Windows executables
    let has_windows_exe = walkdir::WalkDir::new(path)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|entry| {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                ext.to_string_lossy().to_lowercase() == "exe"
            } else {
                false
            }
        });

    if has_windows_exe {
        return Ok(GameType::PortableWindows);
    }

    // Check for ELF binaries (Linux)
    let has_linux_binary = walkdir::WalkDir::new(path)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|entry| {
            let path = entry.path();
            if path.is_file() {
                is_elf_binary(path)
            } else {
                false
            }
        });

    if has_linux_binary {
        return Ok(GameType::PortableLinux);
    }

    Err(anyhow!(
        "Could not determine game type for directory: {:?}\nNo executables found",
        path
    ))
}

fn is_archive(file_name: &str) -> bool {
    file_name.ends_with(".zip")
        || file_name.ends_with(".tar.gz")
        || file_name.ends_with(".tar.xz")
        || file_name.ends_with(".tar.bz2")
        || file_name.ends_with(".tgz")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_detection() {
        assert!(is_archive("game.zip"));
        assert!(is_archive("game.tar.gz"));
        assert!(is_archive("game.tar.xz"));
        assert!(is_archive("game.tar.bz2"));
        assert!(!is_archive("game.exe"));
    }
}
