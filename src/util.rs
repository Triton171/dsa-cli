use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::BufReader;
use std::io::BufWriter;
use std::path;
use std::collections::HashMap;
use super::print::Printer;

static DEFAULT_CONFIG: &'static str = include_str!("default_config/config.json");

pub struct IOError {
    message: String,
}

impl IOError {
    pub fn from_str(message: &str) -> IOError {
        IOError {
            message: String::from(message),
        }
    }

    pub fn from_string(message: String) -> IOError {
        IOError { message }
    }

    pub fn message<'a>(&'a self) -> &'a str {
        &self.message
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub loaded_character_path: Option<String>,
    pub alternative_crits: Option<bool>,
    pub skills: HashMap<String, SkillConfig>
}

#[derive(Serialize, Deserialize)]
pub struct SkillConfig {
    pub attributes: Vec<String>,
}

impl Config {
    pub fn get_or_create(printer: &impl Printer) -> Result<Config, IOError> {
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
                Err(e) => {
                    Err(IOError::from_string(format!(
                        "Unable to write to config file ({})",
                        e.to_string()
                    )))
                }
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

#[derive(Deserialize)]
pub struct Character {
    name: String,
    attributes: Vec<CharacterAttribute>,
    skills: Vec<CharacterSkill>
}

#[derive(Deserialize)]
pub struct CharacterSkill {
    id: String,
    level: i64
}

#[derive(Deserialize)]
pub struct CharacterAttribute {
    id: String,
    level: i64
}


impl Character {
    pub fn loaded_character(config: &Config) -> Result<Option<Character>, IOError> {
        let char_path = match &config.loaded_character_path {
            Some(p) => p,
            None => {
                return Ok(None);
            }
        };

        let char_file = match fs::File::open(path::PathBuf::from(char_path)) {
            Ok(f) => f,
            Err(_) => {
                return Err(IOError::from_str("Unable to open character file"));
            }
        };
        let reader = BufReader::new(char_file);
        match serde_json::from_reader(reader) {
            Ok(c) => Ok(Some(c)),
            Err(e) => Err(IOError::from_string(format!(
                "Invalid character format, detected at line {}",
                e.line()
            )))
        }
    }

    pub fn load(path: &str, config: &mut Config) -> Result<Character, IOError> {
        let p = path::Path::new(path);
        let p = match fs::canonicalize(p) {
            Ok(p) => p,
            Err(_) => {
                return Err(IOError::from_str("Unable to resolve character path"));
            }
        };
        config.loaded_character_path = Some(p.to_str().unwrap().to_owned());
        match Character::loaded_character(config) {
            Ok(Some(c)) => Ok(c),
            Ok(None) => Err(IOError::from_str("Character was not loaded correctly")),
            Err(e) => Err(e),
        }
    }

    pub fn unload(config: &mut Config) {
        config.loaded_character_path = None;
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_skill_level(&self, skill_id: &str) -> i64 {
        for skill in &self.skills {
            if skill.id.eq_ignore_ascii_case(skill_id) {
                return skill.level;
            }
        }
        0
    }

    pub fn get_attribute_level(&self, attr_id: &str) -> i64 {
        for attr in &self.attributes {
            if attr.id.eq_ignore_ascii_case(attr_id) {
                return attr.level;
            }
        }
        0
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
                "Could not read environment variable \"appdata\""
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
                "Could not read environment variable \"appdata\""
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