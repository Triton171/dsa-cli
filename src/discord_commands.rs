use crate::character_manager::CharacterManager;

use super::character::Character;
use super::config::*;
use super::dsa;
use super::util::*;
use clap::{App, Arg, ArgMatches, ArgSettings};
use futures::stream::StreamExt;
use serde_json::Value;
use std::borrow::Borrow;
use std::iter::Iterator;
use std::ops::Deref;
use substring::Substring;

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
pub trait CommandContext: Sync {
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
            .edit(&self.context().http, |edit| {
                edit.nickname((*new_name).substring(0, 32))
            })
            .await?;
        Ok(())
    }

    async fn get_guild_owner(&self) -> Result<Option<UserId>, Error> {
        let channel_id = self.channel()?;
        let channel = channel_id.to_channel(self.context()).await?;

        let channel = match channel.guild() {
            Some(gc) => gc,
            None => {
                return Ok(None);
            }
        };

        let guild = match channel.guild(self.context()).await {
            Some(guild) => guild,
            None => {
                return Ok(None);
            }
        };

        Ok(Some(guild.owner_id))
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
    //The code that's commented out can be used for testing, as guild commands refresh faster than global commands
    /*let test_server = serenity::model::id::GuildId();
    if let Ok(cmds) = test_server.get_application_commands(&ctx).await {
        for c in cmds {
            if let Err(e) = test_server.delete_application_command(&ctx, c.id).await {
                println!("Error deleting guild application command: {}", e);
            }
        }
    }
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
        if arg.get_name().contains("user_id") {
            slash_cmd_option.kind(ApplicationCommandOptionType::User);
        } else {
            slash_cmd_option.kind(ApplicationCommandOptionType::String);
        }
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
    character_manager: &RwLock<CharacterManager>,
    cmd_ctx: &T,
    config: &Config,
    dsa_data: &DSAData,
    output: &mut impl OutputWrapper,
) where
    T: CommandContext,
{
    let matches = match matches {
        Err(e) => {
            output.output_line(&format!("{}", e));
            return;
        }
        Ok(m) => m,
    };
    match matches.subcommand() {
        Some(("upload", _)) => {
            //Attachement validation
            let attachments = match cmd_ctx.attachments().await {
                Ok(a) => a,
                Err(e) => {
                    output.output_line(&"Internal server error");
                    println!("Error reading attachments: {}", e);
                    return;
                }
            };
            if attachments.len() != 1 {
                output.output_line(&format!(
                    "Invalid number of attachments: {}",
                    attachments.len()
                ));
                return;
            } else if attachments[0].size > config.discord.max_attachement_size {
                output.output_line(&format!(
                    "Attachment too big ({} bytes)",
                    attachments[0].size
                ));
                return;
            }
            let data = match attachments[0].download().await {
                Ok(d) => d,
                Err(e) => {
                    output.output_line(&"Internal server error");
                    println!("Error downloading attachment: {}", e);
                    return;
                }
            };
            let sender = match cmd_ctx.sender() {
                Ok(s) => s,
                Err(e) => {
                    output.output_line(&"Internal server error");
                    println!("Error getting sender user id: {}", e);
                    return;
                }
            };
            match character_manager
                .write()
                .await
                .add_character(*sender.as_u64(), data, config)
                .await
            {
                Ok((true, name)) => {
                    output.output_line(&format!("Successfully replaced character \"{}\"", name));
                }
                Ok((false, name)) => {
                    output.output_line(&format!("Successfully uploaded character \"{}\"", name));
                }
                Err(e) => match e.err_type() {
                    ErrorType::InvalidInput(_) => {
                        output.output_line(&e);
                    }
                    _ => {
                        output.output_line(&"Internal server error");
                        println!("Error adding character: {}", e);
                    }
                },
            };
        }

        Some(("list", _)) => {
            let sender = match cmd_ctx.sender() {
                Ok(s) => s,
                Err(e) => {
                    output.output_line(&"Internal serverv error");
                    println!("Error getting sender: {}", e);
                    return;
                }
            };
            let (selected, non_selected) = character_manager
                .read()
                .await
                .list_characters(*sender.as_u64());
            if let Some(selected) = selected {
                output.output_line(&"Selected character:");
                output.output_line(&selected);
                output.output_line(&"");
            } else {
                if non_selected.is_empty() {
                    output.output_line(&"No character found for your discord account");
                } else {
                    output.output_line(&"No character currently selected");
                    output.output_line(&"");
                }
            }
            if !non_selected.is_empty() {
                output.output_line(&"Other characters:");
                for name in non_selected {
                    output.output_line(&name);
                }
            }
        }

        Some(("select", sub_m)) => {
            let sender = match cmd_ctx.sender() {
                Ok(s) => s,
                Err(e) => {
                    output.output_line(&"Internal serverv error");
                    println!("Error getting sender: {}", e);
                    return;
                }
            };
            match character_manager
                .write()
                .await
                .select_character(*sender.as_u64(), sub_m.value_of("character_name").unwrap())
                .await
            {
                Ok(name) => {
                    output.output_line(&format!("Successfully selected character \"{}\"", name));
                }
                Err(e) => match e.err_type() {
                    ErrorType::InvalidInput(_) => {
                        output.output_line(&e);
                    }
                    _ => {
                        output.output_line(&"Internal server error");
                        println!("Error selecting character: {}", e);
                    }
                },
            }
        }

        Some(("remove", sub_m)) => {
            let sender = match cmd_ctx.sender() {
                Ok(s) => s,
                Err(e) => {
                    output.output_line(&"Internal serverv error");
                    println!("Error getting sender: {}", e);
                    return;
                }
            };
            match character_manager
                .write()
                .await
                .delete_character(*sender.as_u64(), sub_m.value_of("character_name").unwrap())
                .await
            {
                Ok(removed_characters) => {
                    if removed_characters.is_empty() {
                        output.output_line(&"No character matched the given name, use the \"characters\" command to see a list of uploaded characters");
                    } else {
                        output.output_line(&"Successfully removed characters:");
                        for c in removed_characters {
                            output.output_line(&c);
                        }
                    }
                }
                Err(e) => {
                    output.output_line(&"Internal server error");
                    println!("Error removing character: {}", e);
                }
            }
        }

        Some(("attribute", sub_m)) => {
            execute_character_command(
                &dsa::attribute_check,
                sub_m,
                character_manager.read().await,
                dsa_data,
                config,
                cmd_ctx,
                output,
            )
            .await;
        }
        Some(("check", sub_m)) => {
            execute_character_command(
                &dsa::talent_check,
                sub_m,
                character_manager.read().await,
                dsa_data,
                config,
                cmd_ctx,
                output,
            )
            .await;
        }
        Some(("attack", sub_m)) => {
            execute_character_command(
                &dsa::attack_check,
                sub_m,
                character_manager.read().await,
                dsa_data,
                config,
                cmd_ctx,
                output,
            )
            .await;
        }
        Some(("spell", sub_m)) => {
            execute_character_command(
                &dsa::spell_check,
                sub_m,
                character_manager.read().await,
                dsa_data,
                config,
                cmd_ctx,
                output,
            )
            .await;
        }
        Some(("chant", sub_m)) => {
            execute_character_command(
                &dsa::chant_check,
                sub_m,
                character_manager.read().await,
                dsa_data,
                config,
                cmd_ctx,
                output,
            )
            .await;
        }
        Some(("dodge", sub_m)) => {
            execute_character_command(
                &dsa::dodge_check,
                sub_m,
                character_manager.read().await,
                dsa_data,
                config,
                cmd_ctx,
                output,
            )
            .await;
        }
        Some(("parry", sub_m)) => {
            execute_character_command(
                &dsa::parry_check,
                sub_m,
                character_manager.read().await,
                dsa_data,
                config,
                cmd_ctx,
                output,
            )
            .await;
        }
        Some(("roll", sub_m)) => {
            dsa::roll(sub_m, output);
        }

        Some(("ini", sub_m)) => {
            match initiative(character_manager.read().await, &sub_m, cmd_ctx, output).await {
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

        Some(("rename", sub_m)) => {
            match rename(character_manager.read().await, &sub_m, cmd_ctx, output).await {
                Ok(()) => {}
                Err(e) => match e.err_type() {
                    ErrorType::InvalidInput(_) => {
                        output.output_line(&e);
                    }
                    _ => {
                        output.output_line(&"Internal server error while setting nicknames");
                        println!("Error setting nicknames: {:?}", e);
                    }
                },
            };
        }
        _ => {}
    };
}

async fn execute_character_command<O>(
    check_fn: impl Fn(&ArgMatches, &Character, &DSAData, &Config, &mut O),
    matches: &ArgMatches,
    character_manager: impl Deref<Target = CharacterManager>,
    dsa_data: &DSAData,
    config: &Config,
    ctx: &impl CommandContext,
    output: &mut O,
) where
    O: OutputWrapper,
{
    let character_manager = character_manager.borrow();
    let character_id = match matches.value_of("user_id") {
        None => {
            character_manager
                .find_character(ctx, matches.value_of("character_name"))
                .await
        }
        Some(id) => {
            let id = match id.parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    output.output_line(&"Found invalid user id");
                    return;
                }
            };
            character_manager
                .find_character_for_user(id, matches.value_of("character_name"))
                .await
        }
    };
    let character_id = match character_id {
        Ok(id) => id,
        Err(e) => match e.err_type() {
            ErrorType::InvalidInput(_) => {
                output.output_line(&e);
                return;
            }
            _ => {
                output.output_line(&"Internal server error while matching character");
                println!("Error matching character: {}", e);
                return;
            }
        },
    };
    let character = match character_manager.get_character(character_id).await {
        Ok(c) => c,
        Err(e) => match e.err_type() {
            ErrorType::InvalidInput(_) => {
                output.output_line(&e);
                return;
            }
            _ => {
                output.output_line(&"Internal server error while loading character");
                println!("Error loading character: {}", e);
                return;
            }
        },
    };
    check_fn(matches, &character, dsa_data, config, output);
}

async fn initiative<T>(
    character_manager: impl Deref<Target = CharacterManager>,
    sub_m: &clap::ArgMatches,
    cmd_ctx: &T,
    output: &mut impl OutputWrapper,
) -> Result<(), Error>
where
    T: CommandContext,
{
    //Reset trumps all other arguments
    if sub_m.is_present("reset") {
        let members = cmd_ctx.members_in_channel().await?;
        let mut rename_futs = Vec::new();
        for member in members {
            let user_id = *member.user.id.as_u64();
            /*
            Reset the nickname if all of the following apply
            1. The user has uploaded a character
            2. The user has a discord nickname
            3. The discord nickname is of the form "[i64](,[i64]...,[i64]) orig_name"
            */
            if let Ok(character_id) = character_manager
                .find_character_for_user(user_id, None::<String>)
                .await
            {
                if let Some(nickname) = member.nick.clone() {
                    let mut new_name = String::default();
                    if nickname.contains('Ξ') {
                        // cool name
                        match character_manager.get_character_name(user_id, character_id) {
                            Err(_) => {
                                return Err(Error::new(
                                    format!("Unable to retrieve character for {}", member.display_name()),
                                    ErrorType::InvalidInput(InputErrorType::InvalidFormat),
                                ));
                            }
                            Ok(character_name) => {
                                let display_name = member.display_name();
                                let display_name = display_name.split(" Ξ ").last().unwrap();
                                new_name = calculate_name(&character_name, &display_name, 32)?;
                            }
                        };
                    } else if let Some(index) = nickname.find(' ') {
                        if !nickname[..index]
                            .split(',')
                            .all(|ini_part| ini_part.parse::<i64>().is_ok())
                        {
                            continue;
                        }
                        new_name = nickname[index + 1..].to_string();
                    }
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
            let user_id = *member.user.id.as_u64();
            if let Ok(character_id) = character_manager
                .find_character_for_user(user_id, None::<String>)
                .await
            {
                match character_manager.get_character(character_id).await {
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
        if let Ok(character_id) = character_manager
            .find_character(cmd_ctx, None::<String>)
            .await
        {
            let character = character_manager.get_character(character_id).await?;
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
                let ini_str = roll.1.iter().skip(1).fold(
                    format!("{}", character.1 + roll.1[0]),
                    |mut s, roll| {
                        s.push_str(&format!(",{}", roll));
                        s
                    },
                );
                let discord_name = displ_name.split(" Ξ ").last().unwrap();
                let suffix = calculate_name(&character.0, &discord_name, 32 - ini_str.len())?;
                let new_name = match displ_name.contains('Ξ') { // only use cool renameing if already used rename
                    true => format!("{} {}", ini_str, suffix),
                    false => format!("{} {}", ini_str, displ_name)
                };
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

async fn rename<T>(
    character_manager: impl Deref<Target = CharacterManager>,
    sub_m: &clap::ArgMatches,
    cmd_ctx: &T,
    output: &mut impl OutputWrapper,
) -> Result<(), Error>
where
    T: CommandContext + Sync,
{
    if sub_m.is_present("reset") {
        let members = cmd_ctx.members_in_channel().await?;
        let mut rename_futs = Vec::new();
        for member in members {
            let user_id = *member.user.id.as_u64();
            /*
            Reset the nickname if all of the following apply
            1. The user has uploaded a character
            2. The user has a discord nickname
            3. The discord nickname is of the form ".* Ξ orig_name"
            */
            if let Ok(_) = character_manager
                .find_character_for_user(user_id, None::<String>)
                .await
            {
                if let Some(nickname) = member.nick.clone() {
                    if let Some(index) = nickname.find('Ξ') {
                        let new_name = nickname[index + 2..].to_string();
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

    let members = cmd_ctx.members_in_channel().await?;
    let mut rename_futs = Vec::new();
    for member in members {
        let user_id = *member.user.id.as_u64();

        let mut nickname = member.user.name.clone();
        if let Some(nick) = member.nick.clone() {
            nickname = nick;
        }

        if nickname.clone().contains('Ξ') {
            let out = format!(
                "\"{}\"s nickname contains the symbol Ξ. This is not allowed...",
                nickname
            );
            output.output_line(&out);
            println!("{}", out);
            return Ok(());
        }
        if let Ok(character_id) = character_manager
            .find_character_for_user(user_id, None::<String>)
            .await
        {
            match character_manager.get_character_name(user_id, character_id) {
                Err(_) => {
                    return Err(Error::new(
                        format!("Unable to retrieve character for {}", member.display_name()),
                        ErrorType::InvalidInput(InputErrorType::InvalidFormat),
                    ));
                }
                Ok(character_name) => {
                    let new_name = calculate_name(&character_name, &nickname, 32)?;

                    rename_futs.push(async {
                        let member = member;
                        let new_name = new_name;
                        if let Err(e) = cmd_ctx.rename_member(&member, &new_name).await {
                            if e.message() == "Missing Permissions" {
                                match &cmd_ctx.get_guild_owner().await {
                                    Ok(Some(owner)) => {
                                        if owner == &member.user.id {
                                            return Ok(Some(format!(
                                                "Unable to change server owners nickname to {}",
                                                new_name
                                            )));
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            println!("Error changing user nickname: {:?}", e);
                            return Err(Error::new(
                                format!(
                                    "Unable to change nickname for {}: {}",
                                    member.display_name(),
                                    e.message()
                                ),
                                ErrorType::IO(IOErrorType::Discord),
                            ));
                        }
                        Ok(None)
                    });
                }
            };
        }
    }
    let res = futures::future::join_all(rename_futs).await;
    let iter = res.iter();
    let succ = iter.clone().all(|res| res.is_ok());

    if !succ {
        output.output_line(&"Error setting nicknames:");
        iter.clone()
            .filter(|e| !e.is_ok())
            .for_each(|e| output.output_line(&e.as_ref().err().unwrap().message()));
    } else {
        output.output_line(&"Set nicknames");
    }

    iter.filter(|e| e.is_ok() && e.as_ref().ok().unwrap().is_some())
        .for_each(|e| output.output_line(&e.as_ref().unwrap().clone().unwrap()));

    Ok(())
}

fn calculate_name(character_name: &str, org_name: &str, limit: usize) -> Result<String, Error> {

    let min_length = org_name.len() + 3; // new display_name must include orig_name + ` Ξ `
    if min_length > limit {
        return Err(Error::new(
            format!("{}: Display Name is too large!", org_name),
            ErrorType::InvalidInput(InputErrorType::InvalidDiscordContext),
        ));
    } 

    // name should be `fist_name [[...] last_name] Ξ discord_name`
    let mut character_split = character_name.split(" ");
    let mut allowed_character_len = limit - org_name.len() - 3; // we only want our character name to be limit - original name - 3 padding
    let mut character_name = String::default();

    let first_name = character_split.next();
    let character_split_len = character_split.clone().count();

    if first_name.is_none() {
        //?????
        return Err(Error::new(
            format!("{}: Invalid first name!", org_name),
            ErrorType::InvalidInput(InputErrorType::InvalidArgument),
        ));
    }

    let first_name = first_name.unwrap();

    if first_name.len() > allowed_character_len {
        // we don't fit our first name :(
        character_name.push_str(&first_name[..allowed_character_len]);
    } else {
        character_name.push_str(&first_name);
        allowed_character_len -= first_name.len();

        let last_name = character_split.clone().last().unwrap_or("");

        if allowed_character_len > last_name.len() + 1 {
            // we fit our last name + 1 space padding
            // only use lastname if not cut-off
            allowed_character_len -= last_name.len() + 1;

            for (index, mid_name) in character_split.enumerate() {
                if index >= character_split_len - 1 || mid_name.len() + 1 > allowed_character_len {
                    // break if  last name or if size does not fit with 1 space padding
                    break;
                }
                allowed_character_len -= mid_name.len() + 1;
                character_name.push_str(" ");
                character_name.push_str(mid_name);
            }

            character_name.push_str(" ");
            character_name.push_str(last_name);
        }
    }

    Ok(format! {"{} Ξ {}", character_name, org_name})
}
