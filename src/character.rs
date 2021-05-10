use super::config;
use super::util::{Error, ErrorType};
use serde::Deserialize;
use std::path::Path;
use tokio::fs;

const LOADED_CHARACTER_FILE: &'static str = "loaded_character";

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
    pub async fn loaded_character() -> Result<Option<Character>, Error> {
        let mut path = config::get_config_dir()?;
        path.push(LOADED_CHARACTER_FILE);
        if Path::exists(&path) {
            let char_path = std::fs::read_to_string(&path)?;
            let char_path = Path::new(&char_path);
            let character = Self::from_file(&char_path).await?;
            Ok(Some(character))
        } else {
            Ok(None)
        }
    }

    pub async fn from_file(path: &Path) -> Result<Character, Error> {
        let json_data = fs::read_to_string(path).await?;
        let character: Character = serde_json::from_str(&json_data)?;
        Ok(character)
    }

    pub async fn load(path: &str) -> Result<Character, Error> {
        let character_path = Path::new(path);
        let character_path = fs::canonicalize(character_path).await?;
        let mut path = config::get_config_dir()?;
        path.push(LOADED_CHARACTER_FILE);
        fs::write(&path, character_path.to_str().unwrap()).await?;
        match Character::loaded_character().await {
            Ok(Some(c)) => Ok(c),
            Ok(None) => Err(Error::new(
                "Character was not loaded correctly",
                ErrorType::Unknown,
            )),
            Err(e) => Err(e),
        }
    }

    pub async fn unload() -> Result<(), Error> {
        let mut path = config::get_config_dir()?;
        path.push(LOADED_CHARACTER_FILE);
        fs::remove_file(&path).await?;
        Ok(())
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

    fn get_technique_level(&self, technique_id: &str) -> i64 {
        let techniques = match &self.combattechniques {
            Some(techniques) => techniques,
            None => {
                return 6;
            }
        };
        for technique in techniques {
            if technique.id.eq_ignore_ascii_case(technique_id) {
                return technique.level;
            }
        }
        6
    }

    pub fn get_attack_level(&self, technique_id: &str) -> i64 {
        let mut_level = self.get_attribute_level("mut");
        self.get_technique_level(technique_id) + std::cmp::max(0, (mut_level - 8) / 3)
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

    pub fn get_parry_level(&self, technique_id: &str, technique_attributes: &[String]) -> i64 {
        let technique_level = self.get_technique_level(technique_id);

        let mut max_attr = 0;
        for attr in &self.attributes {
            for bonus_attr in technique_attributes {
                if attr.id.eq_ignore_ascii_case(bonus_attr) {
                    max_attr = std::cmp::max(max_attr, attr.level);
                }
            }
        }
        technique_level / 2 + std::cmp::max(0, (max_attr - 8) / 3)
    }
}
