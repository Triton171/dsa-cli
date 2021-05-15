use crate::config;

use super::character::Character;
use super::config::*;
use super::dsa;
use super::util::*;
use clap::{App, Arg, ArgMatches, ArgSettings};
use futures::stream::StreamExt;
use serde_json::Value;
use std::iter::Iterator;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::{fs, io};

use serenity::{
    async_trait,
    builder::{CreateApplicationCommand, CreateApplicationCommandOption},
    model::{
        channel::{Attachment, ChannelType, Message},
        guild::Member,
        id::{ChannelId, /*GuildId, */ UserId},
        interactions::ApplicationCommand,
        interactions::{ApplicationCommandOptionType, Interaction},
        permissions::Permissions,
    },
    prelude::*,
};

#[async_trait]
pub trait CommandContext {
    fn context<'a>(&'a self) -> &'a Context;
    fn sender(&self) -> Result<UserId, Error>;
    fn channel(&self) -> Result<ChannelId, Error>;
    async fn attachments<'a>(&'a self) -> Result<&'a [Attachment], Error>;
    async fn members_in_channel(&self) -> Result<Vec<Member>, Error> {
        let invalid_channel_err = Err(Error::new(
            String::from("Invalid channel targeted"),
            ErrorType::InvalidInput(InputErrorType::InvalidDiscordContext),
        ));

        let channel_id = self.channel()?;
        let channel = channel_id.to_channel(self.context()).await?;

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

        let guild = channel.guild(self.context()).await.unwrap();
        let g_members = guild.members(self.context(), Some(1000), None).await?;

        Ok(
            futures::stream::iter(g_members.iter().map(|m| m.clone())) // fetch members in the channel message was sent in
                .filter_map(|member| async {
                    if guild
                        .user_permissions_in(&channel, &member)
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

    async fn rename_member(&self, member: &Member, new_name: &str) -> Result<(), Error> {
        member
            .edit(&self.context().http, |edit| edit.nickname(new_name))
            .await?;
        Ok(())
    }
}

//The context for a command that was sent in a guild channel or as a direct message to the bot
pub struct MessageContext<'b> {
    ctx: &'b Context,
    message: &'b Message,
}

impl MessageContext<'_> {
    pub fn new<'a>(ctx: &'a Context, message: &'a Message) -> MessageContext<'a> {
        MessageContext { ctx, message }
    }
}

#[async_trait]
impl CommandContext for MessageContext<'_> {
    fn context<'a>(&'a self) -> &'a Context {
        self.ctx
    }
    fn sender(&self) -> Result<UserId, Error> {
        Ok(self.message.author.id)
    }
    fn channel(&self) -> Result<ChannelId, Error> {
        Ok(self.message.channel_id)
    }
    async fn attachments<'a>(&'a self) -> Result<&'a [Attachment], Error> {
        Ok(&self.message.attachments)
    }
}

pub struct SlashCommandContext<'b> {
    ctx: &'b Context,
    interaction: &'b Interaction,
}

impl SlashCommandContext<'_> {
    pub fn new<'a>(ctx: &'a Context, interaction: &'a Interaction) -> SlashCommandContext<'a> {
        SlashCommandContext { ctx, interaction }
    }
}

#[async_trait]
impl CommandContext for SlashCommandContext<'_> {
    fn context<'a>(&'a self) -> &'a Context {
        self.ctx
    }
    fn sender(&self) -> Result<UserId, Error> {
        if let Some(member) = &self.interaction.member {
            Ok(member.user.id)
        } else if let Some(user) = &self.interaction.user {
            Ok(user.id)
        } else {
            Err(Error::new(
                "Unable to find sender of slash command",
                ErrorType::IO(IOErrorType::Discord),
            ))
        }
    }
    fn channel(&self) -> Result<ChannelId, Error> {
        self.interaction.channel_id.map_or(
            Err(Error::new(
                "Error retrieving channel for slash command",
                ErrorType::IO(IOErrorType::Discord),
            )),
            |c| Ok(c),
        )
    }
    async fn attachments<'a>(&'a self) -> Result<&'a [Attachment], Error> {
        Err(Error::new(
            "Downloading attachments is not yet supported for slash commands",
            ErrorType::InvalidInput(InputErrorType::InvalidArgument),
        ))
    }
}

/*
Translates the subcommands and arguments of the given app to discord slash commands and registers them
*/
pub async fn register_slash_commands(app: App<'_>, ctx: &Context) -> Result<(), Error> {
    /*let test_server = GuildId(830394313783246858);
    test_server
        .create_application_commands(ctx, |create_cmds| {*/
    ApplicationCommand::create_global_application_commands(ctx, |create_cmds| {
        for sub_app in app.get_subcommands() {
            let mut slash_cmd = CreateApplicationCommand::default();
            let mut slash_cmd_options: Vec<CreateApplicationCommandOption> = Vec::new();

            //Add all the required arguments
            for arg in sub_app
                .get_arguments()
                .filter(|arg| arg.is_set(ArgSettings::Required))
            {
                let mut option = clap_to_discord_arg(arg);
                option.required(true);
                slash_cmd_options.push(option);
            }
            //Add all the non-required arguments
            for arg in sub_app
                .get_arguments()
                .filter(|arg| !arg.is_set(ArgSettings::Required))
            {
                let mut option = clap_to_discord_arg(arg);
                option.required(false);
                slash_cmd_options.push(option);
            }
            if !slash_cmd_options.is_empty() {
                slash_cmd.set_options(slash_cmd_options);
            }
            slash_cmd
                .name(sub_app.get_name())
                .description(sub_app.get_about().unwrap_or("Missing description"));
            create_cmds.add_application_command(slash_cmd);
        }
        create_cmds
    })
    .await?;
    Ok(())
}

fn clap_to_discord_arg(arg: &Arg) -> CreateApplicationCommandOption {
    let mut slash_cmd_option = CreateApplicationCommandOption::default();
    slash_cmd_option
        .name(arg.get_name())
        .description(arg.get_about().unwrap_or(""));
    if arg.is_set(ArgSettings::TakesValue) {
        slash_cmd_option.kind(ApplicationCommandOptionType::String);
    } else {
        slash_cmd_option.kind(ApplicationCommandOptionType::Boolean);
    }
    slash_cmd_option
}

/*
Translates the given discord slash command to a clap::ArgMatches object.
This should generally only be used with discord commands created with the 'register_slash_commands' function
*/
pub async fn parse_discord_interaction(
    interaction: &Interaction,
    app: App<'_>,
) -> Result<clap::Result<ArgMatches>, Error> {
    match interaction.kind {
        serenity::model::interactions::InteractionType::ApplicationCommand => {}
        _ => {
            return Err(Error::new(
                "Unknown interaction kind",
                ErrorType::IO(IOErrorType::UnknownInteractionType),
            ));
        }
    };
    let data = interaction.data.as_ref().unwrap();
    let sub_cmd = match app.find_subcommand(&data.name) {
        Some(cmd) => cmd,
        None => {
            return Err(Error::new(
                "Did not find clap command for discord command",
                ErrorType::Unknown,
            ));
        }
    };

    //Construct a string that can be passed to the clap argument parsing
    let mut constructed_str = String::from("dsa-cli ") + sub_cmd.get_name();
    for arg in sub_cmd.get_arguments() {
        for option in &data.options {
            if arg.get_name() == option.name {
                let arg_val = if arg.is_set(ArgSettings::TakesValue) {
                    let val = match &option.value {
                        Some(v) => v,
                        None => {
                            return Err(Error::new(
                                "Missing value in discord argument",
                                ErrorType::Unknown,
                            ));
                        }
                    };
                    match val {
                        Value::String(s) => s,
                        _ => {
                            return Err(Error::new(
                                "Illegal value type in discord argument",
                                ErrorType::Unknown,
                            ));
                        }
                    }
                } else {
                    ""
                };
                if let Some(long) = arg.get_long() {
                    constructed_str.push_str(" --");
                    constructed_str.push_str(long);
                    constructed_str.push(' ');
                    constructed_str.push_str(arg_val);
                } else if let Some(short) = arg.get_short() {
                    constructed_str.push_str(" -");
                    constructed_str.push(short);
                    constructed_str.push(' ');
                    constructed_str.push_str(arg_val);
                } else {
                    constructed_str.push(' ');
                    constructed_str.push_str(arg_val);
                }
            }
        }
    }
    Ok(app.try_get_matches_from(constructed_str.split_ascii_whitespace()))
}

pub async fn execute_command<T>(
    matches: &clap::Result<ArgMatches>,
    cmd_ctx: &T,
    config: &Config,
    dsa_data: &DSAData,
    output: &mut impl OutputWrapper,
) where
    T: CommandContext + Send + Sync,
{
    let matches = match matches {
        Err(e) => {
            output.output_line(&format!("{}", e));
            return;
        }
        Ok(m) => m,
    };
    match matches.subcommand() {
        Some(("upload", _)) => match upload_character(cmd_ctx, config).await {
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
        },

        Some(("check", sub_m)) => {
            match try_get_character(cmd_ctx).await {
                Ok(character) => {
                    dsa::talent_check(sub_m, &character, dsa_data, config, output);
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

        Some(("attack", sub_m)) => {
            match try_get_character(cmd_ctx).await {
                Ok(character) => {
                    dsa::attack_check(sub_m, &character, dsa_data, output);
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

        Some(("spell", sub_m)) => {
            match try_get_character(cmd_ctx).await {
                Ok(character) => {
                    dsa::spell_check(sub_m, &character, dsa_data, config, output);
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

        Some(("dodge", sub_m)) => {
            match try_get_character(cmd_ctx).await {
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

        Some(("parry", sub_m)) => {
            match try_get_character(cmd_ctx).await {
                Ok(character) => {
                    dsa::parry_check(sub_m, &character, dsa_data, output);
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

        Some(("roll", sub_m)) => {
            dsa::roll(sub_m, output);
        }

        Some(("ini", sub_m)) => {
            match initiative(&sub_m, cmd_ctx, output).await {
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
        _ => {}
    };
}

async fn try_get_character(cmd_ctx: &impl CommandContext) -> Result<Character, Error> {
    let mut char_path = config::get_config_dir()?;
    char_path.push("discord_characters");
    char_path.push(cmd_ctx.sender()?.to_string());
    if !std::path::Path::exists(&char_path) {
        return Err(Error::new(
            "Error loading character: No character found for your discord account",
            ErrorType::InvalidInput(InputErrorType::MissingCharacter),
        ));
    }
    Character::from_file(&char_path).await
}

async fn upload_character(
    cmd_ctx: &impl CommandContext,
    config: &Config,
) -> Result<Character, Error> {
    //Attachement validation
    let attachments = cmd_ctx.attachments().await?;
    if attachments.len() != 1 {
        return Err(Error::new(
            format!("Invalid number of attachements: {}", attachments.len()),
            ErrorType::InvalidInput(InputErrorType::InvalidAttachements),
        ));
    } else if attachments[0].size > config.discord.max_attachement_size {
        return Err(Error::new(
            format!("Attachement too big ({} bytes)", attachments[0].size),
            ErrorType::InvalidInput(InputErrorType::InvalidAttachements),
        ));
    }
    //Get character path
    let mut char_path = config::get_config_dir()?;
    char_path.push("discord_characters");
    fs::create_dir_all(&char_path).await?;
    char_path.push(cmd_ctx.sender()?.to_string());
    //Download data
    let data = attachments[0].download().await?;
    //Open file
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&char_path)
        .await?;
    //Write
    let mut writer =
        io::BufWriter::with_capacity(config.discord.max_attachement_size as usize, file);
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

async fn initiative<T>(
    sub_m: &clap::ArgMatches,
    cmd_ctx: &T,
    output: &mut impl OutputWrapper,
) -> Result<(), Error>
where
    T: CommandContext + Sync,
{
    let config_path = get_config_dir()?;

    //Reset trumps all other arguments
    if sub_m.is_present("reset") {
        let members = cmd_ctx.members_in_channel().await?;
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
                            //Force moves
                            let member = member;
                            let new_name = new_name;
                            if let Err(e) = cmd_ctx.rename_member(&member, &new_name).await {
                                println!("Error changing user nickname: {:?}", e);
                            }
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
        let members = cmd_ctx.members_in_channel().await?;

        for member in members {
            let user_id = member.user.id.to_string();
            let mut path = PathBuf::from(&config_path);
            path.push("discord_characters");
            path.push(user_id);
            if std::path::Path::exists(&path) {
                match Character::from_file(&path).await {
                    Err(_) => {
                        return Err(Error::new(
                            format!("Unable to retrieve character for {}", member.display_name()),
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
        path.push(cmd_ctx.sender()?.to_string());
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
                    let roll = roll;
                    let member = characters_members[roll.0].as_ref().unwrap();
                    let new_name = new_name;
                    if let Err(e) = cmd_ctx.rename_member(&member, &new_name).await {
                        println!("Error changing user nickname: {:?}", e);
                    }
                });
            }
        }

        futures::future::join_all(rename_futs).await;
    }
    Ok(())
}
