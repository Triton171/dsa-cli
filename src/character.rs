use super::config::Config;
use super::util::IOError;
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
pub struct Character {
    name: String,
    attributes: Vec<CharacterAttribute>,
    skills: Vec<CharacterSkill>,
}

#[derive(Deserialize)]
pub struct CharacterSkill {
    id: String,
    level: i64,
}

#[derive(Deserialize)]
pub struct CharacterAttribute {
    id: String,
    level: i64,
}

impl Character {
    pub fn loaded_character(config: &Config) -> Result<Option<Character>, IOError> {
        let char_path = match &config.loaded_character_path {
            Some(p) => p,
            None => {
                return Ok(None);
            }
        };

        let char_file = match File::open(PathBuf::from(char_path)) {
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
            ))),
        }
    }

    pub fn load(path: &str, config: &mut Config) -> Result<Character, IOError> {
        let p = Path::new(path);
        let p = match std::fs::canonicalize(p) {
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
