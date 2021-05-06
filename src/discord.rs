use super::character::Character;
use super::cli;
use super::config::{self, Config, DSAData};
use super::dsa;
use super::util::*;
use async_std::fs;
use async_std::io;
use async_std::prelude::*;
use futures::stream::StreamExt;
use serenity::{
    async_trait,
    model::{
        channel::{ChannelType, Message},
        gateway::Ready,
        guild::Member,
        id::UserId,
        permissions::Permissions,
    },
    prelude::*,
};
use std::path::PathBuf;
use tokio::runtime::Builder;

struct Handler {
    config: Config,
    dsa_data: DSAData,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Started bot with username: {}", ready.user.name);
    }

    async fn message(&self, ctx: Context, message: Message) {
        let mut output = match self.config.discord.use_reply {
            true => DiscordOutputWrapper::new_reply_to(&message),
            false => DiscordOutputWrapper::new_simple_message(message.channel_id),
        };

        if message.content.starts_with('!') {
            let matches = cli::get_discord_app().try_get_matches_from({
                let command = &message.content[1..];
                let args: Box<dyn Iterator<Item = &str>> =
                    if self.config.discord.require_complete_command {
                        Box::new(command.split(' '))
                    } else {
                        Box::new(std::iter::once("dsa-cli").chain(command.split(' ')))
                    };
                args
            });
            let matches = match matches {
                Err(e) => {
                    output.output_line(&format!("{}", e));
                    output.send(&ctx).await;
                    return;
                }
                Ok(m) => m,
            };
            match matches.subcommand() {
                Some(("upload", _)) => {
                    match upload_character(&message, &self.config).await {
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
                                output
                                    .output_line(&"Internal server error while loading character");
                                println!("Error loading character: {:?}", e);
                            }
                        },
                    }
                    output.send(&ctx).await;
                }

                Some(("check", sub_m)) => {
                    match try_get_character(&message.author.id) {
                        Ok(character) => {
                            dsa::talent_check(
                                sub_m,
                                &character,
                                &self.dsa_data,
                                &self.config,
                                &mut output,
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
                    output.send(&ctx).await;
                }

                Some(("attack", sub_m)) => {
                    match try_get_character(&message.author.id) {
                        Ok(character) => {
                            dsa::attack_check(sub_m, &character, &self.dsa_data, &mut output);
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
                    output.send(&ctx).await;
                }

                Some(("spell", sub_m)) => {
                    match try_get_character(&message.author.id) {
                        Ok(character) => {
                            dsa::spell_check(
                                sub_m,
                                &character,
                                &self.dsa_data,
                                &self.config,
                                &mut output,
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
                    output.send(&ctx).await;
                }

                Some(("dodge", sub_m)) => {
                    match try_get_character(&message.author.id) {
                        Ok(character) => {
                            dsa::dodge_check(sub_m, &character, &mut output);
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
                    output.send(&ctx).await;
                }

                Some(("parry", sub_m)) => {
                    match try_get_character(&message.author.id) {
                        Ok(character) => {
                            dsa::parry_check(sub_m, &character, &self.dsa_data, &mut output);
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
                    output.send(&ctx).await;
                }

                Some(("roll", sub_m)) => {
                    dsa::roll(sub_m, &mut output);
                    output.send(&ctx).await;
                }

                Some(("ini", sub_m)) => {
                    match initiative(&sub_m, &message, &ctx, &mut output).await {
                        Ok(()) => {}
                        Err(e) => match e.err_type() {
                            ErrorType::InvalidInput(_) => {
                                output.output_line(&e);
                            }
                            _ => {
                                output
                                    .output_line(&"Internal server error while rolling initiative");
                                println!("Error rolling initiative: {:?}", e);
                            }
                        },
                    };
                    output.send(&ctx).await;
                }
                _ => {}
            };
        }
    }
}

pub fn start_bot(config: Config, dsa_data: DSAData) {
    let login_token = match &config.discord.login_token {
        Some(token) => token.clone(),
        None => {
            println!("Unable to start bot: Missing discord token");
            return;
        }
    };

    let handler = Handler { config, dsa_data };

    let runtime = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut client = match Client::builder(&login_token).event_handler(handler).await {
            Ok(client) => client,
            Err(e) => {
                println!("Error creating discord client: {}", e.to_string());
                return;
            }
        };

        if let Err(e) = client.start().await {
            println!("Error starting discord client: {}", e.to_string());
        }
    });
}

fn try_get_character(user_id: &UserId) -> Result<Character, Error> {
    let mut char_path = config::get_config_dir()?;
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

async fn upload_character(message: &Message, config: &Config) -> Result<Character, Error> {
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
    match Character::from_file(&char_path) {
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

async fn fetch_discord_members(ctx: &Context, message: &Message) -> Result<Vec<Member>, Error> {
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

    Ok(futures::stream::iter(g_members.iter().map(|m| m.clone())) // fetch members in the channel message was sent in
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
        .await)
}

async fn initiative(
    sub_m: &clap::ArgMatches,
    message: &Message,
    ctx: &Context,
    output: &mut impl OutputWrapper,
) -> Result<(), Error> {
    let config_path = config::get_config_dir()?;

    //Reset trumps all other arguments
    if sub_m.is_present("reset") {
        let members = match fetch_discord_members(ctx, message).await {
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
        let members = match fetch_discord_members(ctx, message).await {
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
                            format!("Unable to retrieve character for {}", member.display_name()),
                            ErrorType::InvalidInput(InputErrorType::InvalidFormat),
                        ));
                    }
                    Ok(character) => {
                        characters.push((
                            character.get_name().to_string(),
                            character.get_initiative_level(),
                        ));
                        characters_members.push(Some(member));
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
