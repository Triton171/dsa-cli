use crate::{config::Config, util::IOErrorType};

use super::{
    character::Character,
    config,
    discord_commands::CommandContext,
    util::{Error, ErrorType, InputErrorType},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{borrow::Borrow, collections::HashMap};
use tokio::{fs, io::AsyncWriteExt};

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterId(u64);

#[derive(Serialize, Deserialize)]
struct CharacterInfo {
    character_id: CharacterId,
    name: String,
    selected: bool,
}

#[derive(Serialize, Deserialize)]
struct CharacterList {
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
    pub async fn init(config: &Config) -> Result<Self, Error> {
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
            let mut character_manager = CharacterManager {
                characters: CharacterList::new(),
            };
            if folder_path.exists() {
                // Migrate old characters to the new storage system
                println!("Migrating old characters to the new character storage system");
                let mut files = fs::read_dir(&folder_path).await?;
                let mut characters: Vec<(u64, Vec<u8>)> = Vec::new();
                while let Some(f) = files.next_entry().await? {
                    let os_file_name = f.file_name();
                    let file_name = match os_file_name.to_str() {
                        Some(s) => s,
                        None => {
                            return Err(Error::new(
                                "Invalid file name encountered in discord_characters folder",
                                ErrorType::IO(IOErrorType::Unknown),
                            ));
                        }
                    };
                    let id: u64 = match file_name.parse() {
                        Ok(num) => num,
                        Err(_) => {
                            return Err(Error::new(
                                "Unable to parse discord character file name as id",
                                ErrorType::IO(IOErrorType::Unknown),
                            ));
                        }
                    };
                    characters.push((id, fs::read(&f.path()).await?));
                }
                fs::remove_dir_all(&folder_path).await?;
                fs::create_dir(&folder_path).await?;
                for (id, raw_character) in characters {
                    if let Err(e) = character_manager
                        .add_character(id, raw_character, config)
                        .await
                    {
                        println!("Error migrating character: {}", e);
                    }
                }
            }
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
        config: &Config,
    ) -> Result<(bool, String), Error> {
        let id = self.characters.next_character_id;
        let path = get_character_path(id).await?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .await?;
        file.write(&raw_character).await?;
        file.flush().await?;
        let name = Character::from_file(&path)
            .await?
            .get_name()
            .trim()
            .to_string();
        if name.len() > config.discord.max_name_length {
            return Err(Error::new(
                "Character name exceeds maximum length",
                ErrorType::InvalidInput(InputErrorType::CharacterNameTooLong),
            ));
        }

        if let Some(user_characters) = self.characters.characters.get_mut(&user_id) {
            for character in user_characters.iter() {
                // Replace a character with the same name
                if character.name == name {
                    let old_path = get_character_path(character.character_id).await?;
                    fs::rename(&path, &old_path).await?;
                    return Ok((true, name));
                }
            }
            if user_characters.len() >= config.discord.max_num_characters {
                return Err(Error::new("Exceeded maximum number of characters, use the \"remove\" command to free up space.", ErrorType::InvalidInput(InputErrorType::TooManyCharacters)));
            }
            let info = CharacterInfo {
                character_id: id,
                name: name.clone(),
                // Set the new character as selected if there is currently no selected character
                selected: user_characters.iter().all(|c| !c.selected),
            };
            user_characters.push(info);
        } else {
            let info = CharacterInfo {
                character_id: id,
                name: name.clone(),
                selected: true,
            };
            self.characters.characters.insert(user_id, vec![info]);
        }
        self.characters.next_character_id = CharacterId(id.0 + 1);
        self.write_character_list().await?;
        Ok((false, name))
    }

    /*
    Deletes all matching characters from storage and returns a list of their names
    */
    pub async fn delete_character(
        &mut self,
        user_id: u64,
        name: impl Borrow<str>,
    ) -> Result<Vec<String>, Error> {
        let name = name.borrow().trim().to_ascii_lowercase();
        if let Some(user_characters) = self.characters.characters.get_mut(&user_id) {
            let mut removed_names: Vec<String> = Vec::new();
            for c in user_characters
                .iter()
                .filter(|c| c.name.to_ascii_lowercase().contains(&name))
            {
                let path = get_character_path(c.character_id).await?;
                fs::remove_file(path).await?;
                removed_names.push(c.name.clone());
            }
            user_characters.retain(|c| !c.name.to_ascii_lowercase().contains(&name));
            self.write_character_list().await?;
            Ok(removed_names)
        } else {
            Ok(Vec::new())
        }
    }

    /*
    Returns the names for the currently selected character (if any) and all of the other characters
    */
    pub fn list_characters(&self, user_id: u64) -> (Option<String>, Vec<String>) {
        if let Some(user_characters) = self.characters.characters.get(&user_id) {
            let selected = user_characters
                .iter()
                .filter(|c| c.selected)
                .map(|c| c.name.clone())
                .next();
            let non_selected = user_characters
                .iter()
                .filter(|c| !c.selected)
                .map(|c| c.name.clone())
                .collect();
            (selected, non_selected)
        } else {
            (None, Vec::new())
        }
    }

    pub async fn select_character(
        &mut self,
        user_id: u64,
        name: impl Borrow<str>,
    ) -> Result<String, Error> {
        if let Some(user_characters) = self.characters.characters.get_mut(&user_id) {
            let name = name.borrow().trim().to_ascii_lowercase();
            let mut matching_characters = user_characters
                .iter_mut()
                .filter(|c| c.name.to_ascii_lowercase().contains(&name));
            if let Some(c) = matching_characters.next() {
                if let Some(c2) = matching_characters.next() {
                    Err(Error::new(
                        format!("Ambiguous name, matches \"{}\" and \"{}\"", c.name, c2.name),
                        ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                    ))
                } else {
                    let selected_id = c.character_id;
                    let selected_name = c.name.clone();
                    for c in user_characters {
                        c.selected = c.character_id == selected_id;
                    }
                    self.write_character_list().await?;
                    Ok(selected_name)
                }
            } else {
                Err(Error::new(
                    "No matching character found",
                    ErrorType::InvalidInput(InputErrorType::MissingCharacter),
                ))
            }
        } else {
            Err(Error::new(
                "No character found for your discord account",
                ErrorType::InvalidInput(InputErrorType::MissingCharacter),
            ))
        }
    }

    pub async fn find_character(
        &self,
        ctx: &impl CommandContext,
        name: Option<impl Borrow<str>>,
    ) -> Result<CharacterId, Error> {
        let sender_id = *ctx.sender()?.as_u64();
        match name {
            None => {
                // Use the selected character for this discord account
                if let Some(user_characters) = self.characters.characters.get(&sender_id) {
                    if let Some(character) = user_characters.iter().find(|c| c.selected) {
                        Ok(character.character_id)
                    } else {
                        Err(Error::new(
                            "Error getting character: No character is currently selected",
                            ErrorType::InvalidInput(InputErrorType::MissingCharacter),
                        ))
                    }
                } else {
                    Err(Error::new(
                        "Error getting character: No character found for this discord account",
                        ErrorType::InvalidInput(InputErrorType::MissingCharacter),
                    ))
                }
            }
            Some(name) => {
                let name = name.borrow().trim().to_ascii_lowercase();
                // First, look for a character matching the name in the senders discord account
                if let Some(user_characters) = self.characters.characters.get(&sender_id) {
                    let mut matching_characters = user_characters
                        .iter()
                        .filter(|c| c.name.to_ascii_lowercase().contains(&name));
                    if let Some(c) = matching_characters.next() {
                        if let Some(c2) = matching_characters.next() {
                            return Err(Error::new(
                                format!(
                                    "Ambiguous character name, matches \"{}\" and \"{}\"",
                                    c.name, c2.name
                                ),
                                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                            ));
                        }
                        return Ok(c.character_id);
                    }
                }
                // If no matching character was found for the user, check all the users in the channel
                let mut matching_character: Option<&CharacterInfo> = None;
                for user_id in ctx
                    .members_in_channel()
                    .await?
                    .iter()
                    .map(|m| m.user.id.as_u64())
                {
                    if let Some(user_characters) = self.characters.characters.get(user_id) {
                        for c in user_characters.iter() {
                            if c.name.to_ascii_lowercase().contains(&name) {
                                if let Some(c0) = matching_character {
                                    return Err(Error::new(
                                        format!(
                                            "Ambiguous character name, matches \"{}\" and \"{}\"",
                                            c0.name, c.name
                                        ),
                                        ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                                    ));
                                } else {
                                    matching_character = Some(c);
                                }
                            }
                        }
                    }
                }

                if let Some(c) = matching_character {
                    Ok(c.character_id)
                } else {
                    Err(Error::new(
                        "Error getting character: No matching character found",
                        ErrorType::InvalidInput(InputErrorType::MissingCharacter),
                    ))
                }
            }
        }
    }

    pub async fn find_character_for_user(
        &self,
        user_id: u64,
        name: Option<impl Borrow<str>>,
    ) -> Result<CharacterId, Error> {
        if let Some(user_characters) = self.characters.characters.get(&user_id) {
            match name {
                None => {
                    // Use the selected character for this discord account
                    if let Some(character) = user_characters.iter().find(|c| c.selected) {
                        Ok(character.character_id)
                    } else {
                        Err(Error::new(
                            "Error getting character: No character is currently selected",
                            ErrorType::InvalidInput(InputErrorType::MissingCharacter),
                        ))
                    }
                }
                Some(name) => {
                    let name = name.borrow().trim().to_ascii_lowercase();
                    // Look for a character matching the name in the specified discord account
                    let mut matching_characters = user_characters
                        .iter()
                        .filter(|c| c.name.to_ascii_lowercase().contains(&name));
                    if let Some(c) = matching_characters.next() {
                        if let Some(c2) = matching_characters.next() {
                            return Err(Error::new(
                                format!(
                                    "Ambiguous character name, matches \"{}\" and \"{}\"",
                                    c.name, c2.name
                                ),
                                ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                            ));
                        }
                        return Ok(c.character_id);
                    }
                    Err(Error::new(
                        "Error getting character: No matching character found",
                        ErrorType::InvalidInput(InputErrorType::MissingCharacter),
                    ))
                }
            }
        } else {
            Err(Error::new(
                "Error getting character: No character found for this discord account",
                ErrorType::InvalidInput(InputErrorType::MissingCharacter),
            ))
        }
    }

    pub async fn get_character(&self, id: CharacterId) -> Result<Character, Error> {
        let path = get_character_path(id).await?;
        Ok(Character::from_file(&path).await?)
    }

    pub fn get_character_name<'a>(
        &'a self,
        user_id: u64,
        character_id: CharacterId,
    ) -> Result<&'a str, Error> {
        match self.characters.characters.get(&user_id) {
            None => Err(Error::new(
                "Error getting character name: No character found for this account",
                ErrorType::InvalidInput(InputErrorType::MissingCharacter),
            )),
            Some(user_characters) => {
                match user_characters
                    .iter()
                    .filter(|c| c.character_id == character_id)
                    .next()
                {
                    None => Err(Error::new(
                        "Error getting character name: No character found with the given id",
                        ErrorType::InvalidInput(InputErrorType::MissingCharacter),
                    )),
                    Some(c) => Ok(&c.name),
                }
            }
        }
    }

    async fn write_character_list(&self) -> Result<(), Error> {
        let mut path = config::get_config_dir()?;
        path.push("discord_character_list");
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .await?;
        let data = serde_json::to_string(&self.characters)?;
        file.write(data.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }
}

async fn get_character_path(character_id: CharacterId) -> Result<PathBuf, Error> {
    let mut path = config::get_config_dir()?;
    path.push("discord_characters");
    fs::create_dir_all(&path).await?;
    path.push(&character_id.0.to_string());
    Ok(path)
}
