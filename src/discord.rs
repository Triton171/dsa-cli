use super::cli;
use super::config::{Config, DSAData};
use super::discord_commands;
use super::util::*;
use clap::ArgMatches;
use serenity::{
    async_trait,
    client::bridge::gateway::GatewayIntents,
    model::{
        channel::Message,
        gateway::Ready,
        id::{ChannelId, GuildId},
        interactions::{ApplicationCommand, Interaction, InteractionResponseType},
    },
    prelude::*,
};
use std::fmt::Write;

const DISCORD_MAX_MESSAGE_LENGTH: usize = 2000;
const DISCORD_TABLE_COL_SEP: usize = 4; //The number of whitespaces between 2 table columns

pub struct Handler {
    pub config: Config,
    pub dsa_data: DSAData,
}

#[async_trait]
pub trait DiscordCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str {
        ""
    }
    fn create_interaction_options(
        &self,
        _: &Handler,
    ) -> Vec<serenity::builder::CreateApplicationCommandOption> {
        vec![]
    }

    async fn handle_slash_command<'a>(
        &self,
        _: &'a Interaction,
        _: &'a Handler,
        _: &'a Context,
    ) -> String {
        String::from("not implemented!")
    }

    async fn execute(
        &self,
        message: &Message,
        handler: &Handler,
        output: &mut DiscordOutputWrapper,
        context: &Context,
        sub_m: &ArgMatches,
    );
}

impl Handler {
    fn new(config: Config, dsa_data: DSAData) -> Handler {
        Handler {
            config: config,
            dsa_data: dsa_data,
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Started bot with username: {}", ready.user.name);
    }

    async fn cache_ready(&self, ctx: Context, _: Vec<GuildId>) {
        let cmds = ApplicationCommand::get_global_application_commands(&ctx).await;

        if cmds.is_ok() {
            let cmds = cmds.ok().unwrap_or(vec![]);
            for cmd in cmds {
                let _ = ApplicationCommand::delete_global_application_command(&ctx, cmd.id);
            }
        }
        if self.config.discord.use_slash_commands {
            if let Err(e) =
                discord_commands::register_slash_commands(cli::get_discord_app(), &ctx).await
            {
                println!("Error registering discord slash commands: {}", e);
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let mut output =
            DiscordOutputWrapper::new(DiscordOutputType::InteractionResponse(&interaction));
        let matches =
            match discord_commands::parse_discord_interaction(&interaction, cli::get_discord_app())
                .await
            {
                Ok(m) => m,
                Err(e) => {
                    output.output_line(&"Error while parsing command");
                    output.send(&ctx).await;
                    println!("Error while parsing command: {:?}", e);
                    return;
                }
            };
        let cmd_context = discord_commands::SlashCommandContext::new(&ctx, &interaction);
        discord_commands::execute_command(
            &matches,
            &cmd_context,
            &self.config,
            &self.dsa_data,
            &mut output,
        )
        .await;
        output.send(&ctx).await;
    }

    async fn message(&self, ctx: Context, message: Message) {
        if message.content.starts_with('!') {
            let mut output = if self.config.discord.use_reply {
                DiscordOutputWrapper::new(DiscordOutputType::ReplyTo(&message))
            } else {
                DiscordOutputWrapper::new(DiscordOutputType::SimpleMessage(message.channel_id))
            };

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
            let cmd_context = discord_commands::MessageContext::new(&ctx, &message);
            discord_commands::execute_command(
                &matches,
                &cmd_context,
                &self.config,
                &self.dsa_data,
                &mut output,
            )
            .await;
            output.send(&ctx).await;
        }
    }
}

pub async fn start_bot(config: Config, dsa_data: DSAData) {
    let login_token = match &config.discord.login_token {
        Some(token) => token.clone(),
        None => {
            println!("Unable to start bot: Missing discord token");
            return;
        }
    };
    let application_id = match &config.discord.application_id {
        Some(app_id) => app_id.clone(),
        None => {
            println!("Unable to start bot: Missing discord application id");
            return;
        }
    };

    let handler = Handler::new(config, dsa_data);

    let mut client = match Client::builder(&login_token)
        .event_handler(handler)
        .application_id(application_id)
        .intents(
            GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::GUILDS
                | GatewayIntents::non_privileged(),
        )
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
}

//A lazy output wrapper for sending discord messages
pub struct DiscordOutputWrapper<'a> {
    output_type: DiscordOutputType<'a>,
    msg_buf: String,
    msg_empty: bool,
}

pub enum DiscordOutputType<'a> {
    SimpleMessage(ChannelId),
    ReplyTo(&'a Message),
    InteractionResponse(&'a Interaction),
}

impl<'a> DiscordOutputWrapper<'a> {
    pub fn new(output_type: DiscordOutputType<'a>) -> DiscordOutputWrapper {
        DiscordOutputWrapper {
            output_type,
            msg_buf: String::from("```"),
            msg_empty: true,
        }
    }

    pub async fn send(&mut self, ctx: &Context) {
        if self.msg_empty {
            return;
        } else if self.msg_buf.as_bytes().len() > DISCORD_MAX_MESSAGE_LENGTH {
            self.msg_buf = format!(
                "```Error: Reply length exceeds the maximum of {}",
                DISCORD_MAX_MESSAGE_LENGTH
            );
        }
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
            DiscordOutputType::InteractionResponse(interaction) => {
                if let Err(e) = interaction
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|data| {
                                data.embed(|f| {
                                    f.color(serenity::utils::Colour::BLITZ_BLUE)
                                        .description(&self.msg_buf)
                                })
                            })
                    })
                    .await
                {
                    println!("Error sending interaction response: {}", e);
                }
            }
        }
        self.msg_buf = String::from("```");
        self.msg_empty = true;
    }
}

impl<'a> OutputWrapper for DiscordOutputWrapper<'a> {
    fn output(&mut self, msg: &impl std::fmt::Display) {
        std::write!(self.msg_buf, "{}", msg).unwrap();
        self.msg_empty = false;
    }
    fn output_line(&mut self, msg: &impl std::fmt::Display) {
        std::write!(self.msg_buf, "{}\n", msg).unwrap();
        self.msg_empty = false;
    }
    fn output_table(&mut self, table: &Vec<Vec<String>>) {
        let num_cols = table.iter().map(|row| row.len()).max().unwrap_or(0);
        let mut col_lengths: Vec<usize> = Vec::with_capacity(num_cols);
        for col in 0..num_cols {
            col_lengths.push(0);
            for row in table {
                col_lengths[col] = std::cmp::max(
                    col_lengths[col],
                    row.get(col).map_or(0, |s| s.len()) + DISCORD_TABLE_COL_SEP,
                );
            }
        }
        if let Some(col_length) = col_lengths.last_mut() {
            *col_length -= DISCORD_TABLE_COL_SEP; //Don't add spacing after the last column
        }

        for row in table {
            for (col, entry) in row.iter().enumerate() {
                self.msg_buf.push_str(entry);
                self.msg_buf
                    .extend(std::iter::repeat(' ').take(col_lengths[col] - entry.len()));
            }
            self.msg_buf.push('\n');
        }
        self.msg_empty = false;
    }
    fn new_line(&mut self) {
        self.msg_buf.push('\n');
    }
}
