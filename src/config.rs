use super::util::IOError;
use super::util::OutputWrapper;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::BufReader;
use std::io::BufWriter;
use std::path;

static DEFAULT_CONFIG: &'static str = include_str!("default_config/config.json");

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub loaded_character_path: Option<String>,
    pub alternative_crits: Option<bool>,
    pub skills: HashMap<String, SkillConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct SkillConfig {
    pub attributes: Vec<String>,
}

impl Config {
    pub fn get_or_create(printer: &impl OutputWrapper) -> Result<Config, IOError> {
        let mut path = get_config_dir()?;
        path.push("config.json");
        let path_str = String::from(path.to_str().unwrap());

        if path::Path::exists(&path) {
            let file = match fs::File::open(path) {
                Ok(f) => f,
                Err(_) => {
                    return Err(IOError::from_string(format!(
                        "Unable to open config file at: {}",
                        path_str
                    )));
                }
            };
            let reader = BufReader::new(file);
            let config: serde_json::Result<Config> = serde_json::from_reader(reader);
            match config {
                Ok(c) => Ok(c),
                Err(e) => Err(IOError::from_string(format!(
                    "Invalid syntax in {}, detected at line {}",
                    path_str,
                    e.line()
                ))),
            }
        } else {
            match fs::write(path, DEFAULT_CONFIG) {
                Ok(()) => {
                    printer.output_line(format!("Created default config file at: {}", path_str));
                    Config::get_or_create(printer)
                }
                Err(e) => Err(IOError::from_string(format!(
                    "Unable to write to config file ({})",
                    e.to_string()
                ))),
            }
        }
    }

    pub fn save(self) -> Result<(), IOError> {
        let mut config_path = get_config_dir()?;
        config_path.push("config.json");
        let file = match fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(config_path)
        {
            Ok(f) => f,
            Err(e) => {
                return Err(IOError::from_string(format!(
                    "Unable to write to config file ({})",
                    e.to_string()
                )));
            }
        };
        let writer = BufWriter::new(file);
        match serde_json::to_writer_pretty(writer, &self) {
            Ok(()) => Ok(()),
            Err(_) => Err(IOError::from_str("Unable to serialize configuration")),
        }
    }
}

#[cfg(target_os = "linux")]
pub fn get_config_dir() -> Result<path::PathBuf, IOError> {
    let home = match env::var("HOME") {
        Ok(s) => s,
        Err(_) => {
            return Err(IOError::from_str(
                "Could not read environment variable $HOME",
            ));
        }
    };
    let mut path = path::PathBuf::new();
    path.push(home);
    path.push(".config");
    path.push("dsa_cli");
    match fs::create_dir_all(&path) {
        Ok(()) => {}
        Err(e) => {
            return Err(IOError::from_string(format!(
                "Unable to create config folder ({})",
                e.to_string()
            )));
        }
    }
    Ok(path)
}

#[cfg(target_os = "windows")]
pub fn get_config_dir() -> Result<path::PathBuf, IOError> {
    let appdata = match env::var("appdata") {
        Ok(s) => s,
        Err(_) => {
            return Err(IOError::from_str(
                "Could not read environment variable \"appdata\"",
            ));
        }
    };
    let mut path = path::PathBuf::new();
    path.push(appdata);
    path.push("dsa_cli");
    match fs::create_dir_all(&path) {
        Ok(()) => {}
        Err(e) => {
            return Err(IOError::from_string(format!(
                "Unable to create config folder ({})",
                e.to_string()
            )));
        }
    }
    Ok(path)
}

#[cfg(target_os = "macos")]
pub fn get_config_dir() -> Result<path::PathBuf, IOError> {
    let appdata = match env::var("HOME") {
        Ok(s) => s,
        Err(_) => {
            return Err(IOError::from_str(
                "Could not read environment variable \"appdata\"",
            ));
        }
    };
    let mut path = path::PathBuf::new();
    path.push(appdata);
    path.push("Library");
    path.push("Application Support");
    path.push("dsa_cli");
    match fs::create_dir_all(&path) {
        Ok(()) => {}
        Err(e) => {
            return Err(IOError::from_string(format!(
                "Unable to create config folder ({})",
                e.to_string()
            )));
        }
    }
    Ok(path)
}
