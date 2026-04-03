use std::{sync::RwLock, time::Duration};

use crate::{
    character_manager::CharacterManager,
    config::{Config, DSAData},
};
use ::serenity::all::{CreateInteractionResponseMessage, CreateMessage};
use anyhow::Error;
use futures::StreamExt;
use poise::{serenity_prelude as serenity, CreateReply};
use serenity::all::{CreateActionRow, CreateInputText, InputTextStyle};

struct Data {
    character_manager: RwLock<CharacterManager>,
    config: Config,
    dsa_data: DSAData,
}
type Context<'a> = poise::Context<'a, Data, Error>;

// TODO: Things to implement:
// - List characters
// - Delete a character
// - Change a character
// - Upload a new character
// - Allow access to a character in a specific channel (low-prio)
#[poise::command(slash_command)]
async fn characters(ctx: Context<'_>) -> Result<(), Error> {
    let components: Vec<CreateActionRow> = Vec::new();

    let text_uuid = ctx.id().to_string() + "_text";

    let reply = CreateReply::default()
        .content("test")
        .components(components);

    let handle = ctx.send(reply).await?;
    let mut interaction_stream = handle
        .message()
        .await?
        .await_component_interaction(&ctx.serenity_context().shard)
        .timeout(Duration::from_mins(15))
        .stream();
    while let Some(interaction) = interaction_stream.next().await {
        interaction
            .create_response(
                &ctx,
                serenity::CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("received interaction"),
                ),
            )
            .await?;
    }
    Ok(())
}

// TODO: Implement a general `check` function & wrapper commands for all these check types. This should support
// - Facilitation
// - Facilitation for specific attributes
// - Bonus points for the check
// - Selecting one of the own characters
// - Selecting a character of a different user in the channel (low-prio)
enum CheckType {
    Attribute(String),
    Skill(String),
    Spell(String),
    Chant(String),
    Attack(String),
    Parry(String),
    Dodge,
}

// TODO: Things to implement
// - facilitation
// - roll for multiple characters in the channel
// - add custom characters
#[poise::command(slash_command)]
async fn initiative(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

// TODO: Implement
#[poise::command(slash_command)]
async fn roll(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

// TODO: Implement
#[poise::command(slash_command)]
async fn hi(ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
