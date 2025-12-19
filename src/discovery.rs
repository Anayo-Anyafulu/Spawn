use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::fs;

pub fn discover_executable(game_dir: &Path) -> Result<PathBuf> {
    let mut candidates = Vec::new();

    for entry in WalkDir::new(game_dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            
            // Heuristics:
            // 1. Common launcher scripts in root or AppImage
            if path.parent() == Some(game_dir) && (file_name == "start.sh" || file_name == "run.sh" || file_name == "launcher.sh" || file_name.ends_with(".AppImage")) {
                return Ok(path.to_path_buf());
            }

            // 2. Ends with .x86_64 or .x86
            if file_name.ends_with(".x86_64") || file_name.ends_with(".x86") {
                if is_elf_binary(path) {
                    candidates.push(path.to_path_buf());
                }
            } else if !file_name.contains('.') {
                // 3. No extension and is not a common text/data file
                if !path.to_string_lossy().contains("/lib/") && !path.to_string_lossy().contains("/docs/") {
                     if is_elf_binary(path) {
                         candidates.push(path.to_path_buf());
                     }
                }
            }
        }
    }

    candidates.sort_by_key(|p| (p.components().count(), p.file_name().map(|n| n.len()).unwrap_or(0)));

    candidates.into_iter().next().ok_or_else(|| anyhow!("No executable found in {:?}\nHint: This archive may not be a Linux build", game_dir))
}

pub fn discover_icon(game_dir: &Path) -> Option<PathBuf> {
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

pub fn is_elf_binary(path: &Path) -> bool {
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
