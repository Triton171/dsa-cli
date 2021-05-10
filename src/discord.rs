use super::cli;
use super::config::{Config, DSAData};
use super::discord_commands::*;
use super::util::*;
use clap::ArgMatches;
use serenity::{
    async_trait,
    client::bridge::gateway::GatewayIntents,
    model::{
        channel::Message,
        gateway::Ready,
        id::GuildId,
        interactions::{Interaction, InteractionResponseType},
    },
    prelude::*,
};
use std::{collections::HashMap, env};

pub struct Handler {
    pub config: Config,
    pub dsa_data: DSAData,
    pub command_registry: DiscordCommandRegistry,
}

pub struct DiscordCommandRegistry {
    _commands: HashMap<String, Box<dyn DiscordCommand>>,
    _names: Vec<String>,
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

    async fn handle_slash_command<'a>(&self, _: &'a Interaction, _: &'a Handler) -> String {
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
            command_registry: DiscordCommandRegistry::new(),
        }
    }
}

impl DiscordCommandRegistry {
    fn new() -> DiscordCommandRegistry {
        DiscordCommandRegistry {
            _commands: HashMap::new(),
            _names: vec![],
        }
    }

    fn get_command(&self, name: &str) -> Option<&Box<dyn DiscordCommand>> {
        self._commands.get(name)
    }

    fn register_command(&mut self, command: Box<dyn DiscordCommand>) {
        let name = command.name().to_string();
        self._commands.insert(name.clone(), command);
        self._names.push(name);
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("Started bot with username: {}", ready.user.name);
    }

    async fn cache_ready(&self, ctx: Context, _: Vec<GuildId>) {
        if self.config.discord.use_slash_commands {
            let test_server = GuildId(match env::var("DISCORD_TEST_SERVER") {
                Ok(val) => val.parse().unwrap_or(839621705701261332),
                _ => 839621705701261332,
            });

            for name in self.command_registry._names.iter() {
                let cmd = self.command_registry.get_command(name.as_str()).unwrap();
                if cmd.description() == "" {
                    continue;
                }
                let a = test_server
                    .create_application_command(&ctx, |fun| {
                        let mut c = fun.name(name).description(cmd.description());
                        let opts = cmd.create_interaction_options(&self);
                        if !opts.is_empty() {
                            c = c.set_options(opts);
                        }
                        c
                    })
                    .await;
                if a.is_err() {
                    println!("Registering '{}': {:?}", name, a);
                }
            }
        } else {
            //todo delete registered slash cmds
        }
        println!("Registered all Slash Commands!");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let name = interaction.clone().data;
        let text = match self
            .command_registry
            .get_command(
                match name {
                    Some(d) => d.name,
                    None => String::new(),
                }
                .as_str(),
            )
            .as_ref()
        {
            None => String::from("Command not found!"), // this should never trigger
            Some(cmd) => cmd.handle_slash_command(&interaction, &self).await,
        };
        let _ = interaction
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|data| {
                        data.embed(|f| {
                            f.color(serenity::utils::Colour::BLITZ_BLUE)
                                .description(text)
                        })
                    })
            })
            .await;
    }

    async fn message(&self, ctx: Context, message: Message) {
        let mut output = if self.config.discord.use_reply {
            DiscordOutputWrapper::new_reply_to(&message)
        } else {
            DiscordOutputWrapper::new_simple_message(message.channel_id)
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
                Some(subcmd) => {
                    match self.command_registry.get_command(subcmd.0) {
                        Some(command) => {
                            command
                                .execute(&message, self, &mut output, &ctx, &subcmd.1)
                                .await;
                            output.send(&ctx).await;
                        }
                        _ => {} // unknown command
                    }
                }
                _ => {} // no command
            };
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

    let mut handler = Handler::new(config, dsa_data);
    register_commands(&mut handler.command_registry);

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

fn register_commands(registry: &mut DiscordCommandRegistry) {
    registry.register_command(Box::new(CommandUpload {}));
    registry.register_command(Box::new(CommandCheck {}));
    registry.register_command(Box::new(CommandAttack {}));
    registry.register_command(Box::new(CommandSpell {}));
    registry.register_command(Box::new(CommandDodge {}));
    registry.register_command(Box::new(CommandParry {}));
    registry.register_command(Box::new(CommandRoll {}));
    registry.register_command(Box::new(CommandIni {}));
}
