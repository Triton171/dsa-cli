use super::config::Config;
use super::util::{Error, ErrorType};
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Deserialize)]
pub struct Character {
    name: String,
    attributes: Vec<CharacterAttribute>,
    skills: Option<Vec<CharacterSkill>>,
    combattechniques: Option<Vec<CharacterCombatTechnique>>,
    spells: Option<Vec<CharacterSpell>>,
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

#[derive(Deserialize)]
pub struct CharacterCombatTechnique {
    id: String,
    level: i64,
}

#[derive(Deserialize)]
pub struct CharacterSpell {
    id: String,
    level: Option<i64>,
}

impl Character {
    pub fn loaded_character(config: &Config) -> Result<Option<Character>, Error> {
        let char_path = match &config.loaded_character_path {
            Some(p) => p,
            None => {
                return Ok(None);
            }
        };
        match Character::from_file(Path::new(char_path)) {
            Ok(c) => Ok(Some(c)),
            Err(e) => Err(e),
        }
    }

    pub fn from_file(path: &Path) -> Result<Character, Error> {
        let char_file = match File::open(path) {
            Ok(f) => f,
            Err(_) => {
                return Err(Error::new(
                    "Unable to open character file",
                    ErrorType::FileSystemError,
                ));
            }
        };
        let reader = BufReader::new(char_file);
        match serde_json::from_reader(reader) {
            Ok(c) => Ok(c),
            Err(e) => Err(Error::new(
                format!("Invalid character format, detected at line {}", e.line()),
                ErrorType::InvalidFormat,
            )),
        }
    }

    pub fn load(path: &str, config: &mut Config) -> Result<Character, Error> {
        let p = Path::new(path);
        let p = match std::fs::canonicalize(p) {
            Ok(p) => p,
            Err(_) => {
                return Err(Error::new(
                    "Unable to resolve character path",
                    ErrorType::FileSystemError,
                ));
            }
        };
        config.loaded_character_path = Some(p.to_str().unwrap().to_owned());
        match Character::loaded_character(config) {
            Ok(Some(c)) => Ok(c),
            Ok(None) => Err(Error::new(
                "Character was not loaded correctly",
                ErrorType::Unknown,
            )),
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
        let skills = match &self.skills {
            Some(skills) => skills,
            None => {
                return 0;
            }
        };
        for skill in skills {
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

    pub fn get_attack_level(&self, technique_id: &str) -> i64 {
        let techniques = match &self.combattechniques {
            Some(techniques) => techniques,
            None => {
                return 0;
            }
        };
        for technique in techniques {
            if technique.id.eq_ignore_ascii_case(technique_id) {
                let mut_level = self.get_attribute_level("mut");
                return technique.level + std::cmp::max(0, (mut_level - 8) / 3);
            }
        }
        0
    }

    pub fn get_spell_level(&self, spell_id: &str) -> i64 {
        let spells = match &self.spells {
            Some(spells) => spells,
            None => {
                return 0;
            }
        };
        for spell in spells {
            if spell.id.eq_ignore_ascii_case(spell_id) {
                return spell.level.unwrap_or(0);
            }
        }
        0
    }

    pub fn get_dodge_level(&self) -> i64 {
        for attr in &self.attributes {
            if attr.id.eq_ignore_ascii_case("gewandtheit") {
                return attr.level / 2;
            }
        }
        0
    }

    pub fn get_initiative_level(&self) -> i64 {
        let mut attr_mut = 0;
        let mut attr_gew = 0;
        for attr in &self.attributes {
            if attr.id.eq_ignore_ascii_case("mut") {
                attr_mut = attr.level;
            } else if attr.id.eq_ignore_ascii_case("gewandtheit") {
                attr_gew = attr.level;
            }
        }
        (attr_mut + attr_gew) / 2
    }
}
