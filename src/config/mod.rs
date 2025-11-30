use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub mod migrate; // use submodule at src/config/migrate.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub database: String,
    pub default_position: String,
    pub min_work_duration: String,
    #[serde(default = "default_min_lunch")]
    pub min_duration_lunch_break: i32,
    #[serde(default = "default_max_lunch")]
    pub max_duration_lunch_break: i32,
    #[serde(default = "default_separator_char")]
    pub separator_char: String,
    pub show_weekday: String,
}

fn default_min_lunch() -> i32 {
    30
}
fn default_max_lunch() -> i32 {
    90
}
fn default_separator_char() -> String {
    "-".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let db_path = Self::database_file();
        Self {
            database: db_path.to_string_lossy().to_string(),
            default_position: "O".to_string(),
            min_work_duration: "8h".to_string(),
            min_duration_lunch_break: default_min_lunch(),
            max_duration_lunch_break: default_max_lunch(),
            separator_char: default_separator_char(),
            show_weekday: "None".to_string(),
        }
    }
}

impl Config {
    /// Return the standard configuration directory depending on the platform
    pub fn config_dir() -> PathBuf {
        if cfg!(target_os = "windows") {
            let appdata = env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata).join("rtimelogger")
        } else {
            let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".rtimelogger")
        }
    }

    /// Return the full path of the config file
    pub fn config_file() -> PathBuf {
        Self::config_dir().join("rtimelogger.conf")
    }

    /// Return the full path of the SQLite database
    pub fn database_file() -> PathBuf {
        Self::config_dir().join("rtimelogger.sqlite")
    }

    /// Load configuration from file, or return defaults if not found
    pub fn load() -> Self {
        let path = Self::config_file();

        // ADD QUI — debug print temporaneo
        if path.exists() {
            let content = fs::read_to_string(&path).expect("❌ Failed to read configuration file");
            serde_yaml::from_str(&content).expect("❌ Failed to parse configuration file")
        } else {
            Config {
                database: Self::database_file().to_string_lossy().to_string(),
                default_position: "O".to_string(),
                min_work_duration: "8h".to_string(),
                min_duration_lunch_break: 30,
                max_duration_lunch_break: 90,
                separator_char: default_separator_char(),
                show_weekday: "None".to_string(),
            }
        }
    }

    /// Initialize configuration and database files
    pub fn init_all(custom_name: Option<String>, is_test: bool) -> io::Result<()> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;

        // DB name: user provided or default
        let db_path = if let Some(name) = custom_name {
            let p = std::path::Path::new(&name);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                dir.join(p)
            }
        } else {
            dir.join("rtimelogger.sqlite")
        };

        let config = Config {
            database: db_path.to_string_lossy().to_string(),
            default_position: "O".to_string(),
            min_work_duration: "8h".to_string(),
            min_duration_lunch_break: 30,
            max_duration_lunch_break: 90,
            separator_char: default_separator_char(),
            show_weekday: "None".to_string(),
        };

        // Write config file
        if !is_test {
            let yaml = serde_yaml::to_string(&config).unwrap();
            let mut file = fs::File::create(Self::config_file())?;
            file.write_all(yaml.as_bytes())?;
            println!("✅ Config file: {:?}", Self::config_file());
        }

        // Create empty DB file if not exists
        if !db_path.exists() {
            fs::File::create(&db_path)?;
        }

        println!("✅ Database:    {:?}", db_path);

        Ok(())
    }
}
