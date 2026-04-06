use crate::config::Config;

use super::{character::Character, config};
use anyhow::{ensure, Context};
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, collections::HashMap, fmt::Display};
use std::{future::Future, path::PathBuf};
use tokio::{fs, io::AsyncWriteExt, sync::broadcast::error::RecvError};

static EMPTY_CHARACTER_LIST: Vec<CharacterInfo> = Vec::new();

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterId(u64);

impl From<u64> for CharacterId {
    fn from(value: u64) -> Self {
        CharacterId(value)
    }
}
impl Display for CharacterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub character_id: CharacterId,
    pub name: String,
    pub selected: bool,
}

#[derive(Serialize, Deserialize)]
struct CharacterList {
    // IMPROVE: Generate UUIDs instead of numbering the characters sequentially
    next_character_id: CharacterId,
    characters: HashMap<u64, Vec<CharacterInfo>>,
}

impl CharacterList {
    fn new() -> Self {
        CharacterList {
            next_character_id: CharacterId(0),
            characters: HashMap::new(),
        }
    }
}

pub struct CharacterManager {
    characters: CharacterList,
}

impl CharacterManager {
    /*
    Initializes the character manager by:
    * If a character list exists, it is simply read
    * If there is no character list but some characters in the old format, they are imported into a new character list
    * If there are no characters, a blank character list is created and stored
    */
    pub async fn init(config: &Config) -> anyhow::Result<Self> {
        let config_path = config::get_config_dir()?;
        let mut character_list_path = config_path.clone();
        character_list_path.push("discord_character_list");
        if character_list_path.exists() {
            let data = fs::read_to_string(&character_list_path).await?;
            Ok(CharacterManager {
                characters: serde_json::from_str(&data)?,
            })
        } else {
            let mut folder_path = config_path;
            folder_path.push("discord_characters");
            let character_manager = CharacterManager {
                characters: CharacterList::new(),
            };
            character_manager.write_character_list().await?;
            Ok(character_manager)
        }
    }

    /*
    Adds a character to the local storage. If a character with the same name already exists, it is replaced
    Returns a bool indicating, if a character was replaced and the character name
    */
    pub async fn add_character(
        &mut self,
        user_id: u64,
        raw_character: Vec<u8>,
        character_id: Option<CharacterId>,
        config: &Config,
    ) -> anyhow::Result<Option<String>> {
        let character_str = String::from_utf8(raw_character)?;
        let name = Character::from_str(&character_str)?
            .get_name()
            .trim()
            .to_string();

        let user_characters = self.characters.characters.entry(user_id).or_default();
        if user_characters.len() >= config.discord.max_num_characters {
            return Ok(Some("Exceeded maximum number of characters, use the \"remove\" command to free up space.".to_string()));
        }
        if name.len() > config.discord.max_name_length {
            return Ok(Some("Character name exceeds maximum length".to_string()));
        }

        let id = match character_id {
            Some(id) => {
                let current_info = user_characters
                    .iter_mut()
                    .find(|c| c.character_id == id)
                    .context("Did not find character that should be replaced")?;
                current_info.name = name;
                id
            }
            None => {
                let id = self.characters.next_character_id;
                ensure!(
                    user_characters
                        .iter()
                        .find(|c| c.character_id == id)
                        .is_none(),
                    "Character ID is already in use"
                );
                user_characters.push(CharacterInfo {
                    character_id: id,
                    name,
                    selected: false,
                });
                self.characters.next_character_id =
                    CharacterId(self.characters.next_character_id.0 + 1);
                id
            }
        };
        self.write_character_list().await?;

        let path = get_character_path(id).await?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .await?;
        file.write_all(character_str.as_bytes()).await?;
        file.flush().await?;
        Ok(None)
    }

    /*
    Deletes all matching characters from storage and returns a list of their names
    */
    pub async fn delete_character(
        &mut self,
        user_id: u64,
        character_id: CharacterId,
    ) -> anyhow::Result<()> {
        let user_characters = self
            .characters
            .characters
            .get_mut(&user_id)
            .context("Did not find any characters for this user")?;
        ensure!(
            user_characters
                .iter()
                .find(|c| c.character_id == character_id)
                .is_some(),
            "Did not find character to delete"
        );
        user_characters.retain(|c| c.character_id != character_id);
        self.write_character_list().await?;
        Ok(())
    }

    pub fn get_characters(&self, user_id: u64) -> &Vec<CharacterInfo> {
        if let Some(user_characters) = self.characters.characters.get(&user_id) {
            &user_characters
        } else {
            &EMPTY_CHARACTER_LIST
        }
    }

    pub async fn get_character(&self, id: CharacterId) -> anyhow::Result<Character> {
        // TODO: Add caching
        let path = get_character_path(id).await?;
        Character::from_file(&path).await
    }

    async fn write_character_list(&self) -> anyhow::Result<()> {
        let mut path = config::get_config_dir()?;
        path.push("discord_character_list");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .await?;
        let data = serde_json::to_string(&self.characters)?;
        file.write_all(data.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }
}

async fn get_character_path(character_id: CharacterId) -> anyhow::Result<PathBuf> {
    let mut path = config::get_config_dir()?;
    path.push("discord_characters");
    fs::create_dir_all(&path).await?;
    path.push(character_id.0.to_string());
    Ok(path)
}
