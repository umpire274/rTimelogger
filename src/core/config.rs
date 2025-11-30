use crate::errors::{AppError, AppResult};
use std::fs;
use std::process::Command;

pub struct ConfigLogic;

impl ConfigLogic {
    pub fn print(path: &str) -> AppResult<()> {
        let content = fs::read_to_string(path).map_err(|_| AppError::ConfigLoad)?;
        println!("{}", content);
        Ok(())
    }

    pub fn edit(path: &str, editor: &Option<String>) -> AppResult<()> {
        let ed = editor
            .clone()
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "nano".into());

        Command::new(ed)
            .arg(path)
            .status()
            .map_err(|e| AppError::Config(e.to_string()))?;

        Ok(())
    }
}
