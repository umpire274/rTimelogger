use crate::config::Config;
use crate::db::pool::DbPool;
use crate::errors::AppResult;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use zip::ZipWriter;
use zip::write::FileOptions;

pub struct BackupLogic;

impl BackupLogic {
    pub fn backup(
        _pool: &mut DbPool,
        cfg: &Config,
        dest_file: &str,
        compress: bool,
    ) -> AppResult<()> {
        let src = Path::new(&cfg.database);
        let dest = Path::new(dest_file);

        // 1️⃣ Check DB exists
        if !src.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Database not found: {}", src.display()),
            )
            .into());
        }

        // 2️⃣ Ensure destination folder exists
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // ⛔ 2.5️⃣ If destination file exists → ask confirmation
        if dest.exists() {
            println!(
                "⚠️  The file '{}' already exists.\nDo you want to overwrite it? [y/N]: ",
                dest.display()
            );

            use std::io::{Write, stdin, stdout};

            let mut answer = String::new();
            print!("> ");
            stdout().flush().ok();

            stdin()
                .read_line(&mut answer)
                .expect("Failed to read user input");

            let answer = answer.trim().to_lowercase();

            if !(answer == "y" || answer == "yes") {
                println!("❌ Backup cancelled by user.");
                return Ok(()); // ← exit safely
            }
            println!();
        }

        // 3️⃣ Copy database
        fs::copy(src, dest)?;
        println!("✅ Backup created: {}", dest.display());

        // 4️⃣ Optional compression
        let final_path = if compress {
            let compressed = compress_backup(dest)?;

            if compressed != dest.to_path_buf() {
                // remove uncompressed copy
                if let Err(e) = fs::remove_file(dest) {
                    eprintln!("⚠️ Failed to remove uncompressed backup: {}", e);
                } else {
                    println!("🗑️ Removed uncompressed backup: {}", dest.display());
                }
            }

            compressed
        } else {
            dest.to_path_buf()
        };

        // 5️⃣ Log in DB
        if let Ok(conn) = Connection::open(src) {
            let _ = crate::db::log::ttlog(
                &conn,
                "backup",
                &final_path.to_string_lossy(),
                if compress {
                    "Backup created and compressed"
                } else {
                    "Backup created"
                },
            );
        }

        Ok(())
    }
}

/// Compress a backup using .zip
fn compress_backup(path: &Path) -> AppResult<PathBuf> {
    let zip_path = path.with_extension("zip");
    let file = fs::File::create(&zip_path)?;
    let mut zip = ZipWriter::new(file);

    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut f = fs::File::open(path)?;
    zip.start_file(path.file_name().unwrap().to_string_lossy(), options)
        .map_err(std::io::Error::other)?;

    std::io::copy(&mut f, &mut zip)?;
    zip.finish().map_err(std::io::Error::other)?;

    println!("📦 Compressed: {}", zip_path.display());

    Ok(zip_path)
}
