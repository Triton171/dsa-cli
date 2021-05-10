mod character;
mod cli;
mod config;
mod discord;
mod discord_commands;
mod dsa;
mod util;

#[macro_use]
extern crate enum_display_derive;

use character::Character;
use config::{AbstractConfig, Config, DSAData};
use tokio::runtime::Builder;
use util::OutputWrapper;

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

    match matches.subcommand() {
        Some(("discord", _)) => {
            let dsa_data = match DSAData::get_or_create(&mut output) {
                Ok(d) => d,
                Err(e) => {
                    output.output_line(&format!("Unable to get dsa data: {}", e));
                    return;
                }
            };
            let dsa_data = dsa_data.check_replacement_needed(&config, &mut output);
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

        Some(("check", sub_m)) => {
            if let Some((character, dsa_data)) =
                try_get_character_and_dsa_data(&config, &mut output).await
            {
                dsa::talent_check(sub_m, &character, &dsa_data, &config, &mut output);
            } else {
                return;
            }
        }

        Some(("attack", sub_m)) => {
            if let Some((character, dsa_data)) =
                try_get_character_and_dsa_data(&config, &mut output).await
            {
                dsa::attack_check(sub_m, &character, &dsa_data, &mut output)
            } else {
                return;
            }
        }

        Some(("spell", sub_m)) => {
            if let Some((character, dsa_data)) =
                try_get_character_and_dsa_data(&config, &mut output).await
            {
                dsa::spell_check(sub_m, &character, &dsa_data, &config, &mut output)
            } else {
                return;
            }
        }

        Some(("dodge", sub_m)) => {
            let character = match Character::loaded_character().await {
                Ok(Some(c)) => c,
                Ok(None) => {
                    output.output_line(&"Error: No character loaded");
                    return;
                }
                Err(e) => {
                    output.output_line(&format!("Error retrieving loaded character: {}", e));
                    return;
                }
            };
            dsa::dodge_check(sub_m, &character, &mut output);
        }

        Some(("parry", sub_m)) => {
            if let Some((character, dsa_data)) =
                try_get_character_and_dsa_data(&config, &mut output).await
            {
                dsa::parry_check(sub_m, &character, &dsa_data, &mut output);
            } else {
                return;
            }
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
    let dsa_data = match DSAData::get_or_create(output) {
        Ok(d) => d,
        Err(e) => {
            output.output_line(&format!("Unable to read DSA data: {}", e));
            return None;
        }
    };
    let dsa_data = dsa_data.check_replacement_needed(&config, output);
    Some((character, dsa_data))
}
