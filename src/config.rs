use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

const DSA_DATA_NEWEST_VERSION: u64 = 10;

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
            super::super::ConfigDSACritType::Default
        }
    }
    pub mod discord {
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
        pub fn max_num_characters() -> usize {
            5
        }
    }
    pub mod dsa_data {
        pub fn combat_technique_ranged() -> bool {
            false
        }
    }
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default::auto_update_dsa_data")]
    pub auto_update_dsa_data: bool,
    #[serde(default = "default::dsa_rules::dsa_rules")]
    pub dsa_rules: ConfigDSARules,
    pub discord: ConfigDiscord,
}

// TODO: Remove redundant config options
#[derive(Deserialize)]
pub struct ConfigDiscord {
    pub login_token: String,
    // Should only be used for testing, commands will only be registered for that specific guild and not globally
    pub test_in_guild_id: Option<u64>,
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
    #[serde(default = "default::discord::max_num_characters")]
    pub max_num_characters: usize,
}
#[derive(Deserialize)]
pub struct ConfigDSARules {
    #[serde(default = "default::dsa_rules::crit_rules")]
    pub crit_rules: ConfigDSACritType,
}

#[derive(Deserialize)]
pub enum ConfigDSACritType {
    None,
    Default,
    Alternative,
}

#[derive(Deserialize)]
pub struct DSAData {
    pub version: u64,
    pub attributes: HashMap<String, AttributeConfig>,
    pub talents: HashMap<String, TalentConfig>,
    pub combat_techniques: HashMap<String, CombatTechniqueConfig>,
    pub spells: HashMap<String, SpellConfig>,
    pub chants: HashMap<String, ChantConfig>,
}

#[derive(Deserialize)]
pub struct AttributeConfig {
    pub short_name: String,
}
#[derive(Deserialize)]
pub struct TalentConfig {
    pub attributes: Vec<String>,
}
#[derive(Deserialize)]
pub struct CombatTechniqueConfig {
    pub attributes: Vec<String>,
    #[serde(default = "default::dsa_data::combat_technique_ranged")]
    pub ranged: bool,
}
#[derive(Deserialize)]
pub struct SpellConfig {
    pub attributes: Vec<String>,
}
#[derive(Deserialize)]
pub struct ChantConfig {
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

    fn absolute_path() -> anyhow::Result<PathBuf> {
        let mut path = get_config_dir()?;
        path.push(Self::RELATIVE_PATH);
        Ok(path)
    }

    fn read() -> anyhow::Result<Self> {
        let path = Self::absolute_path()?;
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let config: Self = serde_json::from_reader(reader)?;
        Ok(config)
    }

    fn create_default() -> anyhow::Result<()> {
        let path = Self::absolute_path()?;
        fs::write(path, Self::DEFAULT_CONFIG)?;
        Ok(())
    }

    fn get_or_create() -> anyhow::Result<Self> {
        let path = Self::absolute_path()?;
        if !std::fs::exists(&path)? {
            println!(
                "Creating default config (did not find file \"{}\")",
                path.to_str().unwrap_or("[Invalid Path]")
            );
            Self::create_default()?;
        }
        Self::read()
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

pub enum MatchSearchResult<'a, V> {
    Success((&'a str, V)),
    NoUniqueMatch(String),
}
impl DSAData {
    /*
    Searches for a search term among the (first) elements of the iterator
    '_' at the beginning or end of the search term marks the beginning/end of the name
    */
    pub fn match_search<'a, V>(
        entries: impl Iterator<Item = (&'a String, V)>,
        search: &str,
    ) -> MatchSearchResult<'a, V> {
        let mut found_entry: Option<(&str, V)> = None;
        let mut search_trimmed: &str = &search
            .to_lowercase()
            .replace('ä', "ae")
            .replace('ö', "oe")
            .replace('ü', "ue");
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
            let lower_name = name.to_lowercase();
            if lower_name.contains(search_trimmed) {
                let mut matches_search = true;
                if search_at_beg && !lower_name.starts_with(search_trimmed) {
                    matches_search = false;
                }
                if search_at_end && !lower_name.ends_with(search_trimmed) {
                    matches_search = false;
                }

                if matches_search {
                    if let Some(found_entry) = found_entry {
                        return MatchSearchResult::NoUniqueMatch(format!("Ambiguous identifier \"{}\": Matched \"{}\" and \"{}\".\nNote: You can use \"_\" to mark the beginning and/or end of the name.", search, found_entry.0, name));
                    } else {
                        found_entry = Some((name, entry));
                    }
                }
            }
        }
        if let Some(found_entry) = found_entry {
            MatchSearchResult::Success(found_entry)
        } else {
            MatchSearchResult::NoUniqueMatch(format!("No matches found for \"{}\"", search))
        }
    }

    pub fn get_attr_short_name<'a>(&'a self, attribute: &str) -> &'a str {
        self.attributes.get(attribute).unwrap().short_name.as_str()
    }

    pub fn check_replacement_needed(self, config: &Config) -> anyhow::Result<DSAData> {
        if config.auto_update_dsa_data && self.version < DSA_DATA_NEWEST_VERSION {
            if let Err(e) =
                Self::create_default().context("Error replacing dsa data with newer version")
            {
                println!("{:?}", e);
                return Ok(self);
            }
            Ok(Self::read()?)
        } else {
            Ok(self)
        }
    }
}

#[cfg(target_os = "linux")]
pub fn get_config_dir() -> anyhow::Result<PathBuf> {
    match env::var("DSA_CLI_CONFIG_DIR") {
        Err(_) => {}
        Ok(s) => {
            let mut path = PathBuf::new();
            path.push(s);
            fs::create_dir_all(&path)?;
            return Ok(path);
        }
    };

    let home = env::var("HOME")?;
    let mut path = PathBuf::new();
    path.push(home);
    path.push(".config");
    path.push("dsa-cli");
    fs::create_dir_all(&path)?;
    Ok(path)
}

#[cfg(target_os = "windows")]
pub fn get_config_dir() -> anyhow::Result<PathBuf> {
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
pub fn get_config_dir() -> anyhow::Result<path::PathBuf> {
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
