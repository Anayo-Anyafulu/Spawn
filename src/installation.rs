use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;

pub fn extract_archive(archive_path: &Path, install_dir: &Path, dry_run: bool) -> Result<PathBuf> {
    let stem = archive_path.file_stem().ok_or_else(|| anyhow!("Invalid file name"))?;
    let stem_str = stem.to_string_lossy();
    
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
            .arg("-q")
            .arg(archive_path)
            .arg("-d")
            .arg(&target_dir)
            .status()
            .context("Failed to execute unzip command. Hint: Ensure 'unzip' is installed.")?
    } else {
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

pub fn install_appimage(appimage_path: &Path, install_dir: &Path, dry_run: bool) -> Result<PathBuf> {
    let file_name = appimage_path.file_name().ok_or_else(|| anyhow!("Invalid AppImage path"))?;
    let stem = appimage_path.file_stem().ok_or_else(|| anyhow!("Invalid file name"))?;
    
    let target_dir = install_dir.join(stem);
    if target_dir.exists() {
        println!("{} {:?} is already installed.", "⚠".yellow().bold(), stem);
        println!("  Do you want to overwrite it? [y/N]");
        
        let mut confirm = String::new();
        std::io::stdin().read_line(&mut confirm).context("Failed to read input")?;
        if confirm.trim().to_lowercase() != "y" {
            println!("{} Using existing directory.", "✔".green());
            return Ok(target_dir);
        }

        if !dry_run {
            fs::remove_dir_all(&target_dir).context("Failed to remove existing directory")?;
        }
    }

    if dry_run {
        println!("{} Would move {:?} to {:?}", "▶".cyan(), appimage_path, target_dir);
        return Ok(target_dir);
    }

    fs::create_dir_all(&target_dir).context("Failed to create install directory")?;
    let target_path = target_dir.join(file_name);
    fs::copy(appimage_path, &target_path).context("Failed to copy AppImage")?;
    
    println!("{} Installed AppImage to {:?}", "✔".green(), target_path);
    
    Ok(target_dir)
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
