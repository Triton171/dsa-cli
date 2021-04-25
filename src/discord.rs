use super::character::Character;
use super::cli;
use super::config;
use super::config::Config;
use super::dsa;
use super::util::ErrorType;
use super::util::OutputWrapper;
use async_std::fs;
use async_std::io;
use async_std::prelude::*;
use serenity::{
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
        guild::Member,
        id::{ChannelId, UserId},
    },
    prelude::*,
};
use std::fmt;
use std::fmt::Write;
use std::path::PathBuf;
use tokio::runtime::Builder;

struct Handler {
    config: Config,
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
            println!("Received command: \"{}\"", message.content);
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
                    //Attachement validation
                    if message.attachments.len() != 1 {
                        output.output_line(&format!(
                            "Invalid number of attachements: {}",
                            message.attachments.len()
                        ));
                        output.send(&ctx).await;
                        return;
                    } else if message.attachments[0].size > self.config.discord.max_attachement_size
                    {
                        output.output_line(&format!(
                            "Attachement too big ({} bytes)",
                            message.attachments[0].size
                        ));
                        output.send(&ctx).await;
                        return;
                    }
                    //Get character path
                    let mut char_path = match config::get_config_dir() {
                        Err(_) => {
                            output.output_line(&"Error retrieving config directory");
                            output.send(&ctx).await;
                            return;
                        }
                        Ok(config_dir) => config_dir,
                    };
                    char_path.push("discord_characters");
                    match fs::create_dir_all(&char_path).await {
                        Err(_) => {
                            output.output_line(&"Error creating character folder");
                            output.send(&ctx).await;
                            return;
                        }
                        Ok(()) => (),
                    };
                    char_path.push(message.author.id.to_string());
                    //Download data
                    let data = match message.attachments[0].download().await {
                        Err(_) => {
                            output.output_line(&"Error downloading attachement");
                            output.send(&ctx).await;
                            return;
                        }
                        Ok(data) => data,
                    };
                    //Open file
                    let file = match fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&char_path)
                        .await
                    {
                        Err(_) => {
                            output.output_line(&"Error creating character file");
                            output.send(&ctx).await;
                            return;
                        }
                        Ok(f) => f,
                    };
                    //Write
                    let mut writer = io::BufWriter::new(file);
                    if let Err(_) = writer.write(&data).await {
                        output.output_line(&"Error writing to character file");
                    }
                    if let Err(_) = writer.flush().await {
                        output.output_line(
                            &"Error writing to character file (Unable to flush output stream)",
                        );
                    }
                    match Character::from_file(&char_path) {
                        Ok(c) => {
                            output.output_line(&format!(
                                "Successfully loaded character \"{}\"",
                                c.get_name()
                            ));
                        }
                        Err(e) => match e.err_type() {
                            ErrorType::InvalidFormat => {
                                output.output_line(
                                    &"Error loading character: Invalid character file",
                                );
                            }
                            _ => {
                                output.output_line(&"Unknown error while loading character");
                            }
                        },
                    };
                    output.send(&ctx).await;
                }

                Some(("check", sub_m)) => {
                    let character = match try_get_character(&message.author.id) {
                        Ok(c) => c,
                        Err(e) => {
                            output.output_line(&e);
                            output.send(&ctx).await;
                            return;
                        }
                    };
                    dsa::skill_check(sub_m, &character, &self.config, &mut output);
                    output.send(&ctx).await;
                }

                Some(("attack", sub_m)) => {
                    let character = match try_get_character(&message.author.id) {
                        Ok(c) => c,
                        Err(e) => {
                            output.output_line(&e);
                            output.send(&ctx).await;
                            return;
                        }
                    };
                    dsa::attack_check(sub_m, &character, &self.config, &mut output);
                    output.send(&ctx).await;
                }

                Some(("spell", sub_m)) => {
                    let character = match try_get_character(&message.author.id) {
                        Ok(c) => c,
                        Err(e) => {
                            output.output_line(&e);
                            output.send(&ctx).await;
                            return;
                        }
                    };
                    dsa::spell_check(sub_m, &character, &self.config, &mut output);
                    output.send(&ctx).await;
                }

                Some(("dodge", sub_m)) => {
                    let character = match try_get_character(&message.author.id) {
                        Ok(c) => c,
                        Err(e) => {
                            output.output_line(&e);
                            output.send(&ctx).await;
                            return;
                        }
                    };
                    dsa::dodge_check(sub_m, &character, &mut output);
                    output.send(&ctx).await;
                }

                Some(("roll", sub_m)) => {
                    dsa::roll(sub_m, &mut output);
                    output.send(&ctx).await;
                }

                Some(("ini", sub_m)) => {
                    let config_path = match config::get_config_dir() {
                        Ok(p) => p,
                        Err(_) => {
                            output.output_line(&"Unable to retrieve config folder");
                            output.send(&ctx).await;
                            return;
                        }
                    };

                    //Reset trumps all other arguments
                    if sub_m.is_present("reset") {
                        let guild = match message.guild(&ctx.cache).await {
                            Some(g) => g,
                            None => {
                                output.output_line(&"This option can only be used in a server");
                                output.send(&ctx).await;
                                return;
                            }
                        };
                        let members = match guild.members(&ctx.http, Some(50), None).await {
                            Err(e) => {
                                output.output_line(&format!("Unable to get guild members: {}", e));
                                output.send(&ctx).await;
                                return;
                            }
                            Ok(m) => m,
                        };
                        let mut rename_futs = Vec::new();
                        for member in members {
                            let user_id = member.user.id.to_string();
                            let mut path = PathBuf::from(&config_path);
                            path.push("discord_characters");
                            path.push(user_id);
                            if std::path::Path::exists(&path) {
                                if let Some(nickname) = member.nick.clone() {
                                    if let Some(index) = nickname.find('_') {
                                        let new_name = nickname[index + 1..].to_string();
                                        /*rename_futs.push(member.edit(&ctx.http,
                                        |edit| edit.nickname(new_name)));*/
                                        rename_futs.push(async {
                                            let member = member;
                                            match member
                                                .edit(&ctx.http, |edit| edit.nickname(new_name))
                                                .await
                                            {
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
                    }

                    //All (name, ini_level) tuples to include in the check
                    let mut characters: Vec<(String, i64)> = Vec::new();
                    //The user_id for all the characters that have one (currently only used for renaming)
                    let mut characters_members: Vec<Option<Member>> = Vec::new();

                    if sub_m.is_present("all") {
                        //Add all guild member's characters to the list
                        let guild = match message.guild(&ctx.cache).await {
                            Some(g) => g,
                            None => {
                                output.output_line(&"This option can only be used in a server");
                                output.send(&ctx).await;
                                return;
                            }
                        };

                        let members = match guild.members(&ctx.http, Some(50), None).await {
                            Ok(m) => m,
                            Err(e) => {
                                println!("Error getting guild members: {}", e);
                                output.output_line(&format!("Unable to get guild members: {}", e));
                                output.send(&ctx).await;
                                return;
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
                                        output.output_line(&format!(
                                            "Unable to retrieve character for {}",
                                            member.display_name()
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
                            match Character::from_file(&path) {
                                Err(_) => {
                                    output.output_line(&format!(
                                        "Error: Unable to retrieve character"
                                    ));
                                    output.send(&ctx).await;
                                    return;
                                }
                                Ok(character) => {
                                    characters.push((
                                        character.get_name().to_string(),
                                        character.get_initiative_level(),
                                    ));
                                    characters_members.push(None);
                                }
                            }
                        } else {
                            output.output_line(&"No character found for your discord account");
                        }
                    }

                    if sub_m.is_present("new") {
                        let custom_args: Vec<&str> = sub_m.values_of("new").unwrap().collect();
                        if custom_args.len() % 2 != 0 {
                            output.output_line(&"The \"new\" argument expects an even number of values (name and level for each custom character)");
                            output.send(&ctx).await;
                            return;
                        }
                        for custom_char in custom_args.chunks(2) {
                            let ini_level = match custom_char[1].parse::<i64>() {
                                Err(_) => {
                                    output.output_line(&format!(
                                        "Unable to parse custom initiative level: {}",
                                        custom_char[1]
                                    ));
                                    output.send(&ctx).await;
                                    return;
                                }
                                Ok(level) => level,
                            };
                            characters.push((custom_char[0].to_string(), ini_level));
                            characters_members.push(None);
                        }
                    }

                    let rolls = dsa::roll_ini(&characters, &mut output);

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
                                        s.push_str(&format!(".{}", roll));
                                        s
                                    },
                                );
                                new_name.push('_');
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
                                            println!("Error changing user nickname: {}", e);
                                        }
                                    }
                                });
                            }
                        }

                        tokio::join!(output.send(&ctx), futures::future::join_all(rename_futs));
                    } else {
                        output.send(&ctx).await;
                    }
                }
                _ => {}
            };
        }
    }
}

enum DiscordOutputType<'a> {
    SimpleMessage(ChannelId),
    ReplyTo(&'a Message),
}

//A lazy output wrapper for sending discord messages
struct DiscordOutputWrapper<'a> {
    output_type: DiscordOutputType<'a>,
    msg_buf: String,
}

impl<'a> DiscordOutputWrapper<'a> {
    fn new_simple_message(channel_id: ChannelId) -> DiscordOutputWrapper<'a> {
        DiscordOutputWrapper {
            output_type: DiscordOutputType::SimpleMessage(channel_id),
            msg_buf: String::from("```"),
        }
    }

    fn new_reply_to(message: &'a Message) -> DiscordOutputWrapper<'a> {
        DiscordOutputWrapper {
            output_type: DiscordOutputType::ReplyTo(message),
            msg_buf: String::from("```"),
        }
    }

    async fn send(&mut self, ctx: &Context) {
        self.msg_buf.push_str("```");
        match &self.output_type {
            DiscordOutputType::SimpleMessage(channel_id) => {
                match channel_id.say(ctx, &self.msg_buf).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error sending simple message: {}", e);
                    }
                }
            }
            DiscordOutputType::ReplyTo(msg) => match msg.reply(&ctx.http, &self.msg_buf).await {
                Ok(_) => {}
                Err(e) => {
                    println!("Error sending reply message: {}", e);
                }
            },
        }
    }
}

impl<'a> OutputWrapper for DiscordOutputWrapper<'a> {
    fn output(&mut self, msg: &impl fmt::Display) {
        write!(self.msg_buf, "{}", msg).unwrap();
    }
    fn output_line(&mut self, msg: &impl fmt::Display) {
        write!(self.msg_buf, "{}\n", msg).unwrap();
    }
    fn output_table(&mut self, table: &Vec<Vec<String>>) {
        for row in table {
            for entry in row {
                self.msg_buf.push_str(&format!("{:<22}", entry));
            }
            self.msg_buf.push('\n');
        }
    }
    fn new_line(&mut self) {
        self.msg_buf.push('\n');
    }
}

pub fn start_bot(config: Config) {
    let login_token = match &config.discord.login_token {
        Some(token) => token.clone(),
        None => {
            println!("Unable to start bot: Missing discord token");
            return;
        }
    };

    let handler = Handler { config };

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

fn try_get_character(user_id: &UserId) -> Result<Character, String> {
    let mut char_path = match config::get_config_dir() {
        Ok(p) => p,
        Err(_) => {
            return Err(String::from("Error retrieving config directory"));
        }
    };
    char_path.push("discord_characters");
    char_path.push(user_id.to_string());
    if !std::path::Path::exists(&char_path) {
        return Err(String::from(
            "Error loading character: No character found for your discord account",
        ));
    }

    match Character::from_file(&char_path) {
        Ok(c) => Ok(c),
        Err(e) => Err(format!("Error loading character: {}", e.message())),
    }
}
