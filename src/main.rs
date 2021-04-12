mod character;
mod cli;
mod config;
mod discord;
mod dsa;
mod util;

use character::Character;
use config::Config;
use util::OutputWrapper;

fn main() {
    let mut output = util::CLIOutputWrapper {};

    let mut config = match Config::get_or_create(&mut output) {
        Ok(c) => c,
        Err(e) => {
            output.output_line(format!(
                "Error while trying to retrieve config: {}",
                e.message()
            ));
            return;
        }
    };

    let app = cli::get_app();
    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("load", sub_m)) => {
            let character =
                match Character::load(sub_m.value_of("character_path").unwrap(), &mut config) {
                    Ok(c) => c,
                    Err(e) => {
                        output.output_line(format!("Error loading character: {}", e.message()));
                        return;
                    }
                };
            match config.save() {
                Ok(()) => {
                    output.output_line(format!(
                        "Successfully loaded character \"{}\"",
                        character.get_name()
                    ));
                }
                Err(e) => {
                    output.output_line(format!("Error saving new config: {}", e.message()));
                }
            }
        }

        Some(("unload", _)) => match Character::loaded_character(&config) {
            Ok(None) => {
                output.output_line(String::from("There is no character currently loaded"));
            }
            Ok(Some(c)) => {
                Character::unload(&mut config);
                match config.save() {
                    Ok(()) => {
                        output.output_line(format!(
                            "Successfully unloaded character \"{}\"",
                            c.get_name()
                        ));
                    }
                    Err(e) => {
                        output.output_line(format!("Error saving new config: {}", e.message()));
                    }
                }
            }
            Err(_) => {
                Character::unload(&mut config);
                match config.save() {
                    Ok(()) => {
                        output.output_line(String::from("Successfully unloaded invalid character"));
                    }
                    Err(e) => {
                        output.output_line(format!("Error saving new config: {}", e.message()));
                    }
                }
            }
        },

        Some(("discord", _)) => {
            discord::start_bot(config);
        }

        Some(("gen-completions", _)) => {
            cli::generate_completions(&mut output);
        }

        Some(("skillcheck", sub_m)) => {
            let character = match Character::loaded_character(&config) {
                Ok(Some(c)) => c,
                Ok(None) => {
                    output.output_line(String::from("Error: No character loaded"));
                    return;
                }
                Err(e) => {
                    output.output_line(format!(
                        "Error retrieving loaded character: {}",
                        e.message()
                    ));
                    return;
                }
            };
            dsa::skill_check(sub_m, &character, &config, &mut output);
        }

        Some(("attack", sub_m)) => {
            let character = match Character::loaded_character(&config) {
                Ok(Some(c)) => c,
                Ok(None) => {
                    output.output_line(String::from("Error: No character loaded"));
                    return;
                }
                Err(e) => {
                    output.output_line(format!(
                        "Error retrieving loaded character: {}",
                        e.message()
                    ));
                    return;
                }
            };
            dsa::attack_check(sub_m, &character, &mut output)
        }

        Some(("dodge", sub_m)) => {
            let character = match Character::loaded_character(&config) {
                Ok(Some(c)) => c,
                Ok(None) => {
                    output.output_line(String::from("Error: No character loaded"));
                    return;
                }
                Err(e) => {
                    output.output_line(format!(
                        "Error retrieving loaded character: {}",
                        e.message()
                    ));
                    return;
                }
            };
            dsa::dodge_check(sub_m, &character, &mut output);
        }

        Some(("roll", sub_m)) => {
            dsa::roll(sub_m, &mut output);
        }

        Some(("ini", _)) => {
            let character = match Character::loaded_character(&config) {
                Ok(Some(c)) => c,
                Ok(None) => {
                    output.output_line(String::from("Error: No character loaded"));
                    return;
                }
                Err(e) => {
                    output.output_line(format!(
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
            output.output_line(String::from(
                "Unknown or missing subcommand. Use -h to get help",
            ));
        }
    };
}
