use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub database: String,
    pub default_position: String,
    pub min_work_duration: String,
    pub lunch_window: String,
    #[serde(default = "default_min_lunch")]
    pub min_duration_lunch_break: i32,
    #[serde(default = "default_max_lunch")]
    pub max_duration_lunch_break: i32,
    #[serde(default = "default_separator_char")]
    pub separator_char: String,
    pub show_weekday: String,
}

// ---------------------------------------------
// DEFAULT VALUE FUNCTIONS
// ---------------------------------------------
fn default_min_lunch() -> i32 {
    30
}
fn default_max_lunch() -> i32 {
    90
}
fn default_separator_char() -> String {
    "-".to_string()
}

// ---------------------------------------------
// CONFIG DEFAULT IMPL
// ---------------------------------------------
impl Default for Config {
    fn default() -> Self {
        let db_path = Self::database_file();
        Self {
            database: db_path.to_string_lossy().to_string(),
            default_position: "O".to_string(),
            min_work_duration: "8h".to_string(),
            lunch_window: "12:30-14:00".to_string(),
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

    /// Load configuration from file, or return defaults if not found.
    /// If some fields are missing in the YAML, they are added with default values
    /// and the file is updated.
    pub fn load() -> Self {
        let path = Self::config_file();

        // 1) Se il file non esiste → crea directory + file con default
        if !path.exists() {
            let defaults = Config::default();

            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            if let Ok(yaml) = serde_yaml::to_string(&defaults) {
                if let Err(e) = fs::write(&path, yaml) {
                    eprintln!("⚠️ Failed to write default config file: {e}");
                }
            }

            return defaults;
        }

        // 2) Leggi il contenuto grezzo
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("⚠️ Failed to read config file ({e}), using defaults.");
                return Config::default();
            }
        };

        if content.trim().is_empty() {
            eprintln!("⚠️ Config file is empty, regenerating defaults.");
            let defaults = Config::default();
            if let Ok(yaml) = serde_yaml::to_string(&defaults) {
                let _ = fs::write(&path, yaml);
            }
            return defaults;
        }

        // 3) Parse raw YAML per vedere cosa esiste *realmente* nel file
        let raw_yaml: serde_yaml::Value = match serde_yaml::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("⚠️ Failed to parse raw YAML ({e}), using defaults.");
                let defaults = Config::default();
                if let Ok(yaml) = serde_yaml::to_string(&defaults) {
                    let _ = fs::write(&path, yaml);
                }
                return defaults;
            }
        };

        // 4) Parse in Config (qui Serde completa i campi mancanti in memoria)
        let mut loaded: Config = match serde_yaml::from_str(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("⚠️ Failed to parse Config struct ({e}), using defaults.");
                let defaults = Config::default();
                if let Ok(yaml) = serde_yaml::to_string(&defaults) {
                    let _ = fs::write(&path, yaml);
                }
                return defaults;
            }
        };

        let defaults = Config::default();
        let mut modified = false;

        // Helper per i campi stringa: li consideriamo "mancanti" se la chiave non esiste nel YAML
        macro_rules! ensure_field {
            ($yaml_key:literal, $field:ident) => {
                if raw_yaml.get($yaml_key).is_none() {
                    // nel file non c'è proprio la chiave → aggiungiamo il default
                    loaded.$field = defaults.$field.clone();
                    eprintln!(
                        "⚠️ Missing field '{}' in config file, inserting default.",
                        $yaml_key
                    );
                    modified = true;
                }
            };
        }

        // String fields
        ensure_field!("database", database);
        ensure_field!("default_position", default_position);
        ensure_field!("min_work_duration", min_work_duration);
        ensure_field!("lunch_window", lunch_window);
        ensure_field!("separator_char", separator_char);
        ensure_field!("show_weekday", show_weekday);

        // Numeric fields: se la chiave non esiste nel file, li impostiamo a default
        if raw_yaml.get("min_duration_lunch_break").is_none() {
            loaded.min_duration_lunch_break = defaults.min_duration_lunch_break;
            eprintln!("⚠️ Missing field 'min_duration_lunch_break', inserting default.");
            modified = true;
        }

        if raw_yaml.get("max_duration_lunch_break").is_none() {
            loaded.max_duration_lunch_break = defaults.max_duration_lunch_break;
            eprintln!("⚠️ Missing field 'max_duration_lunch_break', inserting default.");
            modified = true;
        }

        // 5) Se abbiamo modificato qualcosa → riscriviamo il file aggiornato
        if modified {
            if let Ok(yaml) = serde_yaml::to_string(&loaded) {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if let Err(e) = fs::write(&path, yaml) {
                    eprintln!("⚠️ Failed to update config file: {e}");
                } else {
                    eprintln!("🔧 Config file updated with missing fields.");
                }
            }
        }

        loaded
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
            ..Config::default()
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
