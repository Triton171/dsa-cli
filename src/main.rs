mod character;
mod character_manager;
mod cli;
mod config;
mod discord;
mod discord_commands;
mod dsa;
mod util;

#[macro_use]
extern crate enum_display_derive;

use character::Character;
use clap::ArgMatches;
use config::{AbstractConfig, Config, DSAData};
use tokio::runtime::Builder;
use util::{Error, OutputWrapper};

use crate::util::ErrorType;
// Github Actions Test
fn main() {
    let mut output = util::CLIOutputWrapper {};

    let config = match Config::get_or_create(&mut output) {
        Ok(c) => c,
        Err(e) => {
            output.output_line(&format!(
                "Error while trying to retrieve config: {}",
                e.message()
            ));
            return;
        }
    };

    let app = cli::get_app();
    let matches = app.get_matches();
    println!("Started dsa-cli {}", cli::get_version());

    match matches.subcommand() {
        Some(("discord", _)) => {
            let dsa_data = match get_dsa_data(&config, &mut output) {
                Ok(data) => data,
                Err(e) => {
                    output.output_line(&format!("Error retrieving dsa data: {}", e));
                    return;
                }
            };
            let runtime = Builder::new_multi_thread()
                .worker_threads(config.discord.num_threads)
                .enable_io()
                .enable_time()
                .build()
                .unwrap();
            runtime.block_on(discord::start_bot(config, dsa_data));
        }

        _ => {
            let runtime = Builder::new_current_thread().enable_io().build().unwrap();
            runtime.block_on(parse_local_command(matches, config, output));
        }
    };
}

/*
This function parses and executes the local command defined by matches.
Note that the 'discord' command is handled separately by other functions,
as it may require a different async runtime configuration
*/
async fn parse_local_command(
    matches: clap::ArgMatches,
    config: Config,
    mut output: impl OutputWrapper,
) {
    match matches.subcommand() {
        Some(("load", sub_m)) => {
            let character = match Character::load(sub_m.value_of("character_path").unwrap()).await {
                Ok(c) => c,
                Err(e) => {
                    output.output_line(&format!("Error loading character: {}", e.message()));
                    return;
                }
            };
            output.output_line(&format!(
                "Successfully loaded character \"{}\"",
                character.get_name()
            ));
        }

        Some(("unload", _)) => match Character::loaded_character().await {
            Ok(None) => {
                output.output_line(&"There is no character currently loaded");
            }
            _ => match Character::unload().await {
                Ok(()) => {
                    output.output_line(&"Successfully unloaded character");
                }
                Err(e) => {
                    output.output_line(&format!("Error unloading character: {}", e.message()));
                }
            },
        },

        Some(("attribute", sub_m)) => {
            execute_character_command(&dsa::attribute_check, &sub_m, &config, &mut output).await;
        }

        Some(("check", sub_m)) => {
            execute_character_command(&dsa::talent_check, &sub_m, &config, &mut output).await;
        }

        Some(("attack", sub_m)) => {
            execute_character_command(&dsa::attack_check, &sub_m, &config, &mut output).await;
        }

        Some(("spell", sub_m)) => {
            execute_character_command(&dsa::spell_check, &sub_m, &config, &mut output).await;
        }

        Some(("chant", sub_m)) => {
            execute_character_command(&dsa::chant_check, &sub_m, &config, &mut output).await;
        }

        Some(("dodge", sub_m)) => {
            execute_character_command(&dsa::dodge_check, &sub_m, &config, &mut output).await;
        }

        Some(("parry", sub_m)) => {
            execute_character_command(&dsa::parry_check, &sub_m, &config, &mut output).await;
        }

        Some(("roll", sub_m)) => {
            dsa::roll(sub_m, &mut output);
        }

        Some(("ini", _)) => {
            let character = match Character::loaded_character().await {
                Ok(Some(c)) => c,
                Ok(None) => {
                    output.output_line(&"Error: No character loaded");
                    return;
                }
                Err(e) => {
                    output.output_line(&format!(
                        "Error retrieving loaded character: {}",
                        e.message()
                    ));
                    return;
                }
            };
            dsa::roll_ini(
                &[(
                    character.get_name().to_string(),
                    character.get_initiative_level(),
                )],
                &mut output,
            );
        }

        _ => {
            output.output_line(&"Unknown or missing subcommand. Use -h to get help");
        }
    };
}

fn get_dsa_data(config: &Config, output: &mut impl OutputWrapper) -> Result<DSAData, Error> {
    let dsa_data = match DSAData::get_or_create(output) {
        Ok(d) => d,
        Err(e) => {
            if config.auto_update_dsa_data && matches!(e.err_type(), ErrorType::InvalidInput(_)) {
                output.output_line(&format!(
                    "Found invalid dsa data, replacing it with a newer version ({})",
                    e
                ));
                DSAData::create_default()?;
                return DSAData::read();
            } else {
                return Err(e);
            }
        }
    };
    let dsa_data = dsa_data.check_replacement_needed(&config, output);
    Ok(dsa_data)
}

async fn execute_character_command<O>(
    check_fn: impl Fn(&ArgMatches, &Character, &DSAData, &Config, &mut O),
    matches: &ArgMatches,
    config: &Config,
    output: &mut O,
) where
    O: OutputWrapper,
{
    if let Some((character, dsa_data)) = try_get_character_and_dsa_data(config, output).await {
        check_fn(matches, &character, &dsa_data, config, output);
    } else {
        return;
    }
}

async fn try_get_character_and_dsa_data(
    config: &Config,
    output: &mut impl OutputWrapper,
) -> Option<(Character, DSAData)> {
    let character = match Character::loaded_character().await {
        Ok(Some(c)) => c,
        Ok(None) => {
            output.output_line(&"Error: No character loaded");
            return None;
        }
        Err(e) => {
            output.output_line(&format!(
                "Error retrieving loaded character: {}",
                e.message()
            ));
            return None;
        }
    };
    let dsa_data = match get_dsa_data(&config, output) {
        Ok(data) => data,
        Err(e) => {
            output.output_line(&format!("Error retrieving dsa data: {}", e));
            return None;
        }
    };
    Some((character, dsa_data))
}
