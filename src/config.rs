use super::util::Error;
use super::util::ErrorType;
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
    pub discord: DiscordConfig,
    pub skills: HashMap<String, SkillConfig>,
    pub combattechniques: HashMap<String, CombatTechniqueConfig>,
    pub spells: HashMap<String, SpellConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct SkillConfig {
    pub attributes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CombatTechniqueConfig {
    pub attributes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DiscordConfig {
    pub login_token: Option<String>,
    pub require_complete_command: bool,
    pub max_attachement_size: u64,
    pub use_reply: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SpellConfig {
    pub attributes: Vec<String>,
}

impl Config {
    pub fn get_or_create(output: &mut impl OutputWrapper) -> Result<Config, Error> {
        let mut path = get_config_dir()?;
        path.push("config.json");

        if path::Path::exists(&path) {
            match Config::get(&path) {
                Ok(c) => Ok(c),
                Err(e) => {
                    let err_type = e.err_type();
                    if let ErrorType::InvalidFormat = err_type {
                        let mut old_config = get_config_dir()?;
                        old_config.push("old_config.json");
                        match fs::rename(&path, old_config) {
                            Ok(()) => {
                                output.output_line(&format!("Found invalid configuration file at {}, renamed it to \"old_config.json\"",
                                    path.to_str().unwrap()));
                                output.output_line(&"If you made any changes to the old configuration file, make sure to transfer them, otherwise you don't need to do anything");
                                output.new_line();
                                Config::get_or_create(output)
                            }
                            Err(e) => Err(Error::from_string(
                                format!("Unable to rename outdated config file: {}", e.to_string()),
                                ErrorType::FileSystemError,
                            )),
                        }
                    } else {
                        Err(e)
                    }
                }
            }
        } else {
            match fs::write(&path, DEFAULT_CONFIG) {
                Ok(()) => {
                    output.output_line(&format!(
                        "Created default config file at: {}",
                        path.to_str().unwrap()
                    ));
                    Config::get_or_create(output)
                }
                Err(e) => Err(Error::from_string(
                    format!("Unable to write to config file ({})", e.to_string()),
                    ErrorType::FileSystemError,
                )),
            }
        }
    }

    fn get(path: &path::Path) -> Result<Config, Error> {
        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => {
                return Err(Error::from_string(
                    format!("Unable to open config file at: {}", path.to_str().unwrap()),
                    ErrorType::FileSystemError,
                ));
            }
        };
        let reader = BufReader::new(file);
        let config: serde_json::Result<Config> = serde_json::from_reader(reader);
        match config {
            Ok(c) => Ok(c),
            Err(e) => Err(Error::from_string(
                format!(
                    "Invalid syntax in {}, detected at line {}",
                    path.to_str().unwrap(),
                    e.line()
                ),
                ErrorType::InvalidFormat,
            )),
        }
    }

    pub fn save(self) -> Result<(), Error> {
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
                return Err(Error::from_string(
                    format!("Unable to write to config file ({})", e.to_string()),
                    ErrorType::FileSystemError,
                ));
            }
        };
        let writer = BufWriter::new(file);
        match serde_json::to_writer_pretty(writer, &self) {
            Ok(()) => {}
            Err(_) => {
                return Err(Error::from_str(
                    "Unable to serialize configuration",
                    ErrorType::Unknown,
                ))
            }
        }
        Ok(())
    }

    /*
    Searches for a search term among the keys of the given hashmap
    '_' at the beginning or end of the search term marks the beginning/end of the name
    */
    pub fn match_search<'a, V>(
        entries: &'a HashMap<String, V>,
        search: &str,
    ) -> Result<&'a str, Error> {
        let mut found_name: Option<&str> = None;
        let mut search_trimmed = search;
        let search_at_beg = if search_trimmed.starts_with('_') {
            search_trimmed = &search_trimmed[1..];
            true
        } else {
            false
        };
        let search_at_end = if search_trimmed.ends_with('_') {
            search_trimmed = &search_trimmed[..search_trimmed.len() - 1];
            true
        } else {
            false
        };
        for (name, _) in entries {
            if name.contains(search_trimmed) {
                let mut matches_search = true;
                if search_at_beg && !name.starts_with(search_trimmed) {
                    matches_search = false;
                }
                if search_at_end && !name.ends_with(search_trimmed) {
                    matches_search = false;
                }

                if matches_search {
                    if let Some(found_name) = found_name {
                        return Err(Error::from_string(format!("Ambiguous identifier \"{}\": Matched \"{}\" and \"{}\".\nNote: You can use \"_\" to mark the beginning and/or end of the name.", search, found_name, name),
                            ErrorType::InvalidArgument));
                    } else {
                        found_name = Some(name);
                    }
                }
            }
        }
        if let Some(found_name) = found_name {
            Ok(found_name)
        } else {
            Err(Error::from_string(
                format!("No matches found for \"{}\"", search),
                ErrorType::InvalidArgument,
            ))
        }
    }
}

#[cfg(target_os = "linux")]
pub fn get_config_dir() -> Result<path::PathBuf, Error> {
    match env::var("DSA_CLI_CONFIG_DIR") {
        Err(_) => {}
        Ok(s) => {
            let mut path = path::PathBuf::new();
            path.push(s);
            match fs::create_dir_all(&path) {
                Ok(()) => {
                    return Ok(path);
                }
                Err(e) => {
                    return Err(Error::from_string(
                        format!("Unable to create config folder ({})", e.to_string()),
                        ErrorType::FileSystemError,
                    ));
                }
            }
        }
    };

    let home = match env::var("HOME") {
        Ok(s) => s,
        Err(_) => {
            return Err(Error::from_str(
                "Could not read environment variable $HOME",
                ErrorType::MissingEnvironmentVariable,
            ));
        }
    };
    let mut path = path::PathBuf::new();
    path.push(home);
    path.push(".config");
    path.push("dsa-cli");
    match fs::create_dir_all(&path) {
        Ok(()) => {}
        Err(e) => {
            return Err(Error::from_string(
                format!("Unable to create config folder ({})", e.to_string()),
                ErrorType::FileSystemError,
            ));
        }
    }
    Ok(path)
}

#[cfg(target_os = "windows")]
pub fn get_config_dir() -> Result<path::PathBuf, Error> {
    match env::var("DSA_CLI_CONFIG_DIR") {
        Err(_) => {}
        Ok(s) => {
            let mut path = path::PathBuf::new();
            path.push(s);
            match fs::create_dir_all(&path) {
                Ok(()) => {
                    return Ok(path);
                }
                Err(e) => {
                    return Err(Error::from_string(
                        format!("Unable to create config folder ({})", e.to_string()),
                        ErrorType::FileSystemError,
                    ));
                }
            }
        }
    };

    let appdata = match env::var("appdata") {
        Ok(s) => s,
        Err(_) => {
            return Err(Error::from_str(
                "Could not read environment variable \"appdata\"",
                ErrorType::MissingEnvironmentVariable,
            ));
        }
    };
    let mut path = path::PathBuf::new();
    path.push(appdata);
    path.push("dsa-cli");
    match fs::create_dir_all(&path) {
        Ok(()) => {}
        Err(e) => {
            return Err(Error::from_string(
                format!("Unable to create config folder ({})", e.to_string()),
                ErrorType::FileSystemError,
            ));
        }
    }
    Ok(path)
}

#[cfg(target_os = "macos")]
pub fn get_config_dir() -> Result<path::PathBuf, Error> {
    match env::var("DSA_CLI_CONFIG_DIR") {
        Err(_) => {}
        Ok(s) => {
            let mut path = path::PathBuf::new();
            path.push(s);
            match fs::create_dir_all(&path) {
                Ok(()) => {
                    return Ok(path);
                }
                Err(e) => {
                    return Err(Error::from_string(
                        format!("Unable to create config folder ({})", e.to_string()),
                        ErrorType::FileSystemError,
                    ));
                }
            }
        }
    };

    let appdata = match env::var("HOME") {
        Ok(s) => s,
        Err(_) => {
            return Err(Error::from_str(
                "Could not read environment variable \"appdata\"",
                ErrorType::MissingEnvironmentVariable,
            ));
        }
    };
    let mut path = path::PathBuf::new();
    path.push(appdata);
    path.push("Library");
    path.push("Application Support");
    path.push("dsa-cli");
    match fs::create_dir_all(&path) {
        Ok(()) => {}
        Err(e) => {
            return Err(Error::from_string(
                format!("Unable to create config folder ({})", e.to_string()),
                ErrorType::FileSystemError,
            ));
        }
    }
    Ok(path)
}
