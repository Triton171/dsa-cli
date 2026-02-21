use crate::util::InputErrorType;

use super::config;
use super::util::{Error, ErrorType};
use serde::{Deserialize, Deserializer};
use std::path::Path;
use tokio::fs;

const LOADED_CHARACTER_FILE: &str = "loaded_character";

mod default {
    pub fn skills() -> Vec<super::CharacterSkill> {
        Vec::new()
    }
    pub fn combattechniques() -> Vec<super::CharacterCombatTechnique> {
        Vec::new()
    }
    pub fn spells() -> Vec<super::CharacterSpell> {
        Vec::new()
    }
    pub fn chants() -> Vec<super::CharacterChant> {
        Vec::new()
    }
    pub fn specialabilities() -> Vec<super::SpecialAbility> {
        Vec::new()
    }
}

#[derive(Deserialize)]
pub struct Character {
    name: String,
    attributes: Vec<CharacterAttribute>,
    #[serde(default = "default::skills")]
    skills: Vec<CharacterSkill>,
    #[serde(default = "default::combattechniques")]
    combattechniques: Vec<CharacterCombatTechnique>,
    #[serde(default = "default::spells")]
    spells: Vec<CharacterSpell>,
    #[serde(default = "default::chants")]
    chants: Vec<CharacterChant>,
    #[serde(default = "default::specialabilities")]
    specialabilities: Vec<SpecialAbility>,
}

#[derive(Deserialize)]
pub enum IdOrCustomInfo {
    #[serde(rename = "id")]
    Id(String),
    #[serde(rename = "ruleelement")]
    RuleElement {
        name: String,
        #[serde(rename = "check")]
        attributes: Vec<String>,
    },
}

impl IdOrCustomInfo {
    fn matches_name(&self, input_name: &str) -> bool {
        match self {
            IdOrCustomInfo::Id(id) => id.eq_ignore_ascii_case(input_name),
            IdOrCustomInfo::RuleElement {
                name,
                attributes: _,
            } => name.eq_ignore_ascii_case(input_name),
        }
    }
}

#[derive(Deserialize)]
pub enum IdOrCustomId {
    #[serde(rename = "id")]
    Id(String),
    #[serde(rename = "ruleelement")]
    RuleElement { name: String },
}

impl IdOrCustomId {
    fn matches_name(&self, input_name: &str) -> bool {
        match self {
            IdOrCustomId::Id(id) => id.eq_ignore_ascii_case(input_name),
            IdOrCustomId::RuleElement { name } => name.eq_ignore_ascii_case(input_name),
        }
    }
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
    #[serde(flatten)]
    id_or_custom_id: IdOrCustomId,
    level: i64,
}

#[derive(Deserialize)]
pub struct CharacterSpell {
    #[serde(flatten)]
    id_or_rule_element: IdOrCustomInfo,
    level: Option<i64>,
}

#[derive(Deserialize)]
pub struct CharacterChant {
    #[serde(flatten)]
    id_or_rule_element: IdOrCustomInfo,
    level: Option<i64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SpecialAbility {
    SkillSpecialization(SkillSpecialization),
    FavoriteSpell(FavoriteSpell),
    FavoriteIncantation(FavoriteIncantation),
    Generic {},
}
pub struct SkillSpecialization {
    skill: String,
    application: String,
}
pub struct FavoriteSpell {
    spell: String,
}
pub struct FavoriteIncantation {
    incantation: String,
}
impl<'de> Deserialize<'de> for SkillSpecialization {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Input {
            #[serde(rename = "id")]
            _id: InputId,
            variant: InputVariant,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum InputId {
            FertigkeitsSpezialisierung,
        }
        #[derive(Deserialize)]
        struct InputVariant {
            skill: String,
            application: String,
        }

        let input = Input::deserialize(deserializer)?;
        Ok(SkillSpecialization {
            skill: input.variant.skill,
            application: input.variant.application,
        })
    }
}

fn deserialize_favorite_spell_or_incant<'de, D, InputId: Deserialize<'de>>(
    deserializer: D,
) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Input<Id> {
        #[serde(rename = "id")]
        _id: Id,
        variant: InputVariant,
    }
    #[derive(Deserialize)]
    struct InputVariant {
        ruleelement: InputRuleElement,
    }
    #[derive(Deserialize)]
    struct InputRuleElement {
        id: String,
    }
    let input = Input::<InputId>::deserialize(deserializer)?;
    Ok(input.variant.ruleelement.id)
}

impl<'de> Deserialize<'de> for FavoriteSpell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum InputId {
            LieblingsZauber,
        }
        let spell = deserialize_favorite_spell_or_incant::<D, InputId>(deserializer)?;
        Ok(FavoriteSpell { spell: spell })
    }
}
impl<'de> Deserialize<'de> for FavoriteIncantation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum InputId {
            LieblingsLiturgie,
        }
        let incant = deserialize_favorite_spell_or_incant::<D, InputId>(deserializer)?;
        Ok(FavoriteIncantation {
            incantation: incant,
        })
    }
}

impl Character {
    pub async fn loaded_character() -> Result<Option<Character>, Error> {
        let mut path = config::get_config_dir()?;
        path.push(LOADED_CHARACTER_FILE);
        if Path::exists(&path) {
            let char_path = std::fs::read_to_string(&path)?;
            let char_path = Path::new(&char_path);
            let character = Self::from_file(char_path).await?;
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

    pub fn from_raw(raw: Vec<u8>) -> Result<Character, Error> {
        let json_data = match String::from_utf8(raw) {
            Err(_) => {
                return Err(Error::new(
                    "Character data contains invalid UTF-8",
                    ErrorType::InvalidInput(InputErrorType::InvalidFormat),
                ));
            }
            Ok(s) => s,
        };
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

    fn get_technique_level(&self, technique_id: &str) -> i64 {
        for technique in &self.combattechniques {
            if technique.id_or_custom_id.matches_name(technique_id) {
                return technique.level;
            }
        }
        6
    }

    pub fn get_attack_level(&self, technique_id: &str, ranged: bool) -> i64 {
        let mut_level = if ranged {
            self.get_attribute_level("fingerfertigkeit")
        } else {
            self.get_attribute_level("mut")
        };
        self.get_technique_level(technique_id) + std::cmp::max(0, (mut_level - 8) / 3)
    }

    pub fn get_spell_level(&self, spell_id: &str) -> i64 {
        for spell in &self.spells {
            if spell.id_or_rule_element.matches_name(spell_id) {
                return spell.level.unwrap_or(0);
            }
        }
        0
    }

    pub fn get_chant_level(&self, chant_id: &str) -> i64 {
        for chant in &self.chants {
            if chant.id_or_rule_element.matches_name(chant_id) {
                return chant.level.unwrap_or(0);
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
        // We add 1 to the technique_level since it has to be rounded up
        (technique_level + 1) / 2 + std::cmp::max(0, (max_attr - 8) / 3)
    }

    pub fn get_custom_techniques(&self) -> impl Iterator<Item = &String> {
        self.combattechniques
            .iter()
            .filter_map(|t| match &t.id_or_custom_id {
                IdOrCustomId::Id(_) => None,
                IdOrCustomId::RuleElement { name } => Some(name),
            })
    }

    pub fn get_custom_spells(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
        self.spells
            .iter()
            .filter_map(|s| match &s.id_or_rule_element {
                IdOrCustomInfo::Id(_) => None,
                IdOrCustomInfo::RuleElement { name, attributes } => Some((name, attributes)),
            })
    }

    pub fn get_custom_chants(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
        self.chants
            .iter()
            .filter_map(|c| match &c.id_or_rule_element {
                IdOrCustomInfo::Id(_) => None,
                IdOrCustomInfo::RuleElement { name, attributes } => Some((name, attributes)),
            })
    }

    pub fn is_favorite_spell(&self, spell_name: &str) -> bool {
        self.specialabilities.iter().any(|s| {
            if let SpecialAbility::FavoriteSpell(FavoriteSpell { spell }) = s {
                *spell == *spell_name
            } else {
                false
            }
        })
    }

    pub fn is_favorite_incantation(&self, incantation_name: &str) -> bool {
        self.specialabilities.iter().any(|s| {
            if let SpecialAbility::FavoriteIncantation(FavoriteIncantation { incantation }) = s {
                *incantation == *incantation_name
            } else {
                false
            }
        })
    }

    pub fn get_skill_specialization_application(&self, skill_name: &str) -> Option<&str> {
        self.specialabilities.iter().find_map(|s| {
            if let SpecialAbility::SkillSpecialization(s) = s {
                if s.skill == skill_name {
                    return Some(s.application.as_str());
                }
            }
            None
        })
    }
}
