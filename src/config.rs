use super::util::*;
use serde::{de::DeserializeOwned, Deserialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

const DSA_DATA_NEWEST_VERSION: u64 = 2;

mod default {
    pub fn auto_update_dsa_data() -> bool {
        true
    }
    pub mod dsa_rules {
        pub fn dsa_rules() -> super::super::ConfigDSARules {
            super::super::ConfigDSARules {
                crit_rules: crit_rules(),
            }
        }
        pub fn crit_rules() -> super::super::ConfigDSACritType {
            super::super::ConfigDSACritType::DefaultCrits
        }
    }
    pub mod discord {
        pub fn discord() -> super::super::ConfigDiscord {
            super::super::ConfigDiscord {
                login_token: None,
                application_id: None,
                use_slash_commands: use_slash_commands(),
                num_threads: num_threads(),
                require_complete_command: require_complete_command(),
                use_reply: use_reply(),
                max_attachement_size: max_attachement_size(),
                max_name_length: max_name_length(),
            }
        }
        pub fn use_slash_commands() -> bool {
            false
        }
        pub fn num_threads() -> usize {
            1
        }
        pub fn require_complete_command() -> bool {
            false
        }
        pub fn use_reply() -> bool {
            true
        }
        pub fn max_attachement_size() -> u64 {
            1_000_000
        }
        pub fn max_name_length() -> usize {
            32
        }
    }
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default::auto_update_dsa_data")]
    pub auto_update_dsa_data: bool,
    #[serde(default = "default::dsa_rules::dsa_rules")]
    pub dsa_rules: ConfigDSARules,
    #[serde(default = "default::discord::discord")]
    pub discord: ConfigDiscord,
}

#[derive(Deserialize)]
pub struct ConfigDiscord {
    pub login_token: Option<String>,
    pub application_id: Option<u64>,
    #[serde(default = "default::discord::use_slash_commands")]
    pub use_slash_commands: bool,
    #[serde(default = "default::discord::num_threads")]
    pub num_threads: usize,
    #[serde(default = "default::discord::require_complete_command")]
    pub require_complete_command: bool,
    #[serde(default = "default::discord::use_reply")]
    pub use_reply: bool,
    #[serde(default = "default::discord::max_attachement_size")]
    pub max_attachement_size: u64,
    #[serde(default = "default::discord::max_name_length")]
    pub max_name_length: usize,
}
#[derive(Deserialize)]
pub struct ConfigDSARules {
    #[serde(default = "default::dsa_rules::crit_rules")]
    pub crit_rules: ConfigDSACritType,
}

#[derive(Deserialize)]
pub enum ConfigDSACritType {
    NoCrits,
    DefaultCrits,
    AlternativeCrits,
}

#[derive(Deserialize)]
pub struct DSAData {
    pub version: u64,
    pub talents: HashMap<String, TalentConfig>,
    pub combat_techniques: HashMap<String, CombatTechniqueConfig>,
    pub spells: HashMap<String, SpellConfig>,
}

#[derive(Deserialize)]
pub struct TalentConfig {
    pub attributes: Vec<String>,
}
#[derive(Deserialize)]
pub struct CombatTechniqueConfig {
    pub attributes: Vec<String>,
}
#[derive(Deserialize)]
pub struct SpellConfig {
    pub attributes: Vec<String>,
}

/*
A trait that handles reading (and creating default) configuration data
*/
pub trait AbstractConfig
where
    Self: DeserializeOwned,
{
    const DEFAULT_CONFIG: &'static str;
    const RELATIVE_PATH: &'static str;

    fn absolute_path() -> Result<PathBuf, Error> {
        let mut path = get_config_dir()?;
        path.push(Self::RELATIVE_PATH);
        Ok(path)
    }

    fn read() -> Result<Self, Error> {
        let path = Self::absolute_path()?;
        if Path::exists(&path) {
            let file = fs::File::open(&path)?;
            let reader = BufReader::new(file);
            let config: Self = serde_json::from_reader(reader)?;
            Ok(config)
        } else {
            Err(Error::new(
                format!(
                    "Missing file: {}",
                    path.to_str().unwrap_or("[Invalid Path]")
                ),
                ErrorType::IO(IOErrorType::MissingFile),
            ))
        }
    }

    fn create_default() -> Result<(), Error> {
        let path = Self::absolute_path()?;
        fs::write(path, Self::DEFAULT_CONFIG)?;
        Ok(())
    }

    fn get_or_create(output: &mut impl OutputWrapper) -> Result<Self, Error> {
        match Self::read() {
            Ok(config) => Ok(config),
            Err(e) => {
                if let ErrorType::IO(IOErrorType::MissingFile) = e.err_type() {
                    output.output_line(&format!(
                        "Creating default config (did not find file \"{}\")",
                        Self::absolute_path()?.to_str().unwrap_or("[Invalid Path]")
                    ));
                    Self::create_default()?;
                    Self::read()
                } else {
                    Err(e)
                }
            }
        }
    }
}

impl AbstractConfig for Config {
    const DEFAULT_CONFIG: &'static str = include_str!("default_config/config.json");
    const RELATIVE_PATH: &'static str = "config.json";
}

impl AbstractConfig for DSAData {
    const DEFAULT_CONFIG: &'static str = include_str!("default_config/dsa_data.json");
    const RELATIVE_PATH: &'static str = "dsa_data.json";
}

impl DSAData {
    /*
    Searches for a search term among the keys of the given hashmap
    '_' at the beginning or end of the search term marks the beginning/end of the name
    */
    pub fn match_search<'a, V>(
        entries: &'a HashMap<String, V>,
        search: &str,
    ) -> Result<(&'a str, &'a V), Error> {
        let mut found_entry: Option<(&str, &V)> = None;
        let mut search_trimmed: &str = &search.to_lowercase();
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
        for (name, entry) in entries {
            if name.contains(search_trimmed) {
                let mut matches_search = true;
                if search_at_beg && !name.starts_with(search_trimmed) {
                    matches_search = false;
                }
                if search_at_end && !name.ends_with(search_trimmed) {
                    matches_search = false;
                }

                if matches_search {
                    if let Some(found_entry) = found_entry {
                        return Err(Error::new(format!("Ambiguous identifier \"{}\": Matched \"{}\" and \"{}\".\nNote: You can use \"_\" to mark the beginning and/or end of the name.", search, found_entry.0, name),
                            ErrorType::InvalidInput(InputErrorType::InvalidArgument)));
                    } else {
                        found_entry = Some((name, entry));
                    }
                }
            }
        }
        if let Some(found_entry) = found_entry {
            Ok(found_entry)
        } else {
            Err(Error::new(
                format!("No matches found for \"{}\"", search),
                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
            ))
        }
    }

    pub fn check_replacement_needed(
        self,
        config: &Config,
        output: &mut impl OutputWrapper,
    ) -> DSAData {
        if config.auto_update_dsa_data && self.version < DSA_DATA_NEWEST_VERSION {
            match Self::create_default() {
                Err(e) => {
                    output.output_line(&format!(
                        "Error replacing dsa data with newer version: {}",
                        e
                    ));
                    self
                }
                Ok(()) => {
                    output.output_line(&"Replaced dsa data with newer version");
                    match Self::read() {
                        Err(_) => {
                            output.output_line(&"Error reading newly created dsa data, continuing with old version");
                            self
                        }
                        Ok(new_data) => new_data,
                    }
                }
            }
        } else {
            self
        }
    }
}

#[cfg(target_os = "linux")]
pub fn get_config_dir() -> Result<PathBuf, Error> {
    match env::var("DSA_CLI_CONFIG_DIR") {
        Err(_) => {}
        Ok(s) => {
            let mut path = PathBuf::new();
            path.push(s);
            fs::create_dir_all(&path)?;
            return Ok(path);
        }
    };

    let home = match env::var("HOME") {
        Ok(s) => s,
        Err(_) => {
            return Err(Error::new(
                "Could not read environment variable $HOME",
                ErrorType::IO(IOErrorType::MissingEnvironmentVariable),
            ));
        }
    };
    let mut path = PathBuf::new();
    path.push(home);
    path.push(".config");
    path.push("dsa-cli");
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[cfg(target_os = "windows")]
pub fn get_config_dir() -> Result<PathBuf, Error> {
    match env::var("DSA_CLI_CONFIG_DIR") {
        Err(_) => {}
        Ok(s) => {
            let mut path = PathBuf::new();
            path.push(s);
            fs::create_dir_all(&path)?;
            return Ok(path);
        }
    };

    let appdata = match env::var("appdata") {
        Ok(s) => s,
        Err(_) => {
            return Err(Error::new(
                "Could not read environment variable \"appdata\"",
                ErrorType::IO(IOErrorType::MissingEnvironmentVariable),
            ));
        }
    };
    let mut path = PathBuf::new();
    path.push(appdata);
    path.push("dsa-cli");
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[cfg(target_os = "macos")]
pub fn get_config_dir() -> Result<path::PathBuf, Error> {
    match env::var("DSA_CLI_CONFIG_DIR") {
        Err(_) => {}
        Ok(s) => {
            let mut path = path::PathBuf::new();
            path.push(s);
            fs::create_dir_all(&path)?;
            Ok(path);
        }
    };

    let appdata = match env::var("HOME") {
        Ok(s) => s,
        Err(_) => {
            return Err(Error::new(
                "Could not read environment variable $HOME",
                ErrorType::IO(IOErrorType::MissingEnvironmentVariable),
            ));
        }
    };
    let mut path = PathBuf::new();
    path.push(appdata);
    path.push("Library");
    path.push("Application Support");
    path.push("dsa-cli");
    fs::create_dir_all(&path)?;
    Ok(path)
}
