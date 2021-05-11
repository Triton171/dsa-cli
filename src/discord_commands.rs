use crate::{cli, config};

use super::character::Character;
use super::config::*;
use super::discord::*;
use super::dsa;
use super::util::*;
use clap::ArgMatches;
use futures::stream::StreamExt;
use serde_json::{Number, Value};
use std::iter::Iterator;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::{fs, io};

use serenity::{
    async_trait,
    builder::CreateApplicationCommandOption,
    model::{
        channel::{ChannelType, Message},
        guild::Member,
        id::{ChannelId, UserId},
        interactions::{ApplicationCommandOptionType, Interaction},
        permissions::Permissions,
    },
    prelude::*,
};

async fn try_get_character(user_id: &UserId) -> Result<Character, Error> {
    let mut char_path = config::get_config_dir()?;
    char_path.push("discord_characters");
    char_path.push(user_id.to_string());
    if !std::path::Path::exists(&char_path) {
        return Err(Error::new(
            "Error loading character: No character found for your discord account",
            ErrorType::InvalidInput(InputErrorType::MissingCharacter),
        ));
    }
    Character::from_file(&char_path).await
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
        } else if message.attachments[0].size > config.discord.max_attachement_size {
            return Err(Error::new(
                format!(
                    "Attachement too big ({} bytes)",
                    message.attachments[0].size
                ),
                ErrorType::InvalidInput(InputErrorType::InvalidAttachements),
            ));
        }
        //Get character path
        let mut char_path = config::get_config_dir()?;
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
        match Character::from_file(&char_path).await {
            Ok(c) => {
                if c.get_name().len() > config.discord.max_name_length {
                    fs::remove_file(&char_path).await?;
                    Err(Error::new(
                        format!(
                            "Character name exceeds {} characters",
                            config.discord.max_name_length
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
    fn description(&self) -> &'static str {
        "Uploads a character for your discord account. The .json file has to be attached to this message"
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
    fn description(&self) -> &'static str {
        "Performs a skillcheck for the given talent"
    }
    fn create_interaction_options(
        &self,
        _: &Handler,
    ) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        let mut talent_check = &mut CreateApplicationCommandOption::default();
        talent_check = talent_check
            .required(true)
            .description("The talent to check for ")
            .name("talent")
            .kind(ApplicationCommandOptionType::String);

        let mut num = &mut CreateApplicationCommandOption::default();
        num = num
            .description("Facilitation for the talent check")
            .name("facilitation")
            .default_option(false)
            .kind(ApplicationCommandOptionType::Integer);

        vec![talent_check.clone(), num.clone()]
    }

    async fn handle_slash_command<'a>(
        &self,
        interaction: &'a Interaction,
        handler: &'a Handler,
        _: &'a Context,
    ) -> String {
        let output = &mut DiscordOutputWrapper::new();
        let user = interaction.member.clone().unwrap().user;

        if interaction.data.is_none() {
            output.output_line(&"Invalid argument!");
            return output.get_content();
        }

        let data = interaction.data.clone().unwrap().options;

        if !data.iter().any(|cmd| cmd.name == "talent") {
            output.output_line(&"Invalid talent!");
            return output.get_content();
        }

        let name = data
            .iter()
            .filter(|cmd| cmd.name == "talent" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap();
        let facility = data
            .iter()
            .filter(|cmd| cmd.name == "facility" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap_or(Value::String(String::from("0")));

        let sub_m = cli::get_app().get_matches_from(vec![
            "",
            "check",
            name.as_str().unwrap(),
            facility.as_str().unwrap(),
        ]);
        let sub_m = sub_m.subcommand().unwrap().1;

        match try_get_character(&user.id).await {
            Ok(character) => {
                dsa::talent_check(
                    &sub_m,
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

        output.get_content()
    }

    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id).await {
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
    fn description(&self) -> &'static str {
        "Performs an attack skillcheck for the given combat technique"
    }
    fn create_interaction_options(
        &self,
        handler: &Handler,
    ) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        let mut talent_check = &mut CreateApplicationCommandOption::default();
        talent_check = talent_check
            .required(true)
            .description("The combat technique to check for ")
            .name("technique")
            .kind(ApplicationCommandOptionType::String);

        for combat in handler
            .dsa_data
            .combat_techniques
            .keys()
            .into_iter()
            .take(25)
        {
            talent_check = talent_check.add_string_choice(combat, combat);
        }

        let mut num = &mut CreateApplicationCommandOption::default();
        num = num
            .description("Facilitation for the combat check")
            .name("facilitation")
            .default_option(false)
            .kind(ApplicationCommandOptionType::Integer);

        vec![talent_check.clone(), num.clone()]
    }

    async fn handle_slash_command<'a>(
        &self,
        interaction: &'a Interaction,
        handler: &'a Handler,
        _: &'a Context,
    ) -> String {
        let output = &mut DiscordOutputWrapper::new();
        let user = interaction.member.clone().unwrap().user;

        if interaction.data.is_none() {
            output.output_line(&"Invalid argument!");
            return output.get_content();
        }

        let data = interaction.data.clone().unwrap().options;

        if !data.iter().any(|cmd| cmd.name == "technique") {
            output.output_line(&"Invalid talent!");
            return output.get_content();
        }

        let name = data
            .iter()
            .filter(|cmd| cmd.name == "technique" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap();
        let facility = data
            .iter()
            .filter(|cmd| cmd.name == "facilitation" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap_or(Value::Number(Number::from(0)));

        let sub_m = cli::get_app().get_matches_from(vec![
            "",
            "attack",
            name.as_str().unwrap(),
            &facility.to_string(),
        ]);
        let sub_m = sub_m.subcommand().unwrap().1;

        match try_get_character(&user.id).await {
            Ok(character) => {
                dsa::attack_check(&sub_m, &character, &handler.dsa_data, output);
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

        output.get_content()
    }

    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id).await {
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
    fn description(&self) -> &'static str {
        "Performs a spell skillcheck for the given spell"
    }
    fn create_interaction_options(
        &self,
        handler: &Handler,
    ) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        let mut talent_check = &mut CreateApplicationCommandOption::default();
        talent_check = talent_check
            .required(true)
            .description("The spell to check for ")
            .name("spell")
            .kind(ApplicationCommandOptionType::String);

        for spells in handler.dsa_data.spells.keys().into_iter().take(25) {
            talent_check = talent_check.add_string_choice(spells, spells);
        }

        let mut num = &mut CreateApplicationCommandOption::default();
        num = num
            .description("Facilitation for the spell check")
            .name("facilitation")
            .default_option(false)
            .kind(ApplicationCommandOptionType::Integer);

        vec![talent_check.clone(), num.clone()]
    }

    async fn handle_slash_command<'a>(
        &self,
        interaction: &'a Interaction,
        handler: &'a Handler,
        _: &'a Context,
    ) -> String {
        let output = &mut DiscordOutputWrapper::new();
        let user = interaction.member.clone().unwrap().user;

        if interaction.data.is_none() {
            output.output_line(&"Invalid argument!");
            return output.get_content();
        }

        let data = interaction.data.clone().unwrap().options;

        if !data.iter().any(|cmd| cmd.name == "spell") {
            output.output_line(&"Invalid spell!");
            return output.get_content();
        }

        let name = data
            .iter()
            .filter(|cmd| cmd.name == "spell" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap();
        let facility = data
            .iter()
            .filter(|cmd| cmd.name == "facilitation" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap_or(Value::Number(Number::from(0)));

        let sub_m = cli::get_app().get_matches_from(vec![
            "",
            "spell",
            name.as_str().unwrap(),
            &facility.to_string(),
        ]);
        let sub_m = sub_m.subcommand().unwrap().1;

        match try_get_character(&user.id).await {
            Ok(character) => {
                dsa::spell_check(
                    &sub_m,
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

        output.get_content()
    }

    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id).await {
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
    fn description(&self) -> &'static str {
        "Performs a dodge skillcheck"
    }

    fn create_interaction_options(
        &self,
        _: &Handler,
    ) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        let mut num = &mut CreateApplicationCommandOption::default();
        num = num
            .description("Facilitation for the combat check")
            .name("facilitation")
            .default_option(false)
            .kind(ApplicationCommandOptionType::Integer);

        vec![num.clone()]
    }

    async fn handle_slash_command<'a>(
        &self,
        interaction: &'a Interaction,
        _: &'a Handler,
        _: &'a Context,
    ) -> String {
        let output = &mut DiscordOutputWrapper::new();
        let user = interaction.member.clone().unwrap().user;

        if interaction.data.is_none() {
            output.output_line(&"Invalid argument!");
            return output.get_content();
        }

        let data = interaction.data.clone().unwrap().options;
        let facility = data
            .iter()
            .filter(|cmd| cmd.name == "facilitation" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap_or(Value::Number(Number::from(0)));

        let sub_m = cli::get_app().get_matches_from(vec!["", "dodge", &facility.to_string()]);
        let sub_m = sub_m.subcommand().unwrap().1;

        match try_get_character(&user.id).await {
            Ok(character) => {
                dsa::dodge_check(&sub_m, &character, output);
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

        output.get_content()
    }

    async fn execute(
        &self,
        message: &Message,
        _: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id).await {
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
    fn description(&self) -> &'static str {
        "Performs a parry skillcheck for the given combat technique"
    }

    fn create_interaction_options(
        &self,
        handler: &Handler,
    ) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        let mut talent_check = &mut CreateApplicationCommandOption::default();
        talent_check = talent_check
            .required(true)
            .description("The combat technique to check for ")
            .name("technique")
            .kind(ApplicationCommandOptionType::String);

        for combat in handler
            .dsa_data
            .combat_techniques
            .keys()
            .into_iter()
            .take(25)
        {
            talent_check = talent_check.add_string_choice(combat, combat);
        }

        let mut num = &mut CreateApplicationCommandOption::default();
        num = num
            .description("Facilitation for the combat check")
            .name("facilitation")
            .default_option(false)
            .kind(ApplicationCommandOptionType::Integer);

        vec![talent_check.clone(), num.clone()]
    }

    async fn handle_slash_command<'a>(
        &self,
        interaction: &'a Interaction,
        handler: &'a Handler,
        _: &'a Context,
    ) -> String {
        let output = &mut DiscordOutputWrapper::new();
        let user = interaction.member.clone().unwrap().user;

        if interaction.data.is_none() {
            output.output_line(&"Invalid argument!");
            return output.get_content();
        }

        let data = interaction.data.clone().unwrap().options;

        if !data.iter().any(|cmd| cmd.name == "technique") {
            output.output_line(&"Invalid talent!");
            return output.get_content();
        }

        let name = data
            .iter()
            .filter(|cmd| cmd.name == "technique" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap();
        let facility = data
            .iter()
            .filter(|cmd| cmd.name == "facilitation" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap_or(Value::Number(Number::from(0)));

        let sub_m = cli::get_app().get_matches_from(vec![
            "",
            "parry",
            name.as_str().unwrap(),
            &facility.to_string(),
        ]);
        let sub_m = sub_m.subcommand().unwrap().1;

        match try_get_character(&user.id).await {
            Ok(character) => {
                dsa::parry_check(&sub_m, &character, &handler.dsa_data, output);
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

        output.get_content()
    }

    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        _: &Context,
        sub_m: &ArgMatches,
    ) {
        match try_get_character(&message.author.id).await {
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
    fn description(&self) -> &'static str {
        "Rolls some dice"
    }

    fn create_interaction_options(
        &self,
        _: &Handler,
    ) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        let mut cmd = &mut CreateApplicationCommandOption::default();
        cmd = cmd
            .name("expression")
            .description("[number_of_dice]d[dice_type] + [offset]")
            .kind(ApplicationCommandOptionType::String)
            .required(true);
        vec![cmd.clone()]
    }

    async fn handle_slash_command<'a>(
        &self,
        interaction: &'a Interaction,
        _: &'a Handler,
        _: &'a Context,
    ) -> String {
        let output = &mut DiscordOutputWrapper::new();

        if interaction.data.is_none() {
            output.output_line(&"Invalid argument!");
            return output.get_content();
        }

        let data = interaction.data.clone().unwrap().options;

        if !data.iter().any(|cmd| cmd.name == "expression") {
            output.output_line(&"Invalid dice expression!");
            return output.get_content();
        }

        let expression = data
            .iter()
            .filter(|cmd| cmd.name == "expression" && cmd.value.is_some())
            .map(|cmd| cmd.value.clone().unwrap())
            .next()
            .unwrap();

        let sub_m = cli::get_app().get_matches_from(vec!["", "roll", expression.as_str().unwrap()]);
        let sub_m = sub_m.subcommand().unwrap().1;

        dsa::roll(sub_m, output);

        output.get_content()
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
        channel_id: &ChannelId,
    ) -> Result<Vec<Member>, Error> {
        let invalid_channel_err = Err(Error::new(
            String::from("Invalid channel targeted"),
            ErrorType::InvalidInput(InputErrorType::InvalidDiscordContext),
        ));

        let channel = channel_id.to_channel(&ctx.http).await?;

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

        let guild = channel.guild(&ctx).await.unwrap();
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
        channel_id: &ChannelId,
        user_id: &UserId,
        ctx: &Context,
        output: &mut impl OutputWrapper,
    ) -> Result<(), Error> {
        let config_path = get_config_dir()?;

        //Reset trumps all other arguments
        if sub_m.is_present("reset") {
            let members = match self.fetch_discord_members(ctx, channel_id).await {
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
            let members = match self.fetch_discord_members(ctx, channel_id).await {
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
                    match Character::from_file(&path).await {
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
            path.push(user_id.to_string());
            if std::path::Path::exists(&path) {
                let character = Character::from_file(&path).await?;
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
    fn description(&self) -> &'static str {
        "Performs an initiative roll for the current character"
    }

    fn create_interaction_options(
        &self,
        _: &Handler,
    ) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        let mut reset = &mut CreateApplicationCommandOption::default();
        reset = reset
            .name("reset")
            .description("Reset all nicknames that have been changed by the 'rename' option")
            .kind(ApplicationCommandOptionType::SubCommand);

        let mut rename = &mut CreateApplicationCommandOption::default();
        rename = rename
            .name("rename")
            .description("Change the discord nickname of each user to prefix with the initiative")
            .kind(ApplicationCommandOptionType::SubCommand);

        let mut all = &mut CreateApplicationCommandOption::default();
        all = all
            .name("all")
            .description(
                "Calculate the initiative for all characters of users in the current channel",
            )
            .kind(ApplicationCommandOptionType::SubCommand);

        let mut me = &mut CreateApplicationCommandOption::default();
        me = me
            .name("me")
            .description("Calculate the initiative for your character")
            .kind(ApplicationCommandOptionType::SubCommand);

        vec![me.clone(), all.clone(), reset.clone(), rename.clone()]
    }

    async fn handle_slash_command<'a>(
        &self,
        inter: &'a Interaction,
        _: &'a Handler,
        context: &'a Context,
    ) -> String {
        let output = &mut DiscordOutputWrapper::new();
        let sub_cmd = inter.data.clone().unwrap().options[0].name.clone();
        let channel = inter.channel_id.clone().unwrap();
        let executer_id;

        if inter.member.is_some() {
            executer_id = inter.member.clone().unwrap().user.id;
        } else {
            executer_id = inter.user.clone().unwrap().id;
        }

        let mut cmd = vec![];
        cmd.push("dsa-cli");
        cmd.push("ini");
        match sub_cmd.as_str() {
            "me" => {}
            "all" => {
                cmd.push("--all");
            }
            "reset" => {
                cmd.push("--reset");
            }
            "rename" => {
                cmd.push("--all");
                cmd.push("--rename")
            }
            _ => {}
        }
        let sub_m = cli::get_discord_app().get_matches_from(cmd);
        let sub_m = sub_m.subcommand().unwrap().1;

        match &self
            .initiative(&sub_m, &channel, &executer_id, context, output)
            .await
        {
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
        output.get_content()
    }

    async fn execute(
        &self,
        message: &Message,
        _: &Handler,
        output: &mut DiscordOutputWrapper,
        context: &Context,
        sub_m: &ArgMatches,
    ) {
        match &self
            .initiative(
                &sub_m,
                &message.channel_id,
                &message.author.id,
                &context,
                output,
            )
            .await
        {
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
