use crate::config::Config;
use crate::db::pool::DbPool;
use crate::errors::{AppError, AppResult};
use crate::ui::messages::{info, success as ok, warning as warn};
use rusqlite::Connection;
use std::fs;
use std::io::{self, Write};
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

        //
        // 1️⃣ Check database exists
        //
        if !src.exists() {
            return Err(AppError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Database not found: {}", src.display()),
            )));
        }

        //
        // 2️⃣ Ensure destination directory exists
        //
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(AppError::Io)?;
        }

        //
        // 3️⃣ If destination exists → ask user confirmation
        //
        if dest.exists() {
            warn(format!(
                "The backup file '{}' already exists.",
                dest.display()
            ));
            if !ask_overwrite_confirmation()? {
                info("Backup cancelled by user.".to_string());
                return Ok(());
            }
        }

        //
        // 4️⃣ Copy DB
        //
        fs::copy(src, dest).map_err(AppError::Io)?;
        ok(format!("Backup created: {}", dest.display()));

        //
        // 5️⃣ Optional compression
        //
        let final_path = if compress {
            let zipped = compress_backup(dest)?;

            // Remove uncompressed copy only if zip is different
            if zipped != dest {
                if let Err(e) = fs::remove_file(dest) {
                    warn(format!(
                        "Failed to delete temporary uncompressed backup: {}",
                        e
                    ));
                } else {
                    info(format!("Removed uncompressed backup: {}", dest.display()));
                }
            }

            zipped
        } else {
            dest.to_path_buf()
        };

        //
        // 6️⃣ Log operation inside DB
        //
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

//
// ─────────────────────────────────────────────────────────────────────────────
// Helper: Ask confirmation for overwriting an existing backup file
// ─────────────────────────────────────────────────────────────────────────────
//

fn ask_overwrite_confirmation() -> AppResult<bool> {
    use std::io::{stdin, stdout};

    println!("Do you want to overwrite it? [y/N]: ");
    print!("> ");
    stdout().flush().ok();

    let mut input = String::new();
    stdin().read_line(&mut input).map_err(AppError::Io)?;

    let ans = input.trim().to_lowercase();
    Ok(ans == "y" || ans == "yes")
}

//
// ─────────────────────────────────────────────────────────────────────────────
// Helper: Compress to ZIP
// ─────────────────────────────────────────────────────────────────────────────
//

fn compress_backup(path: &Path) -> AppResult<PathBuf> {
    let zip_path = path.with_extension("zip");
    let file = fs::File::create(&zip_path).map_err(AppError::Io)?;
    let mut zip = ZipWriter::new(file);

    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Add the file to zip
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| AppError::InvalidOperation("Invalid file name for backup".into()))?;

    zip.start_file(filename, options)
        .map_err(|e| AppError::Io(io::Error::other(e)))?;

    // Copy DB content into ZIP
    let mut f = fs::File::open(path).map_err(AppError::Io)?;
    io::copy(&mut f, &mut zip).map_err(AppError::Io)?;

    zip.finish()
        .map_err(|e| AppError::Io(io::Error::other(e)))?;

    ok(format!("Compressed backup: {}", zip_path.display()));

    Ok(zip_path)
}
