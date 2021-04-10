use super::config::Config;
use super::character::Character;
use super::config;
use super::cli;
use super::dsa;
use super::util::IOErrorType;
use super::util::OutputWrapper;
use async_std::prelude::*;
use async_std::fs;
use async_std::io;
use tokio::runtime::Builder;
use serenity::{
    prelude::*, 
    async_trait, 
    model::{
        gateway::Ready,
        channel::Message,
        id::{
            ChannelId,
            UserId
        }
    }
};


struct Handler {
    config: Config
}

#[async_trait]
impl EventHandler for Handler {

    async fn ready(&self, _: Context, ready: Ready) {
        println!("Started bot with username: {}", ready.user.name);
    }

    async fn message(&self, ctx: Context, message: Message) {
        let mut output = match self.config.discord.use_reply {
            true => DiscordOutputWrapper::new_reply_to(&message),
            false => DiscordOutputWrapper::new_simple_message(message.channel_id)
        };

        if message.content.starts_with('!') {
            println!("Received command: \"{}\"", message.content);
            let matches = cli::get_discord_app().try_get_matches_from( {
                let command = &message.content[1..];
                let args: Box<dyn Iterator<Item=&str>> = if self.config.discord.require_complete_command {
                    Box::new(command.split(' '))
                } else {
                    Box::new(std::iter::once("dsa-cli").chain(command.split(' ')))
                };
                args
            });
            let matches = match matches {
                Err(e) => {
                    output.output_line(format!("{}", e));
                    output.send(&ctx).await;
                    return;
                }
                Ok(m) => m
            };
            match matches.subcommand() {
                Some(("upload", _)) => {
                    //Attachement validation
                    if message.attachments.len()!=1 {
                        output.output_line(format!("Invalid number of attachements: {}", message.attachments.len()));
                        output.send(&ctx).await;
                        return;
                    } else if message.attachments[0].size > self.config.discord.max_attachement_size {
                        output.output_line(format!("Attachement too big ({} bytes)", message.attachments[0].size));
                        output.send(&ctx).await;
                        return;
                    }
                    //Get character path
                    let mut char_path = match config::get_config_dir() {
                        Err(_) => {
                            output.output_line(String::from("Error retrieving config directory"));
                            output.send(&ctx).await;
                            return;
                        }
                        Ok(config_dir) => config_dir
                    };
                    char_path.push("discord_characters");
                    match fs::create_dir_all(&char_path).await {
                        Err(_) => {
                            output.output_line(String::from("Error creating character folder"));
                            output.send(&ctx).await;
                            return;
                        }
                        Ok(()) => ()
                    };
                    char_path.push(message.author.id.to_string());
                    //Download data
                    let data = match message.attachments[0].download().await {
                        Err(_) => {
                            output.output_line(String::from("Error downloading attachement"));
                            output.send(&ctx).await;
                            return;
                        }
                        Ok(data) => data
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
                            output.output_line(String::from("Error creating character file"));
                            output.send(&ctx).await;
                            return;
                        }
                        Ok(f) => f
                    };
                    //Write
                    let mut writer = io::BufWriter::new(file);
                    if let Err(_) = writer.write(&data).await {
                        output.output_line(String::from("Error writing to character file"));
                    }
                    if let Err(_) = writer.flush().await {
                        output.output_line(String::from("Error writing to character file (Unable to flush output streamm)"));
                    }
                    match Character::from_file(&char_path) {
                        Ok(c) => {
                            output.output_line(format!("Successfully loaded character \"{}\"", c.get_name()));
                        }
                        Err(e) => {
                            match e.err_type() {
                                IOErrorType::InvalidFormat => {
                                    output.output_line(String::from("Error loading character: Invalid character file"));
                                }
                                _ => {
                                    output.output_line(String::from("Unknown error while loading character"));
                                }
                            }
                        }
                    };
                    output.send(&ctx).await;
                }

                Some(("skillcheck", sub_m)) => {
                    let character = match try_get_character(&message.author.id) {
                        Ok(c) => c,
                        Err(e) => {
                            output.output_line(e);
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
                            output.output_line(e);
                            output.send(&ctx).await;
                            return;
                        }
                    };
                    dsa::attack_check(sub_m, &character, &mut output);
                    output.send(&ctx).await;
                }

                Some(("roll", sub_m)) => {
                    dsa::roll(sub_m, &mut output);
                    output.send(&ctx).await;
                }

                _ => {}
            };
        }

        
    }
}

enum DiscordOutputType<'a> {
    SimpleMessage(ChannelId),
    ReplyTo(&'a Message)
}

//A lazy output wrapper for sending discord messages
struct DiscordOutputWrapper<'a> {
    output_type: DiscordOutputType<'a>,
    msg_buf: String
}

impl<'a> DiscordOutputWrapper<'a> {
    fn new_simple_message(channel_id: ChannelId) -> DiscordOutputWrapper<'a> {
        DiscordOutputWrapper {
            output_type: DiscordOutputType::SimpleMessage(channel_id),
            msg_buf: String::from("```")
        }
    }

    fn new_reply_to(message: &'a Message) -> DiscordOutputWrapper<'a> {
        DiscordOutputWrapper {
            output_type: DiscordOutputType::ReplyTo(message),
            msg_buf: String::from("```")
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
            DiscordOutputType::ReplyTo(msg) => {
                match msg.reply(&ctx.http, &self.msg_buf).await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error sending reply message: {}", e);
                    }
                }
            }
        }
    }
}

impl<'a> OutputWrapper for DiscordOutputWrapper<'a> {

    fn output(&mut self, msg: String) {
        self.msg_buf.push_str(&msg);
    }
    fn output_line(&mut self, msg: String) {
        self.msg_buf.push_str(&msg);
        self.msg_buf.push('\n');
    }
    fn output_table(&mut self, table: &Vec<Vec<String>>) {
        for row in table {
            for entry in row {
                self.msg_buf.push_str(&format!("{:<17}", entry));
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

    let handler = Handler {
        config
    };
    
    let runtime = Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut client = match Client::builder(&login_token)
            .event_handler(handler)
            .await
        {
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




fn try_get_character(user_id: &UserId) -> Result<Character, String>{
    let mut char_path = match config::get_config_dir() {
        Ok(p) => p,
        Err(_) => {
            return Err(String::from("Error retrieving config directory"));
        }
    };
    char_path.push("discord_characters");
    char_path.push(user_id.to_string());
    if !std::path::Path::exists(&char_path) {
        return Err(String::from("Error loading character: No character found for your discord account"));
    }

    match Character::from_file(&char_path) {
        Ok(c) => Ok(c),
        Err(e) => {
            Err(format!("Error loading character: {}", e.message()))
        }
    }
}