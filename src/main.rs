mod character;
mod cli;
mod config;
mod dsa;
mod util;

use character::Character;
use config::Config;
use util::OutputWrapper;

fn main() {
    let output = util::CLIOutputWrapper {};

    let mut config = match Config::get_or_create(&output) {
        Ok(c) => c,
        Err(e) => {
            output.output_line(format!(
                "Error while trying to retrieve config: {}",
                e.message()
            ));
            return;
        }
    };

    let app = cli::get_app(&config);
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
                        output
                            .output_line(String::from("Successfully unloaded invalid character"));
                    }
                    Err(e) => {
                        output.output_line(format!("Error saving new config: {}", e.message()));
                    }
                }
            }
        },

        Some(("gen-completions", _)) => {
            cli::generate_completions(&config, &output);
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
            dsa::skillcheck(sub_m, &character, &config, &output);

            /*let (skill_name, facilitation): (&str, i64) = match sub_m.subcommand() {
                Some((s, sub_sub_m)) => match sub_sub_m.value_of("facilitation").unwrap().parse() {
                    Ok(f) => (s, f),
                    Err(_) => {
                        printer.output_line(format!("Error: facilitation must be an integer"));
                        return;
                    }
                },
                _ => {
                    printer.output_line(String::from("Error: skill name missing"));
                    return;
                }
            };
            dsa::skillcheck(skill_name, facilitation, &character, &config, &printer);*/
        }
        _ => {}
    };
}
