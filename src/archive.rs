use anyhow::{Context, Result, anyhow};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::io::{self, Read};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Extract various archive formats to a target directory
#[allow(dead_code)]
pub fn extract_archive_unified(archive_path: &Path, extract_to: &Path) -> Result<PathBuf> {
    let file_name = archive_path.to_string_lossy().to_lowercase();
    
    println!("{} Extracting {:?}...", "▶".cyan(), archive_path.file_name().unwrap());
    
    let pb = create_progress_spinner();
    pb.set_message("Extracting files...");

    let result = if file_name.ends_with(".zip") {
        extract_zip(archive_path, extract_to)
    } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        extract_tar_gz(archive_path, extract_to)
    } else if file_name.ends_with(".tar.xz") {
        extract_tar_xz(archive_path, extract_to)
    } else if file_name.ends_with(".tar.bz2") {
        extract_tar_bz2(archive_path, extract_to)
    } else if file_name.ends_with(".tar") {
        extract_tar(archive_path, extract_to)
    } else {
        Err(anyhow!("Unsupported archive format: {:?}", archive_path))
    };

    pb.finish_and_clear();
    
    match result {
        Ok(path) => {
            println!("{} Extracted successfully", "✔".green());
            Ok(path)
        }
        Err(e) => Err(e)
    }
}

fn extract_zip(archive_path: &Path, extract_to: &Path) -> Result<PathBuf> {
    let file = File::open(archive_path)
        .context("Failed to open ZIP file")?;
    
    let mut archive = zip::ZipArchive::new(file)
        .context("Failed to read ZIP archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .context("Failed to read file from ZIP")?;
        
        let outpath = match file.enclosed_name() {
            Some(path) => extract_to.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)
                .context("Failed to create directory")?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)
                    .context("Failed to create parent directory")?;
            }
            let mut outfile = File::create(&outpath)
                .context("Failed to create output file")?;
            io::copy(&mut file, &mut outfile)
                .context("Failed to extract file")?;
        }

        // Preserve Unix permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))
                    .ok();
            }
        }
    }

    Ok(extract_to.to_path_buf())
}

fn extract_tar_gz(archive_path: &Path, extract_to: &Path) -> Result<PathBuf> {
    let tar_gz = File::open(archive_path)
        .context("Failed to open tar.gz file")?;
    let tar = flate2::read::GzDecoder::new(tar_gz);
    extract_tar_archive(tar, extract_to)
}

fn extract_tar_xz(archive_path: &Path, extract_to: &Path) -> Result<PathBuf> {
    let tar_xz = File::open(archive_path)
        .context("Failed to open tar.xz file")?;
    let tar = xz2::read::XzDecoder::new(tar_xz);
    extract_tar_archive(tar, extract_to)
}

fn extract_tar_bz2(archive_path: &Path, extract_to: &Path) -> Result<PathBuf> {
    let tar_bz2 = File::open(archive_path)
        .context("Failed to open tar.bz2 file")?;
    let tar = bzip2::read::BzDecoder::new(tar_bz2);
    extract_tar_archive(tar, extract_to)
}

fn extract_tar(archive_path: &Path, extract_to: &Path) -> Result<PathBuf> {
    let tar_file = File::open(archive_path)
        .context("Failed to open tar file")?;
    extract_tar_archive(tar_file, extract_to)
}

fn extract_tar_archive<R: Read>(reader: R, extract_to: &Path) -> Result<PathBuf> {
    let mut archive = tar::Archive::new(reader);
    archive.unpack(extract_to)
        .context("Failed to extract tar archive")?;
    Ok(extract_to.to_path_buf())
}

fn create_progress_spinner() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}
