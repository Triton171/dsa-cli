use crate::{
    character_manager::CharacterManager,
    config::{Config, DSAData},
    util::OutputWrapper,
};

use anyhow::{Context as AnyhowContext, Error};
use serenity::{
    all::{
        ClientBuilder, CommandInteraction, CommandOptionType, CommandType, CreateCommand,
        CreateCommandOption, CreateComponent, CreateInteractionResponse,
        CreateInteractionResponseMessage, EditInteractionResponse, Event, FullEvent, GuildId,
        Interaction, MessageFlags,
    },
    async_trait,
    prelude::*,
};
use std::{convert::TryFrom, fmt::Write, process::exit, sync::Arc};
use tokio::sync::RwLock;

const DISCORD_TABLE_COL_SEP: usize = 4; //The number of whitespaces between 2 table columns

pub async fn run_discord_bot(config: Arc<Config>, dsa_data: Arc<DSAData>) -> Result<(), Error> {
    let character_manager = CharacterManager::init(&config).await?;
    let mut client = setup_discord_client(config, dsa_data, character_manager).await?;
    client.start().await?;
    Ok(())
}

async fn setup_discord_client(
    config: Arc<Config>,
    dsa_data: Arc<DSAData>,
    character_manager: CharacterManager,
) -> Result<Client, Error> {
    let intents = GatewayIntents::non_privileged();
    let token = Token::try_from(config.discord.login_token.clone())?;

    let client = ClientBuilder::new(token, intents)
        .event_handler(Arc::new(DiscordHandler {
            config,
            dsa_data,
            character_manager: RwLock::new(character_manager),
        }))
        .await?;
    Ok(client)
}

async fn register_commands(context: &Context, config: &Config) -> Result<(), Error> {
    let commands = vec![
        CreateCommand::new("characters")
            .description("Manage uploaded characters & upload new ones"),
        CreateCommand::new("talent")
            .description("Start a talent check")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "name",
                    "A uniquely identifying part of the talent name",
                )
                .required(true),
            ),
    ];

    let guild_id = config.discord.test_in_guild_id.map(GuildId::new);
    let commands = commands.into_iter().map(|cmd| {
        cmd.kind(CommandType::ChatInput)
            .execute(context.http(), guild_id)
    });
    futures::future::try_join_all(commands).await?;
    Ok(())
}

pub async fn send_command_interaction_reply(
    ctx: &Context,
    command: &CommandInteraction,
    components: Vec<CreateComponent<'_>>,
) -> anyhow::Result<()> {
    command
        .create_response(
            ctx.http(),
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .flags(MessageFlags::IS_COMPONENTS_V2 | MessageFlags::EPHEMERAL)
                    .components(components),
            ),
        )
        .await
        .context("Error sending command interaction reply")
}
pub async fn edit_command_interaction_reply(
    ctx: &Context,
    command: &CommandInteraction,
    components: Vec<CreateComponent<'_>>,
) -> anyhow::Result<()> {
    command
        .edit_response(
            ctx.http(),
            EditInteractionResponse::new()
                .flags(MessageFlags::IS_COMPONENTS_V2 | MessageFlags::EPHEMERAL)
                .components(components),
        )
        .await
        .context("Error editing command interaction reply")
        .map(|_| ())
}

pub struct DiscordHandler {
    pub config: Arc<Config>,
    pub dsa_data: Arc<DSAData>,
    pub character_manager: RwLock<CharacterManager>,
}

impl DiscordHandler {
    async fn run_command(
        &self,
        context: &Context,
        command: &CommandInteraction,
    ) -> Result<(), Error> {
        let cmd_name = &command.data.name;
        if cmd_name == "characters" {
            self.characters(context, command).await?
        } else if cmd_name == "talent" {
            self.talent(context, command).await?;
        } else {
            return Err(Error::msg(format!("Unknown command name: {}", cmd_name)));
        }
        Ok(())
    }
}

#[async_trait]
impl EventHandler for DiscordHandler {
    fn filter_event(&self, _context: &Context, event: Box<Event>) -> Option<Box<Event>> {
        match *event {
            Event::InteractionCreate(_) | Event::Ready(_) => Some(event),
            _ => None,
        }
    }
    async fn dispatch(&self, context: &Context, event: &FullEvent) {
        match event {
            FullEvent::InteractionCreate {
                interaction: Interaction::Command(command),
                ..
            } => {
                if let Err(e) = self
                    .run_command(context, command)
                    .await
                    .with_context(|| format!("Error running command '{}'", command.data.name))
                {
                    println!("{:?}", e);
                }
            }
            FullEvent::Ready { data_about_bot, .. } => {
                println!("Successfully started bot '{}'", data_about_bot.user.name);
                match register_commands(context, &*self.config)
                    .await
                    .context("Error registering commands")
                {
                    Ok(_) => println!("Successfully registered commands"),
                    Err(e) => {
                        println!("{:?}", e);
                        exit(1);
                    }
                }
            }
            _ => {}
        }
    }
}

//A lazy output wrapper for sending discord messages
pub struct DiscordOutputWrapper {
    msg_buf: String,
    msg_empty: bool,
}

impl DiscordOutputWrapper {
    pub fn new() -> DiscordOutputWrapper {
        DiscordOutputWrapper {
            msg_buf: String::from("```"),
            msg_empty: true,
        }
    }
}

impl OutputWrapper for DiscordOutputWrapper {
    fn output(&mut self, msg: &impl std::fmt::Display) {
        std::write!(self.msg_buf, "{}", msg).unwrap();
        self.msg_empty = false;
    }
    fn output_line(&mut self, msg: &impl std::fmt::Display) {
        std::writeln!(self.msg_buf, "{}", msg).unwrap();
        self.msg_empty = false;
    }
    fn output_table(&mut self, table: &[Vec<String>]) {
        // IMRPOVE: Maybe use markdown formatting?
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
                    .extend(std::iter::repeat_n(' ', col_lengths[col] - entry.len()));
            }
            self.msg_buf.push('\n');
        }
        self.msg_empty = false;
    }
    fn new_line(&mut self) {
        self.msg_buf.push('\n');
    }
}
