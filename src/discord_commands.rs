use std::{sync::RwLock, time::Duration};

use crate::{
    character_manager::CharacterManager,
    config::{Config, DSAData},
    discord::{DiscordContext, DiscordData},
};
use anyhow::Error;
use futures::StreamExt;
use poise::{
    serenity_prelude::{
        self as serenity, CreateActionRow, CreateButton, CreateInteractionResponseMessage,
        CreateQuickModal,
    },
    CreateReply, ReplyHandle,
};

pub fn all_discord_commands() -> Vec<poise::Command<DiscordData, Error>> {
    vec![characters()]
}

// TODO: Things to implement:
// - List characters
// - Delete a character
// - Change a character
// - Upload a new character
// - Allow access to a character in a specific channel (low-prio)
#[poise::command(slash_command)]
pub async fn characters(ctx: DiscordContext<'_>) -> Result<(), Error> {
    let user_id = ctx.author().id;
    let character_manager = &ctx.data().character_manager;

    let mut reply_handle: Option<ReplyHandle> = None;

    loop {
        let characters = character_manager
            .read()
            .await
            .get_characters(user_id.get())
            .clone();

        const ID_CHAR_NAME: &str = "_character_name";
        const ID_ADD_CHAR: &str = "add_character";

        let character_buttons = characters.iter().enumerate().map(|(idx, c)| {
            CreateActionRow::Buttons(vec![CreateButton::new(idx.to_string() + ID_CHAR_NAME)
                .label(&c.name)
                .disabled(true)])
        });
        let general_buttons = CreateActionRow::Buttons(vec![CreateButton::new(ID_ADD_CHAR)
            .label("Add")
            .style(serenity::ButtonStyle::Primary)]);

        let components: Vec<CreateActionRow> = character_buttons
            .chain(std::iter::once(general_buttons))
            .collect();

        let reply = CreateReply::default()
            .content("Your Characters:")
            .components(components)
            .ephemeral(true);

        let handle = if let Some(handle) = reply_handle {
            handle.edit(ctx, reply).await?;
            handle
        } else {
            ctx.send(reply).await?
        };

        let mut interaction_stream = handle
            .message()
            .await?
            .await_component_interaction(&ctx.serenity_context().shard)
            .timeout(Duration::from_mins(15))
            .stream();
        if let Some(interaction) = interaction_stream.next().await {
            interaction
                .create_response(
                    &ctx,
                    serenity::CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content(
                                "received interaction: ".to_string() + &interaction.data.custom_id,
                            ),
                    ),
                )
                .await?;
            reply_handle = Some(handle);
            continue;
        }
        break;
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
async fn initiative(ctx: DiscordContext<'_>) -> Result<(), Error> {
    Ok(())
}

// TODO: Implement
#[poise::command(slash_command)]
async fn roll(ctx: DiscordContext<'_>) -> Result<(), Error> {
    Ok(())
}

// TODO: Implement
#[poise::command(slash_command)]
async fn hi(ctx: DiscordContext<'_>) -> Result<(), Error> {
    Ok(())
}
