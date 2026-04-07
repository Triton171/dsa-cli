use std::time::Duration;

use crate::{
    character::Character,
    character_manager::{CharacterId, CharacterInfo},
    config::{DSAData, MatchSearchResult},
    discord::{
        component_interaction_edit_message, edit_command_interaction_reply,
        send_command_interaction_reply, send_component_interaction_reply, DiscordHandler,
        DiscordOutputWrapper,
    },
    dsa::{roll_check, CheckType, CritType, Facilitation},
    util::uppercase_first,
};
use anyhow::{bail, ensure, Context as AnyhowContext, Error};
use serenity::{
    all::{
        ButtonStyle, CommandInteraction, ComponentInteraction, ComponentInteractionCollector,
        CreateActionRow, CreateButton, CreateComponent, CreateFileUpload,
        CreateInteractionResponse, CreateInteractionResponseMessage, CreateLabel, CreateModal,
        CreateModalComponent, CreateSelectMenu, CreateSelectMenuOption, CreateSeparator,
        CreateTextDisplay, ModalInteractionCollector, SeparatorSpacingSize, UserId,
    },
    prelude::*,
};

const DISCORD_INTERACTION_TIMEOUT: Duration = Duration::from_secs(3600);
// Characters command
const ID_CHAR_REPLACE: &str = "_char_repl";
const ID_CHAR_DELETE: &str = "_char_del";
const ID_ADD_CHAR: &str = "add_character";
// Check commands
const ID_SELECT_CHAR: &str = "select_character";
const ID_SELECT_FACILITATION: &str = "select_facilitation";
const ID_ROLL_CHECK: &str = "roll_check";
const ID_ROLL_CHECK_PRIVATE: &str = "roll_check_private";

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
        for character in characters.into_iter() {
            components.push(CreateComponent::TextDisplay(CreateTextDisplay::new(
                character.name,
            )));
            components.push(CreateComponent::ActionRow(CreateActionRow::Buttons(
                vec![
                    CreateButton::new(character.character_id.to_string() + ID_CHAR_REPLACE)
                        .label("Replace character")
                        .style(ButtonStyle::Primary),
                    CreateButton::new(character.character_id.to_string() + ID_CHAR_DELETE)
                        .label("Delete")
                        .style(ButtonStyle::Danger),
                ]
                .into(),
            )));
            components.push(CreateComponent::Separator(
                CreateSeparator::new()
                    .divider(true)
                    .spacing(SeparatorSpacingSize::Large),
            ));
        }
        components.push(CreateComponent::ActionRow(CreateActionRow::Buttons(
            vec![CreateButton::new(ID_ADD_CHAR)
                .label("Add character")
                .style(ButtonStyle::Success)]
            .into(),
        )));
        components
    }

    // Creates a modal for uploading a new character. Returns true, if the user submitted the modal.
    async fn upload_character_modal(
        &self,
        ctx: &Context,
        component_interaction: &ComponentInteraction,
        replace_character: Option<CharacterId>,
    ) -> anyhow::Result<()> {
        const ID_UPLOAD_MODAL: &str = "upload_char_modal";
        const ID_UPLOAD_COMP: &str = "upload_char_component";

        let user_id = component_interaction.user.id;
        component_interaction
            .create_response(
                ctx.http(),
                CreateInteractionResponse::Modal(
                    CreateModal::new(ID_UPLOAD_MODAL, "Upload a character").components(vec![
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
            if let Some(error_text) = self
                .character_manager
                .write()
                .await
                .add_character(
                    user_id.get(),
                    raw_character,
                    replace_character,
                    self.config.as_ref(),
                )
                .await?
            {
                modal_interaction
                    .create_response(
                        ctx.http(),
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(format!("Failed to upload character ({})", error_text)),
                        ),
                    )
                    .await
                    .context("Failed to create modal interaction response")?;
            } else {
                modal_interaction.defer(ctx.http()).await?;
            }
        }
        Ok(())
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
            let interaction_custom_id = &component_interaction.data.custom_id;
            if interaction_custom_id == ID_ADD_CHAR {
                self.upload_character_modal(ctx, &component_interaction, None)
                    .await?;
            } else if let Some(character_id) = interaction_custom_id.strip_suffix(ID_CHAR_REPLACE) {
                let character_id = CharacterId::from(character_id.parse::<u64>()?);
                self.upload_character_modal(ctx, &component_interaction, Some(character_id))
                    .await?;
            } else if let Some(character_id) = interaction_custom_id.strip_suffix(ID_CHAR_DELETE) {
                let character_id = CharacterId::from(character_id.parse::<u64>()?);
                self.character_manager
                    .write()
                    .await
                    .delete_character(user_id.get(), character_id)
                    .await?;
                component_interaction.defer(ctx.http()).await?;
            } else {
                return Err(Error::msg(format!(
                    "Unknown component interaction of type: {:?}",
                    component_interaction.data.kind
                )));
            }
            edit_command_interaction_reply(ctx, command, self.create_character_menu(user_id).await)
                .await?;
        }
        Ok(())
    }

    pub async fn create_check_menu(
        &self,
        check_name: &str,
        characters: &Vec<CharacterInfo>,
        mut selected_character: Option<CharacterId>,
        selected_facilitation: i64,
    ) -> Vec<CreateComponent<'_>> {
        let mut components = Vec::new();

        // TODO: Add level of currently selected character in that skill
        components.push(CreateComponent::TextDisplay(CreateTextDisplay::new(
            format!("Check for {}", uppercase_first(check_name)),
        )));
        // Add a character select menu
        components.push(CreateComponent::ActionRow(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                ID_SELECT_CHAR,
                serenity::all::CreateSelectMenuKind::String {
                    options: characters
                        .iter()
                        .map(|c| {
                            CreateSelectMenuOption::new(
                                c.name.to_string(),
                                c.character_id.to_string(),
                            )
                            .default_selection(Some(c.character_id) == selected_character)
                        })
                        .collect(),
                },
            )
            .placeholder("Select a character"),
        )));
        // Add facilitation select menu
        components.push(CreateComponent::ActionRow(CreateActionRow::SelectMenu(
            CreateSelectMenu::new(
                ID_SELECT_FACILITATION,
                serenity::all::CreateSelectMenuKind::String {
                    options: (-10..=10)
                        .into_iter()
                        .map(|i| {
                            CreateSelectMenuOption::new(
                                "Facilitation: ".to_string() + &i.to_string(),
                                i.to_string(),
                            )
                            .default_selection(i == selected_facilitation)
                        })
                        .collect(),
                },
            ),
        )));
        // Add buttons for rolling the check
        // TODO: Add button for custom facilitation
        components.push(CreateComponent::ActionRow(CreateActionRow::Buttons(
            vec![
                CreateButton::new(ID_ROLL_CHECK)
                    .label("Roll check")
                    .style(ButtonStyle::Success)
                    .disabled(selected_character.is_none()),
                CreateButton::new(ID_ROLL_CHECK_PRIVATE)
                    .label("Roll check privately")
                    .style(ButtonStyle::Secondary)
                    .disabled(selected_character.is_none()),
            ]
            .into(),
        )));
        components
    }

    async fn roll_check_and_reply<A>(
        &self,
        ctx: &Context,
        component_interaction: &ComponentInteraction,
        check_adapter: &A,
        character: CharacterId,
        facilitation: i64,
        ephemeral: bool,
    ) -> anyhow::Result<()>
    where
        A: CheckAdapter,
    {
        let character = self
            .character_manager
            .read()
            .await
            .get_character(character)
            .await?;
        let attrs = check_adapter.get_attribute_short_names();
        let attr_levels = check_adapter.get_attr_levels(&character);
        ensure!(
            attrs.len() == attr_levels.len(),
            format!(
                "There are {} attributes, but {} attribute levels are given",
                attrs.len(),
                attr_levels.len()
            )
        );
        let num_attrs = attrs.len();
        let attrs_with_levels: Vec<_> = attrs
            .iter()
            .zip(attr_levels)
            .map(|(attr, lvl)| (attr.as_str(), lvl))
            .collect();
        let facilitation = Facilitation {
            individual_facilitation: vec![facilitation; num_attrs],
            points_bonus: 0,
            successful_points_bonus: None,
        };
        let check_type = if A::IS_POINTS_CHECKS {
            CheckType::PointsCheck(check_adapter.get_points_level(&character))
        } else {
            CheckType::SimpleCheck
        };
        let crit_type = match self.config.dsa_rules.crit_rules {
            crate::config::ConfigDSACritType::None => CritType::None,
            crate::config::ConfigDSACritType::Default => {
                if num_attrs == 1 {
                    CritType::Confirmable
                } else {
                    CritType::MultipleRequired(2)
                }
            }
            crate::config::ConfigDSACritType::Alternative => CritType::Confirmable,
        };
        let mut output = DiscordOutputWrapper::new();
        roll_check(
            &attrs_with_levels,
            check_adapter.get_name(),
            character.get_name(),
            facilitation,
            check_type,
            crit_type,
            &mut output,
        );
        send_component_interaction_reply(
            ctx,
            &component_interaction,
            vec![CreateComponent::TextDisplay(CreateTextDisplay::new(
                output.message(),
            ))],
            ephemeral,
        )
        .await?;
        Ok(())
    }

    // TODO: Implement a general `check` function & wrapper commands for all these check types. This should support
    // - Facilitation
    // - Facilitation for specific attributes
    // - Bonus points for the check
    // - Selecting one of the own characters
    // - Selecting a character of a different user in the channel (low-prio)
    async fn generic_check<A>(
        &self,
        ctx: &Context,
        command: &CommandInteraction,
        check_adapter: A,
    ) -> anyhow::Result<()>
    where
        A: CheckAdapter,
    {
        let user_id = command.user.id;

        let characters = self
            .character_manager
            .read()
            .await
            .get_characters(user_id.get())
            .clone();
        let mut selected_character = characters
            .iter()
            .find(|c| c.selected)
            .map(|c| c.character_id);
        let mut selected_facilitation = 0;

        send_command_interaction_reply(
            ctx,
            command,
            self.create_check_menu(
                check_adapter.get_name(),
                &characters,
                selected_character,
                selected_facilitation,
            )
            .await,
        )
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
            let interaction_custom_id = &component_interaction.data.custom_id;
            if interaction_custom_id == ID_SELECT_CHAR {
                selected_character =
                    if let serenity::all::ComponentInteractionDataKind::StringSelect { values } =
                        &component_interaction.data.kind
                    {
                        Some(CharacterId::from(
                            values
                                .first()
                                .context("Expected a selected character in the menu")?
                                .parse::<u64>()?,
                        ))
                    } else {
                        bail!(
                            "Unexpected character select menu interaction type: {:?}",
                            component_interaction.data.kind
                        );
                    };
            } else if interaction_custom_id == ID_SELECT_FACILITATION {
                selected_facilitation =
                    if let serenity::all::ComponentInteractionDataKind::StringSelect { values } =
                        &component_interaction.data.kind
                    {
                        values
                            .first()
                            .context("Expected a selected facilitation option in the menu")?
                            .parse::<i64>()?
                    } else {
                        bail!(
                            "Unexpected facilitation select menu interaction type: {:?}",
                            component_interaction.data.kind
                        );
                    };
            } else if interaction_custom_id == ID_ROLL_CHECK
                || interaction_custom_id == ID_ROLL_CHECK_PRIVATE
            {
                let character = selected_character
                    .context("Tried to make a check without having selected a character")?;
                let ephemeral = interaction_custom_id == ID_ROLL_CHECK_PRIVATE;
                self.roll_check_and_reply(
                    ctx,
                    &component_interaction,
                    &check_adapter,
                    character,
                    selected_facilitation,
                    ephemeral,
                )
                .await?;
                command.delete_response(ctx.http()).await?;
                self.character_manager
                    .write()
                    .await
                    .select_character(user_id.get(), character)
                    .await?;
                return Ok(());
            } else {
                bail!("Unknown interaction ID: {}", interaction_custom_id);
            }
            component_interaction_edit_message(
                ctx,
                &component_interaction,
                self.create_check_menu(
                    check_adapter.get_name(),
                    &characters,
                    selected_character,
                    selected_facilitation,
                )
                .await,
            )
            .await?;
        }
        Ok(())
    }

    // Wraps DSAData::match_search for a command that takes a single `name` argument.
    // If the search doesn't yield a unique result, replies with this error message and returns None.
    async fn search_check_name<'a, V>(
        &self,
        ctx: &Context,
        command: &CommandInteraction,
        entries: impl Iterator<Item = (&'a String, V)>,
    ) -> anyhow::Result<Option<(&'a str, V)>> {
        let name_arg = command
            .data
            .options
            .iter()
            .find(|option| option.name == "name")
            .context("Missing 'name' argument for check")?
            .value
            .as_str()
            .context("Expected 'name' argument for the check to be a string")?;
        match DSAData::match_search(entries, name_arg) {
            MatchSearchResult::Success(m) => Ok(Some(m)),
            MatchSearchResult::NoUniqueMatch(msg) => {
                send_command_interaction_reply(
                    ctx,
                    command,
                    vec![CreateComponent::TextDisplay(CreateTextDisplay::new(msg))],
                )
                .await?;
                Ok(None)
            }
        }
    }

    pub async fn talent(&self, ctx: &Context, command: &CommandInteraction) -> anyhow::Result<()> {
        let (name, talent_info) = match self
            .search_check_name(ctx, command, self.dsa_data.talents.iter())
            .await?
        {
            Some(res) => res,
            None => return Ok(()),
        };
        // self.generic_check(ctx, command, name, &CheckType::PointsCheck(talent_i), attributes)
        struct TalentCheck<'a> {
            name: &'a str,
            attrs: &'a [String],
            attr_short_names: Vec<String>,
        }
        impl<'a> CheckAdapter for TalentCheck<'a> {
            const IS_POINTS_CHECKS: bool = true;
            fn get_name(&self) -> &str {
                self.name
            }
            fn get_attribute_short_names(&self) -> &[String] {
                &self.attr_short_names
            }
            fn get_attr_levels(&self, character: &Character) -> Vec<i64> {
                self.attrs
                    .iter()
                    .map(|attr| character.get_attribute_level(attr))
                    .collect()
            }
            fn get_points_level(&self, character: &Character) -> i64 {
                character.get_skill_level(self.name)
            }
        }
        let adapter = TalentCheck {
            name,
            attrs: &talent_info.attributes,
            attr_short_names: talent_info
                .attributes
                .iter()
                .map(|attr| {
                    self.dsa_data
                        .attributes
                        .get(attr)
                        .with_context(|| format!("Unknown attribute '{}'", attr))
                        .map(|attr| attr.short_name.clone())
                })
                .collect::<anyhow::Result<Vec<String>>>()?,
        };
        self.generic_check(ctx, command, adapter).await
    }

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

trait CheckAdapter {
    const IS_POINTS_CHECKS: bool;
    fn get_name(&self) -> &str;

    fn get_attribute_short_names(&self) -> &[String];

    fn get_attr_levels(&self, character: &Character) -> Vec<i64>;
    // Must only be called if IS_POINTS_CHECK is true.
    fn get_points_level(&self, character: &Character) -> i64;
    // TODO: Add interface for talent specializations
}
