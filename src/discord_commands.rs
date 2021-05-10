use super::character::Character;
use super::config::*;
use super::discord::*;
use super::dsa;
use super::util::*;
use async_std::fs;
use async_std::io;
use async_std::prelude::*;
use clap::ArgMatches;
use futures::stream::StreamExt;
use std::iter::Iterator;
use std::path::PathBuf;

use serenity::{
    async_trait,
    model::{
        channel::{ChannelType, Message},
        guild::Member,
        id::UserId,
        permissions::Permissions,
    },
    prelude::*,
};

fn try_get_character(user_id: &UserId) -> Result<Character, Error> {
    let mut char_path = get_config_dir()?;
    char_path.push("discord_characters");
    char_path.push(user_id.to_string());
    if !std::path::Path::exists(&char_path) {
        return Err(Error::new(
            "Error loading character: No character found for your discord account",
            ErrorType::InvalidInput(InputErrorType::MissingCharacter),
        ));
    }
    Character::from_file(&char_path)
}

pub struct CommandUpload;
pub struct CommandCheck;
pub struct CommandAttack;
pub struct CommandSpell;
pub struct CommandDodge;
pub struct CommandParry;
pub struct CommandRoll;
pub struct CommandIni;

impl CommandUpload {
    async fn upload_character(
        &self,
        message: &Message,
        config: &Config,
    ) -> Result<Character, Error> {
        //Attachement validation
        if message.attachments.len() != 1 {
            return Err(Error::new(
                format!(
                    "Invalid number of attachements: {}",
                    message.attachments.len()
                ),
                ErrorType::InvalidInput(InputErrorType::InvalidAttachements),
            ));
        } else if message.attachments[0].size
            > config.discord.max_attachement_size.unwrap_or(1000000)
        {
            return Err(Error::new(
                format!(
                    "Attachement too big ({} bytes)",
                    message.attachments[0].size
                ),
                ErrorType::InvalidInput(InputErrorType::InvalidAttachements),
            ));
        }
        //Get character path
        let mut char_path = get_config_dir()?;
        char_path.push("discord_characters");
        fs::create_dir_all(&char_path).await?;
        char_path.push(message.author.id.to_string());
        //Download data
        let data = message.attachments[0].download().await?;
        //Open file
        let file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&char_path)
            .await?;
        //Write
        let mut writer = io::BufWriter::new(file);
        writer.write(&data).await?;
        writer.flush().await?;
        match Character::from_file(&char_path) {
            Ok(c) => {
                if c.get_name().len() > config.discord.max_name_length.unwrap_or(32) {
                    fs::remove_file(&char_path).await?;
                    Err(Error::new(
                        format!(
                            "Character name exceeds {} characters",
                            config.discord.max_name_length.unwrap_or(32)
                        ),
                        ErrorType::InvalidInput(InputErrorType::CharacterNameTooLong),
                    ))
                } else {
                    Ok(c)
                }
            }
            Err(e) => match e.err_type() {
                ErrorType::InvalidInput(_) => {
                    fs::remove_file(&char_path).await?;
                    Err(e)
                }
                _ => Err(e),
            },
        }
    }
}
#[async_trait]
impl DiscordCommand for CommandUpload {
    fn name(&self) -> &'static str {
        "upload"
    }

    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        _: &ArgMatches,
    ) {
        match self.upload_character(message, &handler.config).await {
            Ok(character) => {
                output.output_line(&format!(
                    "Successfully uploaded character \"{}\"",
                    character.get_name()
                ));
            }
            Err(e) => match e.err_type() {
                ErrorType::InvalidInput(_) => {
                    output.output_line(&format!("Error loading character: {}", e));
                }
                _ => {
                    output.output_line(&"Internal server error while loading character");
                    println!("Error loading character: {:?}", e);
                }
            },
        }
    }
}

#[async_trait]
impl DiscordCommand for CommandCheck {
    fn name(&self) -> &'static str {
        "check"
    }
    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id) {
            Ok(character) => {
                dsa::talent_check(
                    sub_m,
                    &character,
                    &handler.dsa_data,
                    &handler.config,
                    output,
                );
            }
            Err(e) => match e.err_type() {
                ErrorType::InvalidInput(_) => {
                    output.output_line(&e);
                }
                _ => {
                    output.output_line(&"Internal server error while rolling check");
                    println!("Error rolling check: {:?}", e);
                }
            },
        };
    }
}

#[async_trait]
impl DiscordCommand for CommandAttack {
    fn name(&self) -> &'static str {
        "attack"
    }
    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id) {
            Ok(character) => {
                dsa::attack_check(sub_m, &character, &handler.dsa_data, output);
            }
            Err(e) => match e.err_type() {
                ErrorType::InvalidInput(_) => {
                    output.output_line(&e);
                }
                _ => {
                    output.output_line(&"Internal server error while rolling attack");
                    println!("Error rolling attack: {:?}", e);
                }
            },
        };
    }
}

#[async_trait]
impl DiscordCommand for CommandSpell {
    fn name(&self) -> &'static str {
        "spell"
    }
    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id) {
            Ok(character) => {
                dsa::spell_check(
                    sub_m,
                    &character,
                    &handler.dsa_data,
                    &handler.config,
                    output,
                );
            }
            Err(e) => match e.err_type() {
                ErrorType::InvalidInput(_) => {
                    output.output_line(&e);
                }
                _ => {
                    output.output_line(&"Internal server error while rolling spell");
                    println!("Error rolling spell: {:?}", e);
                }
            },
        };
    }
}

#[async_trait]
impl DiscordCommand for CommandDodge {
    fn name(&self) -> &'static str {
        "dodge"
    }
    async fn execute(
        &self,
        message: &Message,
        _: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id) {
            Ok(character) => {
                dsa::dodge_check(sub_m, &character, output);
            }
            Err(e) => match e.err_type() {
                ErrorType::InvalidInput(_) => {
                    output.output_line(&e);
                }
                _ => {
                    output.output_line(&"Internal server error while rolling dodge");
                    println!("Error rolling dodge: {:?}", e);
                }
            },
        };
    }
}

#[async_trait]
impl DiscordCommand for CommandParry {
    fn name(&self) -> &'static str {
        "parry"
    }
    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id) {
            Ok(character) => {
                dsa::parry_check(sub_m, &character, &handler.dsa_data, output);
            }
            Err(e) => match e.err_type() {
                ErrorType::InvalidInput(_) => {
                    output.output_line(&e);
                }
                _ => {
                    output.output_line(&"Internal server error while rolling parry");
                    println!("Error rolling parry: {:?}", e);
                }
            },
        };
    }
}

#[async_trait]
impl DiscordCommand for CommandRoll {
    fn name(&self) -> &'static str {
        "roll"
    }
    async fn execute(
        &self,
        _: &Message,
        _: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        dsa::roll(sub_m, output);
    }
}

impl CommandIni {
    async fn fetch_discord_members(
        &self,
        ctx: &Context,
        message: &Message,
    ) -> Result<Vec<Member>, Error> {
        let only_on_server_err = Err(Error::new(
            String::from("This option can only be used in a server"),
            ErrorType::InvalidInput(InputErrorType::InvalidDiscordContext),
        ));
        let invalid_channel_err = Err(Error::new(
            String::from("Invalid channel targeted"),
            ErrorType::InvalidInput(InputErrorType::InvalidDiscordContext),
        ));

        let channel = match message.channel(&ctx.cache).await {
            Some(c) => c,
            None => {
                return only_on_server_err;
            }
        };

        let channel = match channel.guild() {
            Some(gc) => gc,
            None => {
                return invalid_channel_err;
            }
        };

        match channel.kind {
            ChannelType::Text | ChannelType::Private => {}
            _ => {
                return invalid_channel_err;
            }
        }

        let guild = message.guild(&ctx).await.unwrap();
        let g_members = guild.members(&ctx, Some(1000), None).await?;

        let get_channel_perms = |member: &Member| guild.user_permissions_in(&channel, member); // life time hax

        Ok(
            futures::stream::iter(g_members.iter().map(|m| m.clone())) // fetch members in the channel message was sent in
                .filter_map(|member| async move {
                    if get_channel_perms(&member)
                        .map(|p| p.contains(Permissions::READ_MESSAGES))
                        .unwrap_or(false)
                    {
                        Some(member)
                    } else {
                        None
                    }
                })
                .collect::<Vec<Member>>()
                .await,
        )
    }

    async fn initiative(
        &self,
        sub_m: &clap::ArgMatches,
        message: &Message,
        ctx: &Context,
        output: &mut impl OutputWrapper,
    ) -> Result<(), Error> {
        let config_path = get_config_dir()?;

        //Reset trumps all other arguments
        if sub_m.is_present("reset") {
            let members = match self.fetch_discord_members(ctx, message).await {
                Ok(m) => m,
                Err(e) => {
                    return Err(e);
                }
            };

            let mut rename_futs = Vec::new();
            for member in members {
                let user_id = member.user.id.to_string();
                let mut path = PathBuf::from(&config_path);
                path.push("discord_characters");
                path.push(user_id);
                /*
                Reset the nickname if all of the following apply
                1. The user has uploaded a character
                2. The user has a discord nickname
                3. The discord nickname is of the form "[i64](,[i64]...,[i64]) orig_name"
                */
                if std::path::Path::exists(&path) {
                    if let Some(nickname) = member.nick.clone() {
                        if let Some(index) = nickname.find(' ') {
                            if !nickname[..index]
                                .split(',')
                                .all(|ini_part| ini_part.parse::<i64>().is_ok())
                            {
                                continue;
                            }
                            let new_name = nickname[index + 1..].to_string();
                            rename_futs.push(async {
                                let member = member;
                                match member.edit(&ctx.http, |edit| edit.nickname(new_name)).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        println!("Error changing user nickname: {}", e);
                                    }
                                };
                            });
                        }
                    }
                }
            }
            futures::future::join_all(rename_futs).await;
            output.output_line(&"Reset nicknames");
            return Ok(());
        }

        //All (name, ini_level) tuples to include in the check
        let mut characters: Vec<(String, i64)> = Vec::new();
        //The user_id for all the characters that have one (currently only used for renaming)
        let mut characters_members: Vec<Option<Member>> = Vec::new();

        if sub_m.is_present("all") {
            let members = match self.fetch_discord_members(ctx, message).await {
                Ok(m) => m,
                Err(e) => {
                    return Err(e);
                }
            };

            for member in members {
                let user_id = member.user.id.to_string();
                let mut path = PathBuf::from(&config_path);
                path.push("discord_characters");
                path.push(user_id);
                if std::path::Path::exists(&path) {
                    match Character::from_file(&path) {
                        Err(_) => {
                            return Err(Error::new(
                                format!(
                                    "Unable to retrieve character for {}",
                                    member.display_name()
                                ),
                                ErrorType::InvalidInput(InputErrorType::InvalidFormat),
                            ));
                        }
                        Ok(character) => {
                            characters.push((
                                character.get_name().to_string(),
                                character.get_initiative_level(),
                            ));
                            characters_members.push(Some(member.clone()));
                        }
                    }
                }
            }
        } else {
            //Add the authors character to the list
            let mut path = PathBuf::from(&config_path);
            path.push("discord_characters");
            path.push(message.author.id.to_string());
            if std::path::Path::exists(&path) {
                let character = Character::from_file(&path)?;
                characters.push((
                    character.get_name().to_string(),
                    character.get_initiative_level(),
                ));
                characters_members.push(None);
            } else {
                return Err(Error::new(
                    "No character found for your discord account",
                    ErrorType::InvalidInput(InputErrorType::MissingCharacter),
                ));
            }
        }

        if sub_m.is_present("new") {
            let custom_args: Vec<&str> = sub_m.values_of("new").unwrap().collect();
            if custom_args.len() % 2 != 0 {
                return Err(Error::new(
                "The \"new\" argument expects an even number of values (name and level for each custom character)",
                ErrorType::InvalidInput(InputErrorType::InvalidArgument)
            ));
            }
            for custom_char in custom_args.chunks(2) {
                let ini_level = match custom_char[1].parse::<i64>() {
                    Err(_) => {
                        return Err(Error::new(
                            format!(
                                "Unable to parse custom initiative level: {}",
                                custom_char[1]
                            ),
                            ErrorType::InvalidInput(InputErrorType::InvalidArgument),
                        ));
                    }
                    Ok(level) => level,
                };
                characters.push((custom_char[0].to_string(), ini_level));
                characters_members.push(None);
            }
        }

        let rolls = dsa::roll_ini(&characters, output);

        if rolls.is_empty() {
            return Err(Error::new(
                String::from("No player in this channel has uploaded a character"),
                ErrorType::InvalidInput(InputErrorType::InvalidDiscordContext),
            ));
        }

        if sub_m.is_present("rename") {
            let mut rename_futs = Vec::new();
            for roll in rolls {
                let character = &characters[roll.0];
                let member = &characters_members[roll.0];
                if let Some(member) = member {
                    let displ_name = member.display_name();
                    let mut new_name = roll.1.iter().skip(1).fold(
                        format!("{}", character.1 + roll.1[0]),
                        |mut s, roll| {
                            s.push_str(&format!(",{}", roll));
                            s
                        },
                    );
                    new_name.push(' ');
                    new_name.push_str(&displ_name);

                    rename_futs.push(async {
                        //Shadow the variable to force a move
                        let roll = roll;
                        match characters_members[roll.0]
                            .as_ref()
                            .unwrap()
                            .edit(&ctx.http, |edit| edit.nickname(new_name))
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error changing user nickname: {:?}", e);
                            }
                        }
                    });
                }
            }

            futures::future::join_all(rename_futs).await;
        }
        Ok(())
    }
}

#[async_trait]
impl DiscordCommand for CommandIni {
    fn name(&self) -> &'static str {
        "ini"
    }
    async fn execute(
        &self,
        message: &Message,
        _: &Handler,
        output: &mut DiscordOutputWrapper,
        context: &Context,
        sub_m: &ArgMatches,
    ) {
        match &self.initiative(&sub_m, &message, &context, output).await {
            Ok(()) => {}
            Err(e) => match e.err_type() {
                ErrorType::InvalidInput(_) => {
                    output.output_line(&e);
                }
                _ => {
                    output.output_line(&"Internal server error while rolling initiative");
                    println!("Error rolling initiative: {:?}", e);
                }
            },
        };
    }
}
