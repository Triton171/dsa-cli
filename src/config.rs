use super::util::IOError;
use super::util::IOErrorType;
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
    pub combattechniques: HashMap<String, CombatTechniqueConfig>
}

#[derive(Serialize, Deserialize)]
pub struct SkillConfig {
    pub attributes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CombatTechniqueConfig {
    pub attributes: Vec<String>
}

impl Config {
    pub fn get_or_create(output: &dyn OutputWrapper) -> Result<Config, IOError> {
        let mut path = get_config_dir()?;
        path.push("config.json");

        if path::Path::exists(&path) {
            match Config::get(&path) {
                Ok(c) => Ok(c),
                Err(e) => {
                    let err_type = e.err_type();
                    if let IOErrorType::InvalidFormat = err_type {
                        let mut old_config = get_config_dir()?;
                        old_config.push("old_config.json");
                        match fs::rename(&path, old_config) {
                            Ok(()) => {
                                output.output_line(format!("Found invalid configuration file at {}, renamed it to \"old_config.json\"",
                                    path.to_str().unwrap()));
                                output.output_line(String::from("If you made any changes to the old configuration file, make sure to transfer them, otherwise you don't need to do anything"));
                                output.new_line();
                                Config::get_or_create(output)
                            }
                            Err(e) => {
                                Err(IOError::from_string(format!("Unable to rename outdated config file: {}", e.to_string()), IOErrorType::FileSystemError))
                            }
                        }
                    } else {
                        Err(e)
                    }

                    
                }
            }
        } else {
            match fs::write(&path, DEFAULT_CONFIG) {
                Ok(()) => {
                    output.output_line(format!("Created default config file at: {}", path.to_str().unwrap()));
                    Config::get_or_create(output)
                }
                Err(e) => Err(IOError::from_string(format!(
                    "Unable to write to config file ({})",
                    e.to_string()
                ), IOErrorType::FileSystemError)),
            }
        }
    }

    fn get(path: &path::Path) -> Result<Config, IOError> {
        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => {
                return Err(IOError::from_string(format!(
                    "Unable to open config file at: {}",
                    path.to_str().unwrap()
                ), IOErrorType::FileSystemError));
            }
        };
        let reader = BufReader::new(file);
        let config: serde_json::Result<Config> = serde_json::from_reader(reader);
        match config {
            Ok(c) => Ok(c),
            Err(e) => Err(IOError::from_string(format!(
                "Invalid syntax in {}, detected at line {}",
                path.to_str().unwrap(),
                e.line()
            ), IOErrorType::InvalidFormat)),
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
                ), IOErrorType::FileSystemError));
            }
        };
        let writer = BufWriter::new(file);
        match serde_json::to_writer_pretty(writer, &self) {
            Ok(()) => Ok(()),
            Err(_) => Err(IOError::from_str("Unable to serialize configuration", IOErrorType::Unknown)),
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
                IOErrorType::MissingEnvironmentVariable
            ), );
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
            ), IOErrorType::FileSystemError));
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
