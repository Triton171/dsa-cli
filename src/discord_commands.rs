use std::time::Duration;

use crate::{
    character_manager::CharacterManager,
    config::{Config, DSAData},
    discord::{edit_command_interaction_reply, send_command_interaction_reply, DiscordHandler},
};
use anyhow::Error;
use futures::StreamExt;
use serenity::{
    all::{
        ButtonStyle, CollectComponentInteractions, CommandInteraction, ComponentInteraction,
        ComponentInteractionCollector, CreateActionRow, CreateButton, CreateComponent,
        CreateFileUpload, CreateInteractionResponse, CreateInteractionResponseMessage, CreateLabel,
        CreateModal, CreateModalComponent, CreateQuickModal, CreateSeparator, CreateTextDisplay,
        EditInteractionResponse, EditMessage, MessageFlags, ModalInteraction,
        ModalInteractionCollector, QuickModal, UserId,
    },
    prelude::*,
};

const DISCORD_INTERACTION_TIMEOUT: Duration = Duration::from_secs(3600);
// Characters command
const ID_CHAR_NAME: &str = "_character_name";
const ID_ADD_CHAR: &str = "add_character";

enum CheckType {
    Attribute(String),
    Skill(String),
    Spell(String),
    Chant(String),
    Attack(String),
    Parry(String),
    Dodge,
}

impl DiscordHandler {
    pub async fn create_character_menu(&self, user_id: UserId) -> Vec<CreateComponent<'_>> {
        let characters = self
            .character_manager
            .read()
            .await
            .get_characters(user_id.get())
            .clone();

        let mut components = Vec::new();
        // Components per character
        for (idx, character) in characters.into_iter().enumerate() {
            components.push(CreateComponent::ActionRow(CreateActionRow::Buttons(
                vec![CreateButton::new(idx.to_string() + ID_CHAR_NAME)
                    .label(character.name)
                    .disabled(true)]
                .into(),
            )));
            components.push(CreateComponent::Separator(
                CreateSeparator::new().divider(true),
            ));
        }
        components.push(CreateComponent::ActionRow(CreateActionRow::Buttons(
            vec![CreateButton::new(ID_ADD_CHAR)
                .label("Add")
                .style(ButtonStyle::Primary)]
            .into(),
        )));
        components
    }

    async fn upload_character_modal(
        &self,
        ctx: &Context,
        component_interaction: &ComponentInteraction,
    ) -> Result<Option<ModalInteraction>, Error> {
        const ID_UPLOAD_MODAL: &str = "upload_char_modal";
        const ID_UPLOAD_COMP: &str = "upload_char_component";

        let user_id = component_interaction.user.id;
        component_interaction
            .create_response(
                ctx.http(),
                CreateInteractionResponse::Modal(
                    CreateModal::new(ID_UPLOAD_MODAL, "Upload a new character").components(vec![
                        CreateModalComponent::Label(CreateLabel::file_upload(
                            "The .tdc file created in TheDarkAid",
                            CreateFileUpload::new(ID_UPLOAD_COMP).required(true),
                        )),
                    ]),
                ),
            )
            .await?;
        if let Some(modal_interaction) = ModalInteractionCollector::new(ctx)
            .author_id(user_id)
            .channel_id(component_interaction.channel_id)
            .filter(|mci| mci.data.custom_id == ID_UPLOAD_MODAL)
            .timeout(DISCORD_INTERACTION_TIMEOUT)
            .next()
            .await
        {
            let attachments = &modal_interaction.data.resolved.attachments;
            if attachments.len() != 1 {
                return Err(Error::msg(format!(
                    "Expected 1 attachment for the file upload modal, got {}",
                    attachments.len()
                )));
            }
            let raw_character = attachments.iter().next().unwrap().download().await?;
            self.character_manager
                .write()
                .await
                .add_character(user_id.get(), raw_character, self.config.as_ref())
                .await?;
            modal_interaction.defer(ctx.http()).await?;
            Ok(Some(modal_interaction))
        } else {
            Ok(None)
        }
    }

    // TODO: Things to implement:
    // - List characters
    // - Delete a character
    // - Change a character
    // - Upload a new character
    // - Allow access to a character in a specific channel (low-prio)
    pub async fn characters(
        &self,
        ctx: &Context,
        command: &CommandInteraction,
    ) -> Result<(), Error> {
        let user_id = command.user.id;
        send_command_interaction_reply(ctx, command, self.create_character_menu(user_id).await)
            .await?;

        let msg = command.get_response(ctx.http()).await?;
        let msg_id = msg.id;

        while let Some(component_interaction) = ComponentInteractionCollector::new(ctx)
            .author_id(command.user.id)
            .channel_id(command.channel_id)
            .filter(move |mci| mci.message.id == msg_id)
            .timeout(DISCORD_INTERACTION_TIMEOUT)
            .next()
            .await
        {
            if component_interaction.data.custom_id == ID_ADD_CHAR {
                if let Some(modal_interaction) = self
                    .upload_character_modal(ctx, &component_interaction)
                    .await?
                {
                } else {
                    break;
                }
                edit_command_interaction_reply(
                    ctx,
                    command,
                    self.create_character_menu(user_id).await,
                )
                .await?;
            }
        }
        Ok(())
    }

    // TODO: Implement a general `check` function & wrapper commands for all these check types. This should support
    // - Facilitation
    // - Facilitation for specific attributes
    // - Bonus points for the check
    // - Selecting one of the own characters
    // - Selecting a character of a different user in the channel (low-prio)

    // TODO: Things to implement
    // - facilitation
    // - roll for multiple characters in the channel
    // - add custom characters
    async fn initiative(ctx: &Context, command: &CommandInteraction) -> Result<(), Error> {
        Ok(())
    }

    // TODO: Implement
    async fn roll(ctx: &Context, command: &CommandInteraction) -> Result<(), Error> {
        Ok(())
    }

    // TODO: Implement
    async fn hi(ctx: &Context, command: &CommandInteraction) -> Result<(), Error> {
        Ok(())
    }
}
